use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::{Stream, StreamExt};
use serde::{Deserialize, Deserializer};
use std::convert::Infallible;
use std::pin::Pin;

use app_llm::{ChatChunk, ChatMessage, ChatRequest as LlmReq, MessagePart, ToolCall, ToolResult};

use crate::error::AppError;
use crate::state::AppState;

const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;
const MAX_IMAGES_PER_MESSAGE: usize = 4;

/// Wire-format chat message used at the HTTP boundary. Accepts either a bare
/// `content` string (legacy ergonomics) or a full `parts` array on the user
/// role, normalising both into `app_llm::ChatMessage::User { parts: ... }`.
#[derive(Debug)]
pub enum HttpMessage {
    System {
        content: String,
    },
    User {
        parts: Vec<MessagePart>,
    },
    Assistant {
        content: String,
    },
    AssistantWithToolCalls {
        content: Option<String>,
        tool_calls: Vec<ToolCall>,
    },
    ToolResult(ToolResult),
}

impl<'de> Deserialize<'de> for HttpMessage {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let v = serde_json::Value::deserialize(de)?;
        let role = v
            .get("role")
            .and_then(|r| r.as_str())
            .ok_or_else(|| D::Error::custom("missing role"))?;
        match role {
            "system" => {
                let content = v
                    .get("content")
                    .and_then(|c| c.as_str())
                    .ok_or_else(|| D::Error::custom("system requires string content"))?
                    .to_string();
                Ok(HttpMessage::System { content })
            }
            "assistant" => {
                let content = v
                    .get("content")
                    .and_then(|c| c.as_str())
                    .ok_or_else(|| D::Error::custom("assistant requires string content"))?
                    .to_string();
                Ok(HttpMessage::Assistant { content })
            }
            "assistant_with_tool_calls" => {
                let content = v
                    .get("content")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string());
                let tool_calls: Vec<ToolCall> = serde_json::from_value(
                    v.get("tool_calls").cloned().unwrap_or_else(|| serde_json::json!([])),
                )
                .map_err(|e| D::Error::custom(format!("invalid tool_calls: {e}")))?;
                Ok(HttpMessage::AssistantWithToolCalls {
                    content,
                    tool_calls,
                })
            }
            "tool_result" => {
                let tr: ToolResult = serde_json::from_value(v)
                    .map_err(|e| D::Error::custom(format!("invalid tool_result: {e}")))?;
                Ok(HttpMessage::ToolResult(tr))
            }
            "user" => {
                let parts = if let Some(c) = v.get("content") {
                    if let Some(s) = c.as_str() {
                        vec![MessagePart::Text {
                            text: s.to_string(),
                        }]
                    } else {
                        return Err(D::Error::custom(
                            "user.content must be a string; use `parts` for arrays",
                        ));
                    }
                } else if let Some(p) = v.get("parts") {
                    serde_json::from_value(p.clone())
                        .map_err(|e| D::Error::custom(format!("invalid parts: {e}")))?
                } else {
                    return Err(D::Error::custom("user requires `content` or `parts`"));
                };
                Ok(HttpMessage::User { parts })
            }
            other => Err(D::Error::custom(format!("unknown role: {other}"))),
        }
    }
}

impl From<HttpMessage> for ChatMessage {
    fn from(m: HttpMessage) -> Self {
        match m {
            HttpMessage::System { content } => ChatMessage::System { content },
            HttpMessage::User { parts } => ChatMessage::User { parts },
            HttpMessage::Assistant { content } => ChatMessage::Assistant { content },
            HttpMessage::AssistantWithToolCalls {
                content,
                tool_calls,
            } => ChatMessage::AssistantWithToolCalls {
                content,
                tool_calls,
            },
            HttpMessage::ToolResult(tr) => ChatMessage::ToolResult(tr),
        }
    }
}

/// Validate per-image size and per-message image count. Decodes base64 fully
/// because length-only heuristics are off by up to two bytes per padding rules
/// and we want exact byte counts to match Anthropic / OpenAI limits.
pub fn enforce_size_guards(messages: &[ChatMessage]) -> Result<(), AppError> {
    use base64::Engine;
    for m in messages {
        if let ChatMessage::User { parts } = m {
            let image_count = parts
                .iter()
                .filter(|p| matches!(p, MessagePart::Image { .. }))
                .count();
            if image_count > MAX_IMAGES_PER_MESSAGE {
                return Err(AppError::PayloadTooLarge(format!(
                    "at most {MAX_IMAGES_PER_MESSAGE} image parts per message (got {image_count})"
                )));
            }
            for p in parts {
                if let MessagePart::Image { data_b64, .. } = p {
                    let raw = base64::engine::general_purpose::STANDARD
                        .decode(data_b64.as_bytes())
                        .map_err(|e| AppError::BadRequest(format!("invalid base64 image: {e}")))?;
                    if raw.len() > MAX_IMAGE_BYTES {
                        return Err(AppError::PayloadTooLarge(format!(
                            "image exceeds 5 MB (got {} bytes)",
                            raw.len()
                        )));
                    }
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct ChatHttpRequest {
    pub messages: Vec<HttpMessage>,
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

    let llm_messages: Vec<ChatMessage> = req.messages.into_iter().map(Into::into).collect();
    enforce_size_guards(&llm_messages)?;

    let llm_req = LlmReq {
        messages: llm_messages,
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
