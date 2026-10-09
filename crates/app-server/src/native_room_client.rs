//! Private native guest connection storage and remote transport. No renderer secrets.
use crate::AppState;
use adapter_http::services::{HttpServiceError, NativeRoomClientHttpService};
use app_application::{
    models::{party::PartyRole, room::PublicRoom, room_client::*},
    rooms::RoomSecret,
};
use futures::StreamExt;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use uuid::Uuid;
fn invalid() -> HttpServiceError {
    HttpServiceError::BadRequest {
        code: "native_guest_request_invalid",
    }
}
fn failed() -> HttpServiceError {
    HttpServiceError::BadGateway {
        code: "native_guest_remote_failed",
    }
}
fn slot(join: Uuid) -> String {
    format!("native:guest-join:{join}")
}
/// Archive only the matching selected guest, preserving credentials and gameplay outbox.
pub(crate) async fn archive_departure(
    state: &AppState,
    room: Uuid,
    member: Uuid,
    capability: &str,
    epoch: u64,
) -> Result<(), HttpServiceError> {
    let _lane = state.native_guest_selection_lane().lock_owned().await;
    let secrets = state.secrets_repo();
    let current = secrets
        .get("native:guest-selection")
        .await
        .map_err(|_| failed())?
        .map(|raw| serde_json::from_str::<NativeGuestResumeRequest>(&raw))
        .transpose()
        .map_err(|_| failed())?;
    let raw = secrets
        .get("native:guest-known-joins")
        .await
        .map_err(|_| failed())?;
    let mut known: Vec<NativeGuestResumeRequest> = raw
        .map(|raw| serde_json::from_str(&raw))
        .transpose()
        .map_err(|_| failed())?
        .unwrap_or_default();
    if let Some(pointer) = &current {
        if !known
            .iter()
            .any(|p| p.room_id == pointer.room_id && p.join_id == pointer.join_id)
        {
            known.push(NativeGuestResumeRequest {
                room_id: pointer.room_id,
                join_id: pointer.join_id,
            });
        }
    }
    let raw = secrets
        .get("native:guest-archived-selections")
        .await
        .map_err(|_| failed())?;
    let mut archive: Vec<NativeGuestResumeRequest> = raw
        .map(|raw| serde_json::from_str(&raw))
        .transpose()
        .map_err(|_| failed())?
        .unwrap_or_default();
    for pointer in known.into_iter().filter(|pointer| pointer.room_id == room) {
        let Some(raw) = secrets
            .get(&slot(pointer.join_id))
            .await
            .map_err(|_| failed())?
        else {
            continue;
        };
        let mut record: StoredNativeGuestJoin = serde_json::from_str(&raw).map_err(|_| failed())?;
        if record.connection.as_ref().is_none_or(|connection| {
            connection.member_id != member
                || connection.capability != capability
                || connection.room.epoch > epoch
        }) {
            continue;
        }
        record.departure_epoch = Some(epoch);
        record.return_intent = None;
        secrets
            .set(
                &slot(pointer.join_id),
                &serde_json::to_string(&record).map_err(|_| failed())?,
            )
            .await
            .map_err(|_| failed())?;
        if !archive
            .iter()
            .any(|old| old.room_id == room && old.join_id == pointer.join_id)
        {
            if archive.len() >= 64 {
                return Err(HttpServiceError::Conflict {
                    code: "native_guest_archive_full",
                });
            }
            archive.push(NativeGuestResumeRequest {
                room_id: room,
                join_id: pointer.join_id,
            });
            secrets
                .set(
                    "native:guest-archived-selections",
                    &serde_json::to_string(&archive).map_err(|_| failed())?,
                )
                .await
                .map_err(|_| failed())?;
        }
        if current
            .as_ref()
            .is_some_and(|selected| selected.room_id == room && selected.join_id == pointer.join_id)
        {
            secrets
                .delete("native:guest-selection")
                .await
                .map_err(|_| failed())?;
        }
    }
    Ok(())
}
pub fn service(state: AppState) -> Result<Arc<dyn NativeRoomClientHttpService>, HttpServiceError> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|_| failed())?;
    Ok(Arc::new(ClientService { state, client }))
}
struct ClientService {
    state: AppState,
    client: reqwest::Client,
}
impl ClientService {
    async fn settle_departure(
        &self,
        record: &mut StoredNativeGuestJoin,
    ) -> Result<(), HttpServiceError> {
        let Some(connection) = &record.connection else {
            return Ok(());
        };
        let key = crate::native_room_transport::departure_slot(
            connection.room.room_id,
            connection.member_id,
        );
        let Some(raw) = self
            .state
            .secrets_repo()
            .get(&key)
            .await
            .map_err(|_| failed())?
        else {
            return Ok(());
        };
        let pending: app_application::models::room_transport::NativeRoomTransportRequest =
            serde_json::from_str(&raw).map_err(|_| failed())?;
        let outcome = crate::native_room_transport::service(self.state.clone())?
            .execute(pending)
            .await?;
        if !(200..300).contains(&outcome.status) {
            return Err(failed());
        }
        let epoch = outcome
            .body
            .as_ref()
            .and_then(|body| body["departure_epoch"].as_u64())
            .filter(|epoch| *epoch > 0)
            .ok_or_else(failed)?;
        let _lane = self.state.native_guest_selection_lane().lock_owned().await;
        *record = self
            .load(record.request.join_id)
            .await?
            .ok_or_else(failed)?;
        if record
            .connection
            .as_ref()
            .is_some_and(|connection| connection.room.epoch <= epoch)
        {
            record.departure_epoch = Some(epoch);
            record.return_intent = None;
            self.save(record).await?;
        }
        Ok(())
    }

