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
    generation: std::sync::atomic::AtomicU64,
    streams: Mutex<std::collections::HashMap<Uuid, tokio::sync::watch::Sender<bool>>>,
    cancelled_streams: Mutex<std::collections::VecDeque<Uuid>>,
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
async fn host_error(mut response: reqwest::Response) -> String {
    let mut bytes = Vec::new();
    while let Ok(Some(chunk)) = response.chunk().await {
        if bytes.len() + chunk.len() > 4096 {
            return "native_host_request_rejected".into();
        }
        bytes.extend_from_slice(&chunk);
    }
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();
    match value
        .pointer("/error/code")
        .and_then(serde_json::Value::as_str)
    {
        Some("native_room_bind_failed") => "native_room_bind_failed",
        Some("native_room_create_failed") => "native_room_create_failed",
        Some("native_host_request_superseded") => "native_host_request_superseded",
        _ => "native_host_request_rejected",
    }
    .into()
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
        self.generation
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
        self.generation
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
                return Err(host_error(response).await);
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
pub async fn recover_host_room(app: AppHandle) -> Result<Option<NativeHostRoomView>, String> {
    let state = app.state::<NativeRoomState>();
    let _lifecycle = state.lifecycle.lock().await;
    let (port, token) = owner_connection(&app)?;
    let response = client()?
        .post(format!("http://127.0.0.1:{port}/native/rooms/recover"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|_| "native_host_request_failed")?;
    if !response.status().is_success() {
        return Err(host_error(response).await);
    }
    let grant = response
        .json::<Option<NativeHostRoomGrant>>()
        .await
        .map_err(|_| "native_host_response_invalid")?;
    let Some(grant) = grant else {
        return Ok(None);
    };
    if grant.room.room_id.is_nil()
        || !grant.room.active
        || grant.address.port() == 0
        || !grant.room.members.iter().any(|member| {
            member.member_id == grant.member_id
                && member.active
                && member.role == app_application::models::party::PartyRole::Host
        })
        || app_application::rooms::RoomSecret::from_input(grant.capability.clone()).is_err()
    {
        return Err("native_host_response_invalid".into());
    }
    let view = public_view(&grant);
    *state
        .host
        .lock()
        .map_err(|_| "native_room_state_unavailable")? = Some(grant);
    state
        .generation
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Ok(Some(view))
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
    state
        .generation
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
    state
        .generation
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Ok(view)
}

#[tauri::command]
pub async fn saved_guest_rooms(
    app: AppHandle,
) -> Result<Vec<app_application::models::room_client::NativeGuestRecoveryChoice>, String> {
    let (port, token) = owner_connection(&app)?;
    let response = client()?
        .get(format!("http://127.0.0.1:{port}/native/room-client/saved"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|_| "native_guest_request_failed")?;
    if !response.status().is_success() {
        return Err("native_guest_request_rejected".into());
    }
    response
        .json()
        .await
        .map_err(|_| "native_guest_response_invalid".into())
}
#[tauri::command]
pub async fn postpone_guest_room(app: AppHandle) -> Result<(), String> {
    let state = app.state::<NativeRoomState>();
    let _lifecycle = state.lifecycle.lock().await;
    let (port, token) = owner_connection(&app)?;
    let response = client()?
        .post(format!(
            "http://127.0.0.1:{port}/native/room-client/postpone"
        ))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|_| "native_guest_request_failed")?;
    if !response.status().is_success() {
        return Err("native_guest_request_rejected".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn recover_guest_room(app: AppHandle) -> Result<Option<NativeGuestRoomView>, String> {
    let state = app.state::<NativeRoomState>();
    let _lifecycle = state.lifecycle.lock().await;
    let (port, token) = owner_connection(&app)?;
    let response = client()?
        .post(format!(
            "http://127.0.0.1:{port}/native/room-client/recover"
        ))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|_| "native_guest_request_failed")?;
    if !response.status().is_success() {
        return Err("native_guest_request_rejected".into());
    }
    let connection = response
        .json::<Option<app_application::models::room_client::NativeGuestConnection>>()
        .await
        .map_err(|_| "native_guest_response_invalid")?;
    let Some(connection) = connection else {
        return Ok(None);
    };
    if !connection.room.active
        || connection.room.room_id.is_nil()
        || connection.join_id.is_nil()
        || !connection.room.members.iter().any(|member| {
            member.member_id == connection.member_id
                && member.active
                && member.role == app_application::models::party::PartyRole::Player
        })
        || app_application::rooms::RoomSecret::from_input(connection.capability.clone()).is_err()
    {
        return Err("native_guest_response_invalid".into());
    }
    let view = guest_view(&connection);
    *state
        .guest
        .lock()
        .map_err(|_| "native_room_state_unavailable")? = Some(connection);
    state
        .generation
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Ok(Some(view))
}

impl NativeRoomState {
    fn transport_request(
        &self,
        room_id: Uuid,
        operation: app_application::models::room_transport::NativeRoomOperation,
    ) -> Result<app_application::models::room_transport::NativeRoomTransportRequest, String> {
        use app_application::models::room_transport::NativeRoomTransportRequest;
        if let Some(host) = self
            .host
            .lock()
            .map_err(|_| "native_room_state_unavailable")?
            .as_ref()
            .filter(|host| host.room.room_id == room_id)
        {
            let mut address = host.address;
            if address.ip().is_unspecified() {
                address.set_ip(if address.is_ipv4() {
                    std::net::Ipv4Addr::LOCALHOST.into()
                } else {
                    std::net::Ipv6Addr::LOCALHOST.into()
                });
            }
            return Ok(NativeRoomTransportRequest {
                address,
                room_id,
                member_id: host.member_id,
                capability: host.capability.clone(),
                operation,
            });
        }
        let guest = self
            .guest
            .lock()
            .map_err(|_| "native_room_state_unavailable")?;
        let guest = guest
            .as_ref()
            .filter(|guest| guest.room.room_id == room_id)
            .ok_or("native_room_not_connected")?;
        Ok(NativeRoomTransportRequest {
            address: guest.address,
            room_id,
            member_id: guest.member_id,
            capability: guest.capability.clone(),
            operation,
        })
    }
}
#[tauri::command]
pub async fn room_operation(
    app: AppHandle,
    room_id: Uuid,
    operation: app_application::models::room_transport::NativeRoomOperation,
) -> Result<app_application::models::room_transport::NativeRoomTransportResult, String> {
    use std::sync::atomic::Ordering;
    let state = app.state::<NativeRoomState>();
    let (request, generation) = {
        let _lifecycle = state.lifecycle.lock().await;
        (
            state.transport_request(room_id, operation)?,
            state.generation.load(Ordering::SeqCst),
        )
    };
    let (port, token) = owner_connection(&app)?;
    let mut response = client()?
        .post(format!(
            "http://127.0.0.1:{port}/native/room-client/execute"
        ))
        .bearer_auth(token)
        .json(&request)
        .send()
        .await
        .map_err(|_| "native_room_request_failed")?;
    let owner_status = response.status();
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "native_room_response_invalid")?
    {
        if bytes.len() + chunk.len() > 17 * 1024 * 1024 {
            return Err("native_room_response_invalid".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let result = if owner_status.is_success() {
        serde_json::from_slice(&bytes).map_err(|_| "native_room_response_invalid")?
    } else {
        app_application::models::room_transport::NativeRoomTransportResult {
            status: owner_status.as_u16(),
            body: Some(serde_json::from_slice(&bytes).map_err(|_| "native_room_response_invalid")?),
        }
    };
    if state.generation.load(Ordering::SeqCst) != generation {
        return Err("native_room_selection_changed".into());
    }
    Ok(result)
}

#[derive(serde::Serialize, Clone)]
pub struct NativeRoomStreamChunk {
    room_id: Uuid,
    stream_id: Uuid,
    bytes: Vec<u8>,
}
struct StreamRegistration<'a> {
    state: &'a NativeRoomState,
    stream_id: Uuid,
}
impl Drop for StreamRegistration<'_> {
    fn drop(&mut self) {
        if let Ok(mut streams) = self.state.streams.lock() {
            streams.remove(&self.stream_id);
        }
    }
}
impl NativeRoomState {
    fn cancel_stream(&self, stream_id: Uuid) -> Result<(), String> {
        let streams = self
            .streams
            .lock()
            .map_err(|_| "native_room_state_unavailable")?;
        if let Some(stream) = streams.get(&stream_id) {
            let _ = stream.send(true);
        } else {
            let mut cancelled = self
                .cancelled_streams
                .lock()
                .map_err(|_| "native_room_state_unavailable")?;
            if cancelled.len() >= 256 {
                cancelled.pop_front();
            }
            cancelled.push_back(stream_id);
        }
        Ok(())
    }
    fn register_stream(
        &self,
        stream_id: Uuid,
    ) -> Result<tokio::sync::watch::Receiver<bool>, String> {
        let mut streams = self
            .streams
            .lock()
            .map_err(|_| "native_room_state_unavailable")?;
        let mut cancelled_ids = self
            .cancelled_streams
            .lock()
            .map_err(|_| "native_room_state_unavailable")?;
        if let Some(index) = cancelled_ids.iter().position(|id| *id == stream_id) {
            cancelled_ids.remove(index);
            return Err("native_room_stream_cancelled".into());
        }
        if streams.contains_key(&stream_id) {
            return Err("native_room_stream_exists".into());
        }
        let (cancel, cancelled) = tokio::sync::watch::channel(false);
        streams.insert(stream_id, cancel);
        Ok(cancelled)
    }
}
#[tauri::command]
pub fn cancel_room_stream(app: AppHandle, stream_id: Uuid) -> Result<(), String> {
    app.state::<NativeRoomState>().cancel_stream(stream_id)
}
#[tauri::command]
pub async fn stream_room(
    app: AppHandle,
    room_id: Uuid,
    stream_id: Uuid,
    operation: app_application::models::room_transport::NativeRoomOperation,
    on_chunk: tauri::ipc::Channel<NativeRoomStreamChunk>,
) -> Result<(), String> {
    use app_application::models::room_transport::NativeRoomOperation;
    use std::sync::atomic::Ordering;
    if stream_id.is_nil()
        || !matches!(
            operation,
            NativeRoomOperation::Subscribe { .. } | NativeRoomOperation::DmTurn { .. }
        )
    {
        return Err("native_room_stream_invalid".into());
    }
    let state = app.state::<NativeRoomState>();
    let (request, generation) = {
        let _lifecycle = state.lifecycle.lock().await;
        (
            state.transport_request(room_id, operation)?,
            state.generation.load(Ordering::SeqCst),
        )
    };
    let mut cancelled = state.register_stream(stream_id)?;
    let _registration = StreamRegistration {
        state: &state,
        stream_id,
    };
    let (port, token) = owner_connection(&app)?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| "native_http_unavailable")?;
    let send = client
        .post(format!("http://127.0.0.1:{port}/native/room-client/stream"))
        .bearer_auth(token)
        .json(&request)
        .send();
    let mut response = tokio::select! {
        _ = cancelled.changed() => return Err("native_room_stream_cancelled".into()),
        response = tokio::time::timeout(Duration::from_secs(30), send) => response
            .map_err(|_| "native_room_stream_failed")?.map_err(|_| "native_room_stream_failed")?,
    };
    if !response.status().is_success() {
        let status = response.status().as_u16();
        if status == 409 {
            let mut bytes = Vec::new();
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| "native_room_stream_failed")?
            {
                if bytes.len() + chunk.len() > 4096 {
                    return Err("native_room_state_conflict".into());
                }
                bytes.extend_from_slice(&chunk);
            }
            let value: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|_| "native_room_state_conflict")?;
            let code = match value
                .pointer("/error/code")
                .and_then(serde_json::Value::as_str)
            {
                Some("native_room_turn_superseded") => "native_room_turn_superseded",
                Some("native_room_turn_running") => "native_room_turn_running",
                Some("native_room_turn_interrupted") => "native_room_turn_interrupted",
                _ => "native_room_state_conflict",
            };
            return Err(code.into());
        }
        return Err(match status {
            401 | 403 => "native_room_access_denied",
            409 => "native_room_state_conflict",
            429 => "native_room_rate_limited",
            _ => "native_room_stream_failed",
        }
        .into());
    }
    let mut check = tokio::time::interval(Duration::from_millis(250));
    loop {
        let chunk = tokio::select! {
            _ = cancelled.changed() => return Err("native_room_stream_cancelled".into()),
            _ = check.tick() => {
                if state.generation.load(Ordering::SeqCst) != generation { return Err("native_room_selection_changed".into()); }
                continue;
            },
            chunk = response.chunk() => chunk.map_err(|_| "native_room_stream_failed")?,
        };
        let Some(chunk) = chunk else {
            return Ok(());
        };
        if state.generation.load(Ordering::SeqCst) != generation {
            return Err("native_room_selection_changed".into());
        }
        for bytes in chunk.chunks(32 * 1024) {
            on_chunk
                .send(NativeRoomStreamChunk {
                    room_id,
                    stream_id,
                    bytes: bytes.to_vec(),
                })
                .map_err(|_| "native_room_stream_consumer_closed")?;
        }
    }
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
    #[test]
    fn native_transport_uses_cached_identity_and_loopback_for_wildcard_host() {
        let state = NativeRoomState::default();
        let mut host = grant(Uuid::new_v4());
        host.address = "0.0.0.0:4141".parse().unwrap();
        let room = host.room.room_id;
        let member = host.member_id;
        *state.host.lock().unwrap() = Some(host);
        let request = state
            .transport_request(
                room,
                app_application::models::room_transport::NativeRoomOperation::State {},
            )
            .unwrap();
        assert_eq!(
            request.address,
            "127.0.0.1:4141".parse::<SocketAddr>().unwrap()
        );
        assert_eq!(request.member_id, member);
        assert!(state
            .transport_request(
                Uuid::new_v4(),
                app_application::models::room_transport::NativeRoomOperation::State {}
            )
            .is_err());
        for value in [
            serde_json::json!({"kind":"state","address":"127.0.0.1:1"}),
            serde_json::json!({"kind":"state","member_id":Uuid::new_v4()}),
            serde_json::json!({"kind":"arbitrary","path":"/settings"}),
        ] {
            assert!(serde_json::from_value::<
                app_application::models::room_transport::NativeRoomOperation,
            >(value)
            .is_err());
        }
    }
    #[test]
    fn stream_abort_before_registration_and_duplicate_open_are_safe() {
        let state = NativeRoomState::default();
        let early = Uuid::new_v4();
        state.cancel_stream(early).unwrap();
        assert_eq!(
            state.register_stream(early).unwrap_err(),
            "native_room_stream_cancelled"
        );
        assert!(state.streams.lock().unwrap().is_empty());
        let active = Uuid::new_v4();
        let receiver = state.register_stream(active).unwrap();
        assert_eq!(
            state.register_stream(active).unwrap_err(),
            "native_room_stream_exists"
        );
        state.cancel_stream(active).unwrap();
        assert!(*receiver.borrow());
        let registration = StreamRegistration {
            state: &state,
            stream_id: active,
        };
        drop(registration);
        assert!(state.streams.lock().unwrap().is_empty());
        for _ in 0..300 {
            state.cancel_stream(Uuid::new_v4()).unwrap();
        }
        assert_eq!(state.cancelled_streams.lock().unwrap().len(), 256);
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
