use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::{Stream, StreamExt};
use serde::Deserialize;
use std::convert::Infallible;
use std::pin::Pin;

use app_llm::{ChatChunk, ChatMessage, ChatRequest as LlmReq};

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ChatHttpRequest {
    pub messages: Vec<ChatMessage>,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

pub async fn chat(
    State(state): State<AppState>,
    Json(req): Json<ChatHttpRequest>,
) -> Result<impl IntoResponse, AppError> {
    if req.messages.is_empty() {
        return Err(AppError::BadRequest("messages must not be empty".into()));
    }

    let llm_req = LlmReq {
        messages: req.messages,
        model: req.model.unwrap_or_else(|| state.default_model()),
        max_tokens: req.max_tokens,
        temperature: req.temperature,
        tools: Vec::new(),
        system_prompt: None,
    };

    let provider = state.provider();
    let chunk_stream = provider.stream_chat(llm_req).await?;

    let event_stream: Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>> =
        Box::pin(chunk_stream.map(|chunk| {
            let event = match chunk {
                Ok(ChatChunk::TextDelta { text }) => Event::default()
                    .event("text_delta")
                    .json_data(serde_json::json!({ "text": text }))
                    .expect("json_data"),
                Ok(ChatChunk::Done { reason }) => Event::default()
                    .event("done")
                    .json_data(serde_json::json!({ "reason": reason }))
                    .expect("json_data"),
                Ok(ChatChunk::ToolCallStart { .. })
                | Ok(ChatChunk::ToolCallArgsDelta { .. })
                | Ok(ChatChunk::ToolCallDone { .. }) => {
                    // Legacy /chat endpoint passes empty `tools`, so providers
                    // should not produce these. If they do, drop silently;
                    // the agent endpoint (M3 Phase I) handles tool-call chunks.
                    Event::default().comment("tool_call_chunk_dropped")
                }
                Err(e) => Event::default()
                    .event("error")
                    .json_data(serde_json::json!({
                        "code": "provider_error",
                        "message": e.to_string()
                    }))
                    .expect("json_data"),
            };
            Ok(event)
        }));

    Ok(Sse::new(event_stream).keep_alive(KeepAlive::default()))
}