    async fn check_pending(
        &self,
        request: &NativeGuestJoinRequest,
    ) -> Result<(), HttpServiceError> {
        if let Some(raw) = self
            .state
            .secrets_repo()
            .get("native:guest-pending-selection")
            .await
            .map_err(|_| HttpServiceError::Internal {
                code: "native_guest_store_failed",
            })?
        {
            let pointer: NativeGuestResumeRequest =
                serde_json::from_str(&raw).map_err(|_| HttpServiceError::Internal {
                    code: "native_guest_store_invalid",
                })?;
            if pointer.room_id != request.room_id || pointer.join_id != request.join_id {
                return Err(HttpServiceError::Conflict {
                    code: "native_guest_selection_pending",
                });
            }
        }
        Ok(())
    }
    async fn select(
        &self,
        request: &NativeGuestJoinRequest,
        pending: bool,
    ) -> Result<(), HttpServiceError> {
        if pending {
            self.check_pending(request).await?;
        }
        let pointer = serde_json::to_string(&NativeGuestResumeRequest {
            room_id: request.room_id,
            join_id: request.join_id,
        })
        .map_err(|_| invalid())?;
        self.state
            .secrets_repo()
            .set(
                if pending {
                    "native:guest-pending-selection"
                } else {
                    "native:guest-selection"
                },
                &pointer,
            )
            .await
            .map_err(|_| HttpServiceError::Internal {
                code: "native_guest_store_failed",
            })
    }
    async fn load(&self, join: Uuid) -> Result<Option<StoredNativeGuestJoin>, HttpServiceError> {
        self.state
            .secrets_repo()
            .get(&slot(join))
            .await
            .map_err(|_| HttpServiceError::Internal {
                code: "native_guest_store_failed",
            })?
            .map(|raw| {
                serde_json::from_str(&raw).map_err(|_| HttpServiceError::Internal {
                    code: "native_guest_store_invalid",
                })
            })
            .transpose()
    }
    async fn save(&self, record: &StoredNativeGuestJoin) -> Result<(), HttpServiceError> {
        let raw = serde_json::to_string(record).map_err(|_| HttpServiceError::Internal {
            code: "native_guest_store_invalid",
        })?;
        let secrets = self.state.secrets_repo();
        secrets
            .set(&slot(record.request.join_id), &raw)
            .await
            .map_err(|_| HttpServiceError::Internal {
                code: "native_guest_store_failed",
            })?;
        let known = secrets
            .get("native:guest-known-joins")
            .await
            .map_err(|_| failed())?;
        let mut known: Vec<NativeGuestResumeRequest> = known
            .map(|raw| serde_json::from_str(&raw))
            .transpose()
            .map_err(|_| failed())?
            .unwrap_or_default();
        if !known.iter().any(|pointer| {
            pointer.room_id == record.request.room_id && pointer.join_id == record.request.join_id
        }) {
            known.push(NativeGuestResumeRequest {
                room_id: record.request.room_id,
                join_id: record.request.join_id,
            });
            secrets
                .set(
                    "native:guest-known-joins",
                    &serde_json::to_string(&known).map_err(|_| failed())?,
                )
                .await
                .map_err(|_| failed())?;
        }
        Ok(())
    }
    async fn complete_selected(
        &self,
        record: &mut StoredNativeGuestJoin,
    ) -> Result<NativeGuestConnection, HttpServiceError> {
        match self.complete(record).await {
            Ok(connection) => {
                self.select(&record.request, false).await?;
                self.state
                    .secrets_repo()
                    .delete("native:guest-pending-selection")
                    .await
                    .map_err(|_| HttpServiceError::Internal {
                        code: "native_guest_store_failed",
                    })?;
                Ok(connection)
            }
            Err(error) => {
                if matches!(
                    error,
                    HttpServiceError::Conflict {
                        code: "native_guest_access_rejected"
                    }
                ) {
                    self.state
                        .secrets_repo()
                        .delete("native:guest-pending-selection")
                        .await
                        .map_err(|_| HttpServiceError::Internal {
                            code: "native_guest_store_failed",
                        })?;
                }
                Err(error)
            }
        }
    }
    async fn decode<T: DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T, HttpServiceError> {
        if !response.status().is_success() {
            return Err(
                if response.status().is_client_error()
                    && !matches!(response.status().as_u16(), 408 | 429)
                {
                    HttpServiceError::Conflict {
                        code: "native_guest_access_rejected",
                    }
                } else {
                    failed()
                },
            );
        }
        let mut chunks = response.bytes_stream();
        let mut bytes = Vec::new();
        while let Some(chunk) = chunks.next().await {
            let chunk = chunk.map_err(|_| failed())?;
            if bytes.len() + chunk.len() > 64 * 1024 {
                return Err(failed());
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|_| failed())
    }
    fn valid_connection(
        request: &NativeGuestJoinRequest,
        room: &PublicRoom,
        member: Uuid,
        capability: &str,
    ) -> bool {
        room.room_id == request.room_id
            && room.active
            && !room.campaign_id.0.is_nil()
            && !room.session_id.0.is_nil()
            && !member.is_nil()
            && RoomSecret::from_input(capability.into()).is_ok()
            && room.members.iter().any(|entry| {
                entry.member_id == member && entry.active && entry.role == PartyRole::Player
            })
    }
    async fn refresh(
        &self,
        record: &mut StoredNativeGuestJoin,
    ) -> Result<NativeGuestConnection, HttpServiceError> {
        let connection = record.connection.as_ref().ok_or_else(invalid)?;
        let response = self
            .client
            .get(format!(
                "http://{}/rooms/{}",
                record.request.address, record.request.room_id
            ))
            .bearer_auth(&connection.capability)
            .header("x-room-member-id", connection.member_id.to_string())
            .send()
            .await
            .map_err(|_| failed())?;
        let room: PublicRoom = self.decode(response).await?;
        if !Self::valid_connection(
            &record.request,
            &room,
            connection.member_id,
            &connection.capability,
        ) {
            return Err(failed());
        }
        let connection = NativeGuestConnection {
            address: record.request.address,
            room,
            member_id: connection.member_id,
            capability: connection.capability.clone(),
            join_id: record.request.join_id,
        };
        record.connection = Some(connection.clone());
        self.save(record).await?;
        Ok(connection)
    }
    async fn complete(
        &self,
        record: &mut StoredNativeGuestJoin,
    ) -> Result<NativeGuestConnection, HttpServiceError> {
        if record.connection.is_some()
            && record.return_intent.is_none()
            && record.departure_epoch.is_none()
        {
            return self.refresh(record).await;
        }
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Grant {
            room: PublicRoom,
            member_id: Uuid,
            capability: String,
        }
        let response=self.client.post(format!("http://{}/rooms/{}/join",record.request.address,record.request.room_id)).json(&serde_json::json!({"join_id":record.request.join_id,"display_name":record.request.display_name,"invitation":record.request.invitation,"resume_proof":record.resume_proof,"return_intent":record.return_intent})).send().await.map_err(|_|failed())?;
        let grant: Grant = self.decode(response).await?;
        if !Self::valid_connection(
            &record.request,
            &grant.room,
            grant.member_id,
            &grant.capability,
        ) {
            return Err(failed());
        }
        let connection = NativeGuestConnection {
            address: record.request.address,
            room: grant.room,
            member_id: grant.member_id,
            capability: grant.capability,
            join_id: record.request.join_id,
        };
        record.connection = Some(connection.clone());
        record.departure_epoch = None;
        record.return_intent = None;
        self.save(record).await?;
        Ok(connection)
    }
}
#[async_trait::async_trait]
impl NativeRoomClientHttpService for ClientService {
    async fn saved(&self) -> Result<Vec<NativeGuestRecoveryChoice>, HttpServiceError> {
        let _lane = self.state.native_guest_selection_lane().lock_owned().await;
        let raw = self
            .state
            .secrets_repo()
            .get("native:guest-archived-selections")
            .await
            .map_err(|_| HttpServiceError::Internal {
                code: "native_guest_store_failed",
            })?;
        let pointers: Vec<NativeGuestResumeRequest> = raw
            .map(|raw| {
                serde_json::from_str(&raw).map_err(|_| HttpServiceError::Internal {
                    code: "native_guest_store_invalid",
                })
            })
            .transpose()?
            .unwrap_or_default();
        let mut choices = Vec::new();
        for pointer in pointers {
            let record = self
                .load(pointer.join_id)
                .await?
                .ok_or(HttpServiceError::NotFound)?;
            if record.request.room_id != pointer.room_id {
                return Err(invalid());
            }
            choices.push(NativeGuestRecoveryChoice {
                room_id: pointer.room_id,
                join_id: pointer.join_id,
                address: record.request.address,
                display_name: record.request.display_name,
            });
        }
        Ok(choices)
    }
    async fn postpone(&self) -> Result<(), HttpServiceError> {
        let _lane = self.state.native_guest_selection_lane().lock_owned().await;
        let secrets = self.state.secrets_repo();
        let Some(raw) = secrets
            .get("native:guest-pending-selection")
            .await
            .map_err(|_| HttpServiceError::Internal {
                code: "native_guest_store_failed",
            })?
        else {
            return Ok(());
        };
        let pointer: NativeGuestResumeRequest =
            serde_json::from_str(&raw).map_err(|_| HttpServiceError::Internal {
                code: "native_guest_store_invalid",
            })?;
        let archive = secrets
            .get("native:guest-archived-selections")
            .await
            .map_err(|_| HttpServiceError::Internal {
                code: "native_guest_store_failed",
            })?;
        let mut pointers: Vec<NativeGuestResumeRequest> = archive
            .map(|raw| {
                serde_json::from_str(&raw).map_err(|_| HttpServiceError::Internal {
                    code: "native_guest_store_invalid",
                })
            })
            .transpose()?
            .unwrap_or_default();
        if !pointers
            .iter()
            .any(|saved| saved.room_id == pointer.room_id && saved.join_id == pointer.join_id)
        {
            if pointers.len() >= 64 {
                return Err(HttpServiceError::Conflict {
                    code: "native_guest_archive_full",
                });
            }
            pointers.push(pointer);
            secrets
                .set(
                    "native:guest-archived-selections",
                    &serde_json::to_string(&pointers).map_err(|_| invalid())?,
                )
                .await
                .map_err(|_| HttpServiceError::Internal {
                    code: "native_guest_store_failed",
                })?;
        }
        secrets
            .delete("native:guest-pending-selection")
            .await
            .map_err(|_| HttpServiceError::Internal {
                code: "native_guest_store_failed",
            })
    }

