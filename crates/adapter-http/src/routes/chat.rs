use std::convert::Infallible;

use app_application::models::chat::{ChatChunk, ChatMessage, MessagePart, ToolCall, ToolResult};
use axum::extract::Extension;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::Json;
use base64::Engine;
use futures::{Stream, StreamExt};
use serde::{Deserialize, Deserializer};

use crate::services::{ChatHttpCommand, HttpServiceError, HttpServices};

const MAX_IMAGES_PER_MESSAGE: usize = 4;
const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;

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
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let value = serde_json::Value::deserialize(deserializer)?;
        let role = value
            .get("role")
            .and_then(|v| v.as_str())
            .ok_or_else(|| D::Error::custom("missing role"))?;
        match role {
            "system" | "assistant" => {
                let content = value
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| D::Error::custom("content must be a string"))?
                    .to_string();
                if role == "system" {
                    Ok(Self::System { content })
                } else {
                    Ok(Self::Assistant { content })
                }
            }
            "assistant_with_tool_calls" => Ok(Self::AssistantWithToolCalls {
                content: value
                    .get("content")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                tool_calls: serde_json::from_value(
                    value
                        .get("tool_calls")
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!([])),
                )
                .map_err(D::Error::custom)?,
            }),
            "tool_result" => serde_json::from_value(value)
                .map(Self::ToolResult)
                .map_err(D::Error::custom),
            "user" => {
                let parts = if let Some(content) = value.get("content") {
                    vec![MessagePart::Text {
                        text: content
                            .as_str()
                            .ok_or_else(|| D::Error::custom("user.content must be a string"))?
                            .to_string(),
                    }]
                } else {
                    serde_json::from_value(
                        value
                            .get("parts")
                            .cloned()
                            .ok_or_else(|| D::Error::custom("user requires content or parts"))?,
                    )
                    .map_err(D::Error::custom)?
                };
                Ok(Self::User { parts })
            }
            other => Err(D::Error::custom(format!("unknown role: {other}"))),
        }
    }
}

impl From<HttpMessage> for ChatMessage {
    fn from(value: HttpMessage) -> Self {
        match value {
            HttpMessage::System { content } => Self::System { content },
            HttpMessage::User { parts } => Self::User { parts },
            HttpMessage::Assistant { content } => Self::Assistant { content },
            HttpMessage::AssistantWithToolCalls {
                content,
                tool_calls,
            } => Self::AssistantWithToolCalls {
                content,
                tool_calls,
            },
            HttpMessage::ToolResult(result) => Self::ToolResult(result),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ChatHttpRequest {
    pub messages: Vec<HttpMessage>,
    pub session_id: Option<String>,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

fn validate_images(messages: &[ChatMessage]) -> Result<(), HttpServiceError> {
    for message in messages {
        if let ChatMessage::User { parts } = message {
            if parts
                .iter()
                .filter(|part| matches!(part, MessagePart::Image { .. }))
                .count()
                > MAX_IMAGES_PER_MESSAGE
            {
                return Err(HttpServiceError::PayloadTooLarge {
                    code: "too_many_images",
                });
            }
            for part in parts {
                if let MessagePart::Image { data_b64, .. } = part {
                    let bytes = base64::engine::general_purpose::STANDARD
                        .decode(data_b64)
                        .map_err(|_| HttpServiceError::BadRequest {
                            code: "image_base64_invalid",
                        })?;
                    if bytes.len() > MAX_IMAGE_BYTES {
                        return Err(HttpServiceError::PayloadTooLarge {
                            code: "image_too_large",
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

pub async fn chat(
    Extension(services): Extension<HttpServices>,
    Json(request): Json<ChatHttpRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, HttpServiceError> {
    if request.messages.is_empty() {
        return Err(HttpServiceError::BadRequest {
            code: "messages_empty",
        });
    }
    let messages = request
        .messages
        .into_iter()
        .map(Into::into)
        .collect::<Vec<_>>();
    validate_images(&messages)?;
    let chunks = services
        .chat
        .stream(ChatHttpCommand {
            messages,
            session_id: request.session_id,
            model: request.model,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
        })
        .await?;
    let stream = chunks.map(|chunk| {
        let event = match chunk {
            Ok(ChatChunk::TextDelta { text }) => Event::default()
                .event("text_delta")
                .json_data(serde_json::json!({"text": text}))
                .expect("serializable"),
            Ok(ChatChunk::Done { reason }) => Event::default()
                .event("done")
                .json_data(serde_json::json!({"reason": reason}))
                .expect("serializable"),
            Ok(ChatChunk::ThinkingDelta { .. }) => {
                Event::default().comment("thinking_chunk_dropped")
            }
            Ok(
                ChatChunk::ToolCallStart { .. }
                | ChatChunk::ToolCallArgsDelta { .. }
                | ChatChunk::ToolCallDone { .. },
            ) => Event::default().comment("tool_call_chunk_dropped"),
            Err(_) => Event::default()
                .event("error")
                .json_data(
                    serde_json::json!({"code": "provider_error", "message": "provider_error"}),
                )
                .expect("serializable"),
        };
        Ok(event)
    });
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_malformed_roles_and_invalid_image_payloads() {
        let malformed = serde_json::from_value::<ChatHttpRequest>(serde_json::json!({
            "messages": [{"role":"foreign","content":"x"}],
            "session_id": null,
            "model": null,
            "max_tokens": null,
            "temperature": null
        }));
        assert!(malformed.is_err());

        let messages = vec![ChatMessage::User {
            parts: vec![MessagePart::Image {
                mime: "image/png".into(),
                data_b64: "not-base64".into(),
                name: None,
            }],
        }];
        assert_eq!(
            validate_images(&messages),
            Err(HttpServiceError::BadRequest {
                code: "image_base64_invalid"
            })
        );
    }

    #[test]
    fn enforces_image_count_before_service_dispatch() {
        let image = MessagePart::Image {
            mime: "image/png".into(),
            data_b64: "AA==".into(),
            name: None,
        };
        let messages = vec![ChatMessage::User {
            parts: vec![image; MAX_IMAGES_PER_MESSAGE + 1],
        }];
        assert_eq!(
            validate_images(&messages),
            Err(HttpServiceError::PayloadTooLarge {
                code: "too_many_images"
            })
        );
    }
}
