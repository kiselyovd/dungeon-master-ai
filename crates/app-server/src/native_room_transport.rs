//! Native-only room transport and durable pending mutation recovery.
use crate::AppState;
use adapter_http::services::{HttpServiceError, NativeRoomTransportHttpService};
use app_application::{models::room_transport::*, rooms::RoomSecret};
use async_trait::async_trait;
use futures::StreamExt;
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

fn invalid() -> HttpServiceError {
    HttpServiceError::BadRequest {
        code: "native_room_operation_invalid",
    }
}
fn failed() -> HttpServiceError {
    HttpServiceError::BadGateway {
        code: "native_room_transport_failed",
    }
}
fn store_failed() -> HttpServiceError {
    HttpServiceError::Internal {
        code: "native_room_outbox_failed",
    }
}
pub fn service(
    state: AppState,
) -> Result<Arc<dyn NativeRoomTransportHttpService>, HttpServiceError> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| failed())?;
    Ok(Arc::new(Transport {
        state,
        client,
        lane: Mutex::new(()),
    }))
}
struct Transport {
    state: AppState,
    client: reqwest::Client,
    lane: Mutex<()>,
}
struct WireOperation {
    path: String,
    body: Option<Value>,
}
fn wire(operation: &NativeRoomOperation) -> Result<WireOperation, HttpServiceError> {
    use NativeRoomOperation::*;
    let (path, body) = match operation {
        PendingAction {} => return Err(invalid()),
        State {} => ("state".into(), None),
        Updates { after } => (format!("updates?after={after}"), None),
        Party {} => ("party".into(), None),
        OwnProposal {} => ("character-proposals/own".into(), None),
        PendingProposals {} => ("character-proposals".into(), None),
        RestStatus {} => ("rest".into(), None),
        SubmitCharacter {
            proposal_id,
            character_id,
            character,
        } => {
            if proposal_id.is_nil() || character_id.is_nil() {
                return Err(invalid());
            }
            (
                "character-proposals".into(),
                Some(
                    json!({"proposal_id":proposal_id,"character_id":character_id,"character":character}),
                ),
            )
        }
        ApproveCharacter {
            proposal_id,
            expected_revision,
            causation_id,
        } => {
            if proposal_id.is_nil() || !valid_nonce(causation_id) {
                return Err(invalid());
            }
            (
                format!("character-proposals/{proposal_id}/approve"),
                Some(json!({"expected_revision":expected_revision,"causation_id":causation_id})),
            )
        }
        Adventure {
            request_id,
            expected_revision,
            action,
        } => {
            if !valid_nonce(request_id) {
                return Err(invalid());
            }
            (
                "adventure/actions".into(),
                Some(
                    json!({"request_id":request_id,"expected_revision":expected_revision,"action":action}),
                ),
            )
        }
        Rest {
            request_id,
            expected_revision,
            intent,
        } => {
            if !valid_nonce(request_id) {
                return Err(invalid());
            }
            (
                "rest".into(),
                Some(
                    json!({"request_id":request_id,"expected_revision":expected_revision,"intent":intent}),
                ),
            )
        }
        Combat {
            encounter_id,
            request_id,
            expected_revision,
            action,
        } => {
            if encounter_id.is_nil() || !valid_nonce(request_id) {
                return Err(invalid());
            }
            (
                format!("combat/{encounter_id}/actions"),
                Some(
                    json!({"request_id":request_id,"expected_revision":expected_revision,"action":action}),
                ),
            )
        }
    };
    Ok(WireOperation { path, body })
}
fn valid_nonce(nonce: &str) -> bool {
    !nonce.trim().is_empty() && nonce.len() <= 128
}
#[async_trait]
impl NativeRoomTransportHttpService for Transport {
    async fn execute(
        &self,
        request: NativeRoomTransportRequest,
    ) -> Result<NativeRoomTransportResult, HttpServiceError> {
        if request.room_id.is_nil()
            || request.member_id.is_nil()
            || request.address.port() == 0
            || request.address.ip().is_unspecified()
            || request.address.ip().is_multicast()
            || RoomSecret::from_input(request.capability.clone()).is_err()
        {
            return Err(invalid());
        }
        if matches!(request.operation, NativeRoomOperation::PendingAction {}) {
            let _lane = self.lane.lock().await;
            let slot = format!(
                "native:room-outbox:{}:{}",
                request.room_id, request.member_id
            );
            let pending = self
                .state
                .secrets_repo()
                .get(&slot)
                .await
                .map_err(|_| store_failed())?;
            let body = pending
                .map(|raw| {
                    let saved: NativeRoomTransportRequest =
                        serde_json::from_str(&raw).map_err(|_| store_failed())?;
                    serde_json::to_value(saved.operation).map_err(|_| store_failed())
                })
                .transpose()?;
            return Ok(NativeRoomTransportResult { status: 200, body });
        }
        let operation = wire(&request.operation)?;
        let serialized = serde_json::to_string(&request).map_err(|_| invalid())?;
        if serialized.len() > 9 * 1024 * 1024 {
            return Err(invalid());
        }
        // One pending mutation per participant. Reads can still refresh state while
        // an unknown mutation remains pending, but a new mutation cannot replace it.
        let _lane = self.lane.lock().await;
        let secrets = self.state.secrets_repo();
        let slot = format!(
            "native:room-outbox:{}:{}",
            request.room_id, request.member_id
        );
        if operation.body.is_some() {
            if let Some(pending) = secrets.get(&slot).await.map_err(|_| store_failed())? {
                let pending: NativeRoomTransportRequest =
                    serde_json::from_str(&pending).map_err(|_| store_failed())?;
                let same_operation = serde_json::to_value(&pending.operation)
                    .map_err(|_| store_failed())?
                    == serde_json::to_value(&request.operation).map_err(|_| invalid())?;
                // Renewed credentials or a rebound host address are transport data;
                // the participant and exact canonical mutation remain immutable.
                if pending.room_id != request.room_id
                    || pending.member_id != request.member_id
                    || !same_operation
                {
                    return Err(HttpServiceError::Conflict {
                        code: "native_room_action_pending",
                    });
                }
            } else {
                secrets
                    .set(&slot, &serialized)
                    .await
                    .map_err(|_| store_failed())?;
            }
        }
        let mutation = operation.body.is_some();
        let url = format!(
            "http://{}/rooms/{}/{}",
            request.address, request.room_id, operation.path
        );
        let builder = match operation.body {
            Some(body) => self.client.post(url).json(&body),
            None => self.client.get(url),
        };
        let response = builder
            .bearer_auth(&request.capability)
            .header("x-room-member-id", request.member_id.to_string())
            .send()
            .await
            .map_err(|_| failed())?;
        let status = response.status();
        let definitive_rejection =
            status.is_client_error() && !matches!(status.as_u16(), 408 | 429);
        if mutation && definitive_rejection {
            // A confirmed rejection releases the slot even if its body is plain
            // text, oversized, malformed or interrupted during download.
            secrets.delete(&slot).await.map_err(|_| store_failed())?;
        }
        // Never follow a redirect or expose its response as a successful operation.
        if status.is_redirection() {
            return Err(failed());
        }
        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| failed())?;
            if bytes.len() + chunk.len() > 16 * 1024 * 1024 {
                return Err(failed());
            }
            bytes.extend_from_slice(&chunk);
        }
        let body = if bytes.is_empty() {
            None
        } else {
            Some(match serde_json::from_slice(&bytes) {
                Ok(body) => body,
                Err(_) if status.is_client_error() => json!({"code":"native_room_remote_rejected"}),
                Err(_) => return Err(failed()),
            })
        };
        // 5xx, timeout and rate limiting are uncertain; retain the exact mutation.
        // Confirmed acceptance or a definitive rejection can release the slot.
        if mutation && status.is_success() {
            secrets.delete(&slot).await.map_err(|_| store_failed())?;
        }
        Ok(NativeRoomTransportResult {
            status: status.as_u16(),
            body,
        })
    }
}