    async fn recover(&self) -> Result<Option<NativeGuestConnection>, HttpServiceError> {
        if let Some(raw) = self
            .state
            .secrets_repo()
            .get("native:guest-selection")
            .await
            .map_err(|_| failed())?
        {
            let pointer: NativeGuestResumeRequest =
                serde_json::from_str(&raw).map_err(|_| failed())?;
            if let Some(mut record) = self.load(pointer.join_id).await? {
                self.settle_departure(&mut record).await?;
            }
        }
        let _lane = self.state.native_guest_selection_lane().lock_owned().await;
        let pending = self
            .state
            .secrets_repo()
            .get("native:guest-pending-selection")
            .await
            .map_err(|_| HttpServiceError::Internal {
                code: "native_guest_store_failed",
            })?;
        let confirmed = if pending.is_none() {
            self.state
                .secrets_repo()
                .get("native:guest-selection")
                .await
                .map_err(|_| HttpServiceError::Internal {
                    code: "native_guest_store_failed",
                })?
        } else {
            None
        };
        let Some(raw) = pending.or(confirmed) else {
            return Ok(None);
        };
        let pointer: NativeGuestResumeRequest =
            serde_json::from_str(&raw).map_err(|_| HttpServiceError::Internal {
                code: "native_guest_store_invalid",
            })?;
        if pointer.room_id.is_nil() || pointer.join_id.is_nil() {
            return Err(invalid());
        }
        let mut record = self
            .load(pointer.join_id)
            .await?
            .ok_or(HttpServiceError::NotFound)?;
        if record.request.room_id != pointer.room_id {
            return Err(invalid());
        }
        if record.departure_epoch.is_some() {
            return Ok(None);
        }
        self.complete_selected(&mut record).await.map(Some)
    }

