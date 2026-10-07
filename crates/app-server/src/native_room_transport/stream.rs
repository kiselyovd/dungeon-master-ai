use super::{
    failed, invalid, store_failed, AppState, NativeRoomOperation, NativeRoomTransportRequest,
    OUTBOX_LANE,
};
use adapter_http::services::HttpServiceError;
use app_application::rooms::RoomSecret;
use futures::{stream::BoxStream, StreamExt};
use serde_json::Value;
use std::{
    collections::HashSet,
    sync::{LazyLock, Mutex},
    time::Duration,
};
use tokio::sync::mpsc;
static ACTIVE: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(|| Mutex::new(HashSet::new()));
struct Active(String);
impl Active {
    fn acquire(slot: &str) -> Result<Self, HttpServiceError> {
        if !ACTIVE
            .lock()
            .map_err(|_| store_failed())?
            .insert(slot.into())
        {
            return Err(HttpServiceError::Conflict {
                code: "native_room_stream_busy",
            });
        }
        Ok(Self(slot.into()))
    }
}
impl Drop for Active {
    fn drop(&mut self) {
        if let Ok(mut active) = ACTIVE.lock() {
            active.remove(&self.0);
        }
    }
}
fn semantic(operation: &NativeRoomOperation) -> Result<Value, HttpServiceError> {
    let mut value = serde_json::to_value(operation).map_err(|_| invalid())?;
    if matches!(operation, NativeRoomOperation::DmTurn { .. }) {
        value["resume"] = Value::Bool(false);
    }
    Ok(value)
}
async fn conflict_code(response: reqwest::Response) -> &'static str {
    let read = async {
        let mut chunks = response.bytes_stream();
        let mut bytes = Vec::new();
        while let Some(chunk) = chunks.next().await {
            let chunk = chunk.ok()?;
            if bytes.len() + chunk.len() > 4096 {
                return None;
            }
            bytes.extend_from_slice(&chunk);
        }
        let value: Value = serde_json::from_slice(&bytes).ok()?;
        match value.pointer("/error/code")?.as_str()? {
            "agent_turn_superseded" | "agent_request_mismatch" => {
                Some("native_room_turn_superseded")
            }
            "agent_turn_running" => Some("native_room_turn_running"),
            "agent_turn_interrupted" => Some("native_room_turn_interrupted"),
            _ => None,
        }
    };
    tokio::time::timeout(Duration::from_secs(3), read)
        .await
        .ok()
        .flatten()
        .unwrap_or("native_room_state_conflict")
}
pub(super) async fn open(
    state: AppState,
    request: NativeRoomTransportRequest,
) -> Result<BoxStream<'static, Result<Vec<u8>, HttpServiceError>>, HttpServiceError> {
    if request.room_id.is_nil()
        || request.member_id.is_nil()
        || request.address.port() == 0
        || request.address.ip().is_unspecified()
        || request.address.ip().is_multicast()
        || RoomSecret::from_input(request.capability.clone()).is_err()
    {
        return Err(invalid());
    }
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| failed())?;
    let base = format!("http://{}/rooms/{}", request.address, request.room_id);
    let slot = format!(
        "native:room-outbox:{}:{}",
        request.room_id, request.member_id
    );
    let secrets = state.secrets_repo();
    let (builder, active, dm) = match &request.operation {
        NativeRoomOperation::Subscribe { after } => {
            let mut builder = client.get(format!("{base}/events"));
            if let Some(after) = after {
                builder = builder.header("last-event-id", after.to_string());
            }
            (builder, None, false)
        }
        NativeRoomOperation::DmTurn {
            request_id,
            player_message,
            resume,
        } => {
            if request_id.is_nil()
                || player_message.trim().is_empty()
                || player_message.len() > 16384
            {
                return Err(invalid());
            }
            let _lane = OUTBOX_LANE.lock().await;
            let active = Active::acquire(&slot)?;
            if let Some(raw) = secrets.get(&slot).await.map_err(|_| store_failed())? {
                let pending: NativeRoomTransportRequest =
                    serde_json::from_str(&raw).map_err(|_| store_failed())?;
                if pending.room_id != request.room_id
                    || pending.member_id != request.member_id
                    || semantic(&pending.operation)? != semantic(&request.operation)?
                {
                    return Err(HttpServiceError::Conflict {
                        code: "native_room_action_pending",
                    });
                }
            } else {
                secrets
                    .set(
                        &slot,
                        &serde_json::to_string(&request).map_err(|_| invalid())?,
                    )
                    .await
                    .map_err(|_| store_failed())?;
            }
            (client.post(format!("{base}/dm/turn")).json(&serde_json::json!({"request_id":request_id,"player_message":player_message,"resume":resume})), Some(active), true)
        }
        _ => return Err(invalid()),
    };
    let response = tokio::time::timeout(
        Duration::from_secs(30),
        builder
            .bearer_auth(&request.capability)
            .header("x-room-member-id", request.member_id.to_string())
            .send(),
    )
    .await
    .map_err(|_| failed())?
    .map_err(|_| failed())?;
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let conflict = if status == 409 {
            conflict_code(response).await
        } else {
            "native_room_state_conflict"
        };
        if dm
            && (((400..500).contains(&status) && !matches!(status, 408 | 409 | 429))
                || conflict == "native_room_turn_superseded")
        {
            let _lane = OUTBOX_LANE.lock().await;
            secrets.delete(&slot).await.map_err(|_| store_failed())?;
        }
        return Err(match status {
            401 | 403 => HttpServiceError::Unauthorized {
                code: "native_room_access_denied",
            },
            409 => HttpServiceError::Conflict { code: conflict },
            429 => HttpServiceError::RateLimit {
                code: "native_room_rate_limited",
            },
            _ => failed(),
        });
    }
    if !response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|header| header.to_str().ok())
        .is_some_and(|header| {
            header
                .split(';')
                .next()
                .is_some_and(|kind| kind.trim() == "text/event-stream")
        })
    {
        return Err(failed());
    }
    let (tx, rx) = mpsc::channel(8);
    tokio::spawn(async move {
        let _active = active;
        let mut chunks = response.bytes_stream();
        let mut decoder = Decoder::default();
        let deadline = tokio::time::sleep(Duration::from_secs(15 * 60));
        tokio::pin!(deadline);
        loop {
            let next = tokio::select! {
                _ = tx.closed() => return,
                _ = &mut deadline, if dm => { let _ = tx.send(Err(failed())).await; return; },
                next = tokio::time::timeout(Duration::from_secs(60), chunks.next()) => next,
            };
            let eof = matches!(next, Ok(None));
            let frames = match next {
                Ok(Some(Ok(chunk))) => decoder.push(&chunk),
                Ok(None) => decoder.finish(),
                _ => {
                    let _ = tx.send(Err(failed())).await;
                    return;
                }
            };
            let frames = match frames {
                Ok(frames) => frames,
                Err(error) => {
                    let _ = tx.send(Err(error)).await;
                    return;
                }
            };
            for frame in frames {
                if dm && frame.terminal {
                    let _lane = OUTBOX_LANE.lock().await;
                    if secrets.delete(&slot).await.is_err() {
                        let _ = tx.send(Err(store_failed())).await;
                        return;
                    }
                }
                for part in frame.bytes.chunks(32 * 1024) {
                    if tx.send(Ok(part.to_vec())).await.is_err() {
                        return;
                    }
                }
                if dm && frame.terminal {
                    return;
                }
            }
            if eof {
                let _ = tx.send(Err(failed())).await;
                return;
            }
        }
    });
    Ok(tokio_stream::wrappers::ReceiverStream::new(rx).boxed())
}
struct Frame {
    bytes: Vec<u8>,
    terminal: bool,
}
#[derive(Default)]
struct Decoder {
    buffer: Vec<u8>,
    scan: usize,
}
impl Decoder {
    fn push(&mut self, chunk: &[u8]) -> Result<Vec<Frame>, HttpServiceError> {
        if self.buffer.len() + chunk.len() > 16 * 1024 * 1024 {
            return Err(failed());
        }
        self.buffer.extend_from_slice(chunk);
        let mut frames = Vec::new();
        while self.scan + 1 < self.buffer.len() {
            let tail = &self.buffer[self.scan..];
            let separator = if tail.starts_with(b"\r\n\r\n") {
                4
            } else if tail.starts_with(b"\n\n") || tail.starts_with(b"\r\r") {
                2
            } else {
                0
            };
            if separator > 0 {
                if let Some(frame) = parse(&self.buffer[..self.scan])? {
                    frames.push(frame);
                }
                self.buffer.drain(..self.scan + separator);
                self.scan = 0;
            } else {
                self.scan += 1;
            }
        }
        self.scan = self.scan.saturating_sub(3);
        Ok(frames)
    }
    fn finish(&mut self) -> Result<Vec<Frame>, HttpServiceError> {
        let frame = parse(&self.buffer)?;
        self.buffer.clear();
        Ok(frame.into_iter().collect())
    }
}
fn parse(bytes: &[u8]) -> Result<Option<Frame>, HttpServiceError> {
    let raw = std::str::from_utf8(bytes).map_err(|_| failed())?;
    let mut event = "";
    let mut id = "";
    let mut data = Vec::new();
    for line in raw.split(['\r', '\n']) {
        if line.starts_with(':') {
            continue;
        }
        if let Some((field, value)) = line.split_once(':') {
            let value = value.strip_prefix(' ').unwrap_or(value);
            match field {
                "event" => event = value,
                "id" => id = value,
                "data" => data.push(value),
                _ => {}
            }
        }
    }
    // Defense in depth: a player stream never forwards private DM reasoning.
    if event == "reasoning_text" || data.is_empty() {
        return Ok(None);
    }
    if !matches!(
        event,
        "room_update"
            | "text_delta"
            | "tool_call_start"
            | "tool_call_result"
            | "agent_done"
            | "done"
            | "error"
            | "image_generated"
            | "video_generated"
    ) {
        return Err(failed());
    }
    let payload: Value = serde_json::from_str(&data.join("\n")).map_err(|_| failed())?;
    if event == "agent_done"
        && payload
            .get("total_rounds")
            .and_then(Value::as_u64)
            .is_none()
    {
        return Err(failed());
    }
    if !id.is_empty() && (id.len() > 20 || !id.bytes().all(|byte| byte.is_ascii_digit())) {
        return Err(failed());
    }
    let mut wire = format!("event: {event}\n");
    if !id.is_empty() {
        wire.push_str(&format!("id: {id}\n"));
    }
    wire.push_str(&format!(
        "data: {}\n\n",
        serde_json::to_string(&payload).map_err(|_| failed())?
    ));
    Ok(Some(Frame {
        bytes: wire.into_bytes(),
        terminal: event == "agent_done",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn split_unicode_crlf_and_private_reasoning_are_normalized() {
        let raw="event: reasoning_text\r\ndata: {\"text\":\"private\"}\r\n\r\nevent: text_delta\r\ndata: {\"text\":\"Река\"}\r\n\r\nevent: agent_done\ndata: {\"total_rounds\":1}\n\n";
        let mut decoder = Decoder::default();
        let mut frames = Vec::new();
        for byte in raw.as_bytes() {
            frames.extend(decoder.push(&[*byte]).unwrap());
        }
        assert_eq!(frames.len(), 2);
        assert!(frames[1].terminal);
        let raw = String::from_utf8(frames[0].bytes.clone()).unwrap();
        assert!(raw.contains("Река"));
        assert!(!raw.contains("private"));
    }
    #[test]
    fn event_identity_and_malformed_frames_are_rejected() {
        assert!(parse(b"event: room_update\nid: fake\ndata: {}\n").is_err());
        assert!(parse(b"event: text_delta\ndata: broken\n").is_err());
        assert!(parse(b"event: unknown\ndata: {}\n").is_err());
    }
}
