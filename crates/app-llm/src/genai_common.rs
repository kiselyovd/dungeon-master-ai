//! Shared genai helpers used by both `AnthropicProvider` and
//! `OpenAICompatProvider`. Keeps message conversion and the streaming
//! `ToolCallChunk -> ChatChunk` pump in one place so the two providers do not
//! drift.

use futures::stream::StreamExt;
use genai::chat::{
    ChatMessage as GMsg, ChatRequest as GReq, ChatStream, ChatStreamEvent, Tool as GTool,
    ToolCall as GToolCall, ToolResponse as GToolResponse,
};
use std::collections::HashMap;
use tokio::sync::mpsc::Sender;
use tracing::warn;

use crate::provider::{ChatChunk, ChatMessage, FinishReason, LlmError, MessagePart, Tool};

/// Translate our provider-agnostic `ChatMessage`/`Tool` shape into a genai
/// `ChatRequest`. Identical for every genai-backed provider, so it lives here.
pub(crate) fn convert_messages(messages: Vec<ChatMessage>, tools: Vec<Tool>) -> GReq {
    let mut g_req = GReq::default();
    for tool in tools {
        let g_tool = GTool::new(tool.name)
            .with_description(tool.description)
            .with_schema(tool.parameters);
        g_req = g_req.append_tool(g_tool);
    }
    for m in messages {
        match m {
            ChatMessage::System { content } => g_req = g_req.with_system(content),
            ChatMessage::User { parts } => {
                use genai::chat::{ContentPart, MessageContent};
                let mut content = MessageContent::default();
                for part in parts {
                    match part {
                        MessagePart::Text { text } => {
                            content.push(ContentPart::Text(text));
                        }
                        MessagePart::Image {
                            mime,
                            data_b64,
                            name,
                        } => {
                            content.push(ContentPart::from_binary_base64(mime, data_b64, name));
                        }
                    }
                }
                g_req = g_req.append_message(GMsg::user(content));
            }
            ChatMessage::Assistant { content } => {
                g_req = g_req.append_message(GMsg::assistant(content));
            }
            ChatMessage::AssistantWithToolCalls {
                content,
                tool_calls,
            } => {
                if let Some(text) = content {
                    g_req = g_req.append_message(GMsg::assistant(text));
                }
                let g_calls: Vec<GToolCall> = tool_calls
                    .into_iter()
                    .map(|tc| GToolCall {
                        call_id: tc.id,
                        fn_name: tc.name,
                        fn_arguments: tc.args,
                        thought_signatures: None,
                    })
                    .collect();
                if !g_calls.is_empty() {
                    g_req = g_req.append_message(GMsg::from(g_calls));
                }
            }
            ChatMessage::ToolResult(tr) => {
                // TODO(phase-c): tr.is_error is dropped - genai's ToolResponse has no error variant. Encode error semantics into tr.content instead (e.g. JSON {"error": "..."}).
                let response = GToolResponse::new(tr.tool_call_id, tr.content);
                g_req = g_req.append_message(GMsg::from(response));
            }
        }
    }
    g_req
}

