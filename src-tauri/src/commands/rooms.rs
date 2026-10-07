use super::backend::BackendState;
use crate::processes::control::ControlServer;
use app_application::models::{
    room::PublicRoom,
    room_host::{NativeHostRoomGrant, NativeHostRoomRequest},
};
use std::{net::SocketAddr, sync::Mutex, time::Duration};
use tauri::{AppHandle, Manager};
use uuid::Uuid;
#[derive(Default)]
pub struct NativeRoomState {
    host: Mutex<Option<NativeHostRoomGrant>>,
    guest: Mutex<Option<app_application::models::room_client::NativeGuestConnection>>,
    lifecycle: tokio::sync::Mutex<()>,
}
#[derive(serde::Serialize)]
pub struct NativeHostRoomView {
    room: PublicRoom,
    member_id: Uuid,
    invitation: String,
    address: SocketAddr,
}
fn public_view(grant: &NativeHostRoomGrant) -> NativeHostRoomView {
    NativeHostRoomView {
        room: grant.room.clone(),
        member_id: grant.member_id,
        invitation: grant.invitation.clone(),
        address: grant.address,
    }
}
fn owner_connection(app: &AppHandle) -> Result<(u16, String), String> {
    let backend = app.state::<BackendState>();
    let port = (*backend
        .port
        .lock()
        .map_err(|_| "native_backend_unavailable")?)
    .ok_or("native_backend_unavailable")?;
    let token = app.state::<ControlServer>().token().to_owned();
    Ok((port, token))
}
fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| "native_http_unavailable".into())
}
impl NativeRoomState {
    async fn apply_host<F>(&self, operation: F) -> Result<NativeHostRoomView, String>
    where
        F: std::future::Future<Output = Result<NativeHostRoomGrant, String>>,
    {
        let _lifecycle = self.lifecycle.lock().await;
        let grant = operation.await?;
        let view = public_view(&grant);
        *self
            .host
            .lock()
            .map_err(|_| "native_room_state_unavailable")? = Some(grant);
        Ok(view)
    }
    async fn apply_stop<F>(&self, operation: F) -> Result<(), String>
    where
        F: std::future::Future<Output = Result<(), String>>,
    {
        let _lifecycle = self.lifecycle.lock().await;
        operation.await?;
        *self
            .host
            .lock()
            .map_err(|_| "native_room_state_unavailable")? = None;
        Ok(())
    }
}
#[tauri::command]
pub async fn host_room(
    app: AppHandle,
    request: NativeHostRoomRequest,
) -> Result<NativeHostRoomView, String> {
    let state = app.state::<NativeRoomState>();
    state
        .apply_host(async {
            let (port, token) = owner_connection(&app)?;
            let response = client()?
                .post(format!("http://127.0.0.1:{port}/native/rooms/host"))
                .bearer_auth(token)
                .json(&request)
                .send()
                .await
                .map_err(|_| "native_host_request_failed")?;
            if !response.status().is_success() {
                return Err("native_host_request_rejected".into());
            }
            let grant = response
                .json::<NativeHostRoomGrant>()
                .await
                .map_err(|_| "native_host_response_invalid")?;
            if grant.room.campaign_id != request.campaign_id
                || grant.room.session_id != request.session_id
                || !grant.room.active
                || !grant.room.members.iter().any(|member| {
                    member.member_id == grant.member_id
                        && member.active
                        && member.role == app_application::models::party::PartyRole::Host
                })
                || app_application::rooms::RoomSecret::from_input(grant.capability.clone()).is_err()
            {
                return Err("native_host_response_invalid".into());
            }
            Ok(grant)
        })
        .await
}
#[tauri::command]
pub async fn stop_host_room(app: AppHandle, room_id: Uuid) -> Result<(), String> {
    let state = app.state::<NativeRoomState>();
    state
        .apply_stop(async {
            let (port, token) = owner_connection(&app)?;
            let response = client()?
                .delete(format!("http://127.0.0.1:{port}/native/rooms/{room_id}"))
                .bearer_auth(token)
                .send()
                .await
                .map_err(|_| "native_host_request_failed")?;
            if !response.status().is_success() {
                return Err("native_host_stop_rejected".into());
            }
            Ok(())
        })
        .await
}