    async fn join(
        &self,
        request: NativeGuestJoinRequest,
    ) -> Result<NativeGuestConnection, HttpServiceError> {
        let _lane = self.state.native_guest_selection_lane().lock_owned().await;
        if request.room_id.is_nil()
            || request.join_id.is_nil()
            || request.address.port() == 0
            || request.address.ip().is_unspecified()
            || request.address.ip().is_multicast()
            || !app_application::models::room::valid_display_name(&request.display_name)
            || RoomSecret::from_input(request.invitation.clone()).is_err()
        {
            return Err(invalid());
        }
        self.check_pending(&request).await?;
        let mut record = if let Some(record) = self.load(request.join_id).await? {
            if record.request != request {
                return Err(HttpServiceError::Conflict {
                    code: "native_guest_request_mismatch",
                });
            }
            record
        } else {
            let record = StoredNativeGuestJoin {
                departure_epoch: None,
                return_intent: None,
                request,
                resume_proof: RoomSecret::generate().expose().into(),
                connection: None,
            };
            self.save(&record).await?;
            record
        };
        self.select(&record.request, true).await?;
        self.complete_selected(&mut record).await
    }
    async fn resume(
        &self,
        request: NativeGuestResumeRequest,
    ) -> Result<NativeGuestConnection, HttpServiceError> {
        if request.room_id.is_nil() || request.join_id.is_nil() {
            return Err(invalid());
        }
        let mut record = self
            .load(request.join_id)
            .await?
            .ok_or(HttpServiceError::NotFound)?;
        if record.request.room_id != request.room_id {
            return Err(HttpServiceError::Conflict {
                code: "native_guest_request_mismatch",
            });
        }
        self.settle_departure(&mut record).await?;
        let _lane = self.state.native_guest_selection_lane().lock_owned().await;
        let mut record = self
            .load(request.join_id)
            .await?
            .ok_or(HttpServiceError::NotFound)?;
        if let Some(epoch) = record.departure_epoch {
            if record.return_intent.is_none() {
                record.return_intent = Some(app_application::models::room::RoomReturnIntent {
                    request_id: Uuid::new_v4().to_string(),
                    expected_departure_epoch: epoch,
                });
                self.save(&record).await?;
            }
        }
        self.select(&record.request, true).await?;
        self.complete_selected(&mut record).await
    }
}