/// Drain a genai chat stream, translating each event into our `ChatChunk`
/// shape and forwarding it on `tx`. Bails out as soon as the receiver is
/// dropped. The genai stream is consumed by value.
pub(crate) async fn pump_genai_stream(
    mut g_stream: ChatStream,
    tx: Sender<Result<ChatChunk, LlmError>>,
) {
    let mut args_seen: HashMap<String, String> = HashMap::new();

    while let Some(event) = g_stream.next().await {
        match event {
            Ok(ChatStreamEvent::Chunk(c)) => {
                if tx
                    .send(Ok(ChatChunk::TextDelta { text: c.content }))
                    .await
                    .is_err()
                {
                    return;
                }
            }
            Ok(ChatStreamEvent::ReasoningChunk(_)) => continue,
            Ok(ChatStreamEvent::ThoughtSignatureChunk(_)) => continue,
            Ok(ChatStreamEvent::ToolCallChunk(tc)) => {
                let call_id = tc.tool_call.call_id.clone();
                let name = tc.tool_call.fn_name.clone();
                let current_args = match tc.tool_call.fn_arguments {
                    serde_json::Value::String(s) => s,
                    other => other.to_string(),
                };
                let prev_args = args_seen.get(&call_id).cloned().unwrap_or_default();

                if !args_seen.contains_key(&call_id)
                    && tx
                        .send(Ok(ChatChunk::ToolCallStart {
                            id: call_id.clone(),
                            name,
                        }))
                        .await
                        .is_err()
                {
                    return;
                }

                if current_args != prev_args {
                    let delta = match current_args.strip_prefix(&prev_args) {
                        Some(suffix) => suffix.to_string(),
                        None => {
                            warn!(
                                call_id = %call_id,
                                prev_len = prev_args.len(),
                                current_len = current_args.len(),
                                "genai cumulative-args invariant broken, sending full string as delta"
                            );
                            current_args.clone()
                        }
                    };
                    if !delta.is_empty()
                        && tx
                            .send(Ok(ChatChunk::ToolCallArgsDelta {
                                id: call_id.clone(),
                                args_fragment: delta,
                            }))
                            .await
                            .is_err()
                    {
                        return;
                    }
                }

                args_seen.insert(call_id, current_args);
            }
            Ok(ChatStreamEvent::Start) => continue,
            Ok(ChatStreamEvent::End(_end)) => {
                let had_tool_calls = !args_seen.is_empty();
                for id in std::mem::take(&mut args_seen).into_keys() {
                    if tx
                        .send(Ok(ChatChunk::ToolCallDone { id }))
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
                let reason = if had_tool_calls {
                    FinishReason::ToolUse
                } else {
                    FinishReason::Stop
                };
                let _ = tx.send(Ok(ChatChunk::Done { reason })).await;
                return;
            }
            Err(e) => {
                warn!("genai stream error: {e}");
                let _ = tx.send(Err(classify_genai_error(e.to_string()))).await;
                return;
            }
        }
    }
}

/// Map a genai error string into our `LlmError` taxonomy. Kept in this module
/// so providers do not need to duplicate it.
pub(crate) fn classify_genai_error(msg: String) -> LlmError {
    let lower = msg.to_lowercase();
    if lower.contains("rate") {
        LlmError::RateLimit
    } else if lower.contains("auth") || lower.contains("api key") || lower.contains("401") {
        LlmError::AuthFailure
    } else if lower.contains("network")
        || lower.contains("connection")
        || lower.contains("timeout")
    {
        LlmError::Network(msg)
    } else {
        LlmError::Provider(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_with_image_part_emits_binary_in_content() {
        let messages = vec![ChatMessage::User {
            parts: vec![
                MessagePart::Text {
                    text: "look".into(),
                },
                MessagePart::Image {
                    mime: "image/png".into(),
                    data_b64: "aGVsbG8=".into(),
                    name: None,
                },
            ],
        }];
        let g_req = convert_messages(messages, vec![]);
        assert_eq!(g_req.messages.len(), 1);
        let content = &g_req.messages[0].content;
        assert!(content.contains_text(), "must keep the text part");
        assert!(content.contains_binary(), "must carry an image part");
        let bins = content.binaries();
        assert_eq!(bins.len(), 1);
        assert!(bins[0].content_type.starts_with("image/"));
    }

    #[test]
    fn user_text_only_does_not_produce_binary() {
        let messages = vec![ChatMessage::user_text("hi")];
        let g_req = convert_messages(messages, vec![]);
        assert_eq!(g_req.messages.len(), 1);
        let content = &g_req.messages[0].content;
        assert_eq!(content.first_text(), Some("hi"));
        assert!(!content.contains_binary());
    }

    #[test]
    fn classify_rate_limit() {
        assert!(matches!(
            classify_genai_error("rate limit hit".into()),
            LlmError::RateLimit
        ));
    }
}