#[derive(serde::Serialize)]
pub struct NativeGuestRoomView {
    room: PublicRoom,
    member_id: Uuid,
    join_id: Uuid,
    address: SocketAddr,
}
fn guest_view(
    connection: &app_application::models::room_client::NativeGuestConnection,
) -> NativeGuestRoomView {
    NativeGuestRoomView {
        room: connection.room.clone(),
        member_id: connection.member_id,
        join_id: connection.join_id,
        address: connection.address,
    }
}
#[tauri::command]
pub async fn join_room(
    app: AppHandle,
    request: app_application::models::room_client::NativeGuestJoinRequest,
) -> Result<NativeGuestRoomView, String> {
    let state = app.state::<NativeRoomState>();
    let _lifecycle = state.lifecycle.lock().await;
    let (port, token) = owner_connection(&app)?;
    let response = client()?
        .post(format!("http://127.0.0.1:{port}/native/room-client/join"))
        .bearer_auth(token)
        .json(&request)
        .send()
        .await
        .map_err(|_| "native_guest_request_failed")?;
    if !response.status().is_success() {
        return Err("native_guest_request_rejected".into());
    }
    let connection = response
        .json::<app_application::models::room_client::NativeGuestConnection>()
        .await
        .map_err(|_| "native_guest_response_invalid")?;
    if connection.room.room_id != request.room_id
        || connection.join_id != request.join_id
        || connection.address != request.address
        || app_application::rooms::RoomSecret::from_input(connection.capability.clone()).is_err()
    {
        return Err("native_guest_response_invalid".into());
    }
    let view = guest_view(&connection);
    *state
        .guest
        .lock()
        .map_err(|_| "native_room_state_unavailable")? = Some(connection);
    Ok(view)
}
#[tauri::command]
pub async fn resume_room(
    app: AppHandle,
    request: app_application::models::room_client::NativeGuestResumeRequest,
) -> Result<NativeGuestRoomView, String> {
    let state = app.state::<NativeRoomState>();
    let _lifecycle = state.lifecycle.lock().await;
    let (port, token) = owner_connection(&app)?;
    let response = client()?
        .post(format!("http://127.0.0.1:{port}/native/room-client/resume"))
        .bearer_auth(token)
        .json(&request)
        .send()
        .await
        .map_err(|_| "native_guest_request_failed")?;
    if !response.status().is_success() {
        return Err("native_guest_request_rejected".into());
    }
    let connection = response
        .json::<app_application::models::room_client::NativeGuestConnection>()
        .await
        .map_err(|_| "native_guest_response_invalid")?;
    if connection.room.room_id != request.room_id
        || connection.join_id != request.join_id
        || app_application::rooms::RoomSecret::from_input(connection.capability.clone()).is_err()
    {
        return Err("native_guest_response_invalid".into());
    }
    let view = guest_view(&connection);
    *state
        .guest
        .lock()
        .map_err(|_| "native_room_state_unavailable")? = Some(connection);
    Ok(view)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn renderer_view_never_contains_the_host_capability() {
        let capability = app_application::rooms::RoomSecret::generate();
        let invitation = app_application::rooms::RoomSecret::generate();
        let grant = NativeHostRoomGrant {
            room: PublicRoom {
                room_id: Uuid::new_v4(),
                campaign_id: app_application::models::identity::CampaignId(Uuid::new_v4()),
                session_id: app_application::models::identity::SessionId(Uuid::new_v4()),
                active: true,
                members: vec![],
            },
            member_id: Uuid::new_v4(),
            capability: capability.expose().into(),
            invitation: invitation.expose().into(),
            address: "127.0.0.1:4123".parse().unwrap(),
        };
        let view = serde_json::to_value(public_view(&grant)).unwrap();
        assert!(view.get("capability").is_none());
        assert!(!view.to_string().contains(capability.expose()));
        assert_eq!(view["member_id"], serde_json::json!(grant.member_id));
    }
    #[test]
    fn renderer_guest_view_excludes_connection_credentials() {
        let host = grant(Uuid::new_v4());
        let connection = app_application::models::room_client::NativeGuestConnection {
            address: host.address,
            room: host.room,
            member_id: host.member_id,
            capability: host.capability,
            join_id: Uuid::new_v4(),
        };
        let view = serde_json::to_value(guest_view(&connection)).unwrap();
        assert!(view.get("capability").is_none());
        assert!(view.get("resume_proof").is_none());
        assert!(view.get("invitation").is_none());
        assert!(!view.to_string().contains(&connection.capability));
        assert_eq!(view["join_id"], serde_json::json!(connection.join_id));
    }
    fn grant(room: Uuid) -> NativeHostRoomGrant {
        NativeHostRoomGrant {
            room: PublicRoom {
                room_id: room,
                campaign_id: app_application::models::identity::CampaignId(Uuid::new_v4()),
                session_id: app_application::models::identity::SessionId(Uuid::new_v4()),
                active: true,
                members: vec![],
            },
            member_id: Uuid::new_v4(),
            capability: app_application::rooms::RoomSecret::generate()
                .expose()
                .into(),
            invitation: app_application::rooms::RoomSecret::generate()
                .expose()
                .into(),
            address: "127.0.0.1:4141".parse().unwrap(),
        }
    }
    #[tokio::test]
    async fn delayed_lifecycle_responses_commit_in_order() {
        for first_is_stop in [true, false] {
            let state = std::sync::Arc::new(NativeRoomState::default());
            let entered = std::sync::Arc::new(tokio::sync::Notify::new());
            let release = std::sync::Arc::new(tokio::sync::Notify::new());
            let first_room = Uuid::new_v4();
            let second_room = Uuid::new_v4();
            let first = {
                let state = state.clone();
                let entered = entered.clone();
                let release = release.clone();
                tokio::spawn(async move {
                    if first_is_stop {
                        state
                            .apply_stop(async {
                                entered.notify_one();
                                release.notified().await;
                                Ok(())
                            })
                            .await
                            .unwrap();
                    } else {
                        state
                            .apply_host(async {
                                entered.notify_one();
                                release.notified().await;
                                Ok(grant(first_room))
                            })
                            .await
                            .unwrap();
                    }
                })
            };
            entered.notified().await;
            let second = {
                let state = state.clone();
                tokio::spawn(async move {
                    if first_is_stop {
                        state
                            .apply_host(async { Ok(grant(second_room)) })
                            .await
                            .unwrap();
                    } else {
                        state.apply_stop(async { Ok(()) }).await.unwrap();
                    }
                })
            };
            tokio::time::sleep(Duration::from_millis(10)).await;
            assert!(!second.is_finished());
            release.notify_one();
            first.await.unwrap();
            second.await.unwrap();
            let host = state.host.lock().unwrap();
            if first_is_stop {
                assert_eq!(host.as_ref().unwrap().room.room_id, second_room);
            } else {
                assert!(host.is_none());
            }
        }
    }
}
