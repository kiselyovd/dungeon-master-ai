use async_trait::async_trait;
use futures::stream::StreamExt;
use genai::Client;
use genai::ModelIden;
use genai::adapter::AdapterKind;
use genai::chat::{
    ChatMessage as GMsg, ChatOptions, ChatRequest as GReq, ChatStreamEvent, Tool as GTool,
    ToolCall as GToolCall, ToolResponse as GToolResponse,
};
use genai::resolver::{AuthData, AuthResolver};
use std::collections::HashMap;
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::warn;

use crate::provider::{
    ChatChunk, ChatMessage, ChatRequest, ChunkStream, FinishReason, LlmError, LlmProvider, Tool,
};

pub struct AnthropicProvider {
    client: Client,
}

impl AnthropicProvider {
    pub fn new(api_key: String) -> Self {
        let key = Arc::new(api_key);
        let resolver =
            AuthResolver::from_resolver_fn(move |_iden: ModelIden| -> Result<
                Option<AuthData>,
                genai::resolver::Error,
            > {
                Ok(Some(AuthData::from_single(key.as_str().to_string())))
            });
        let client = Client::builder().with_auth_resolver(resolver).build();
        Self { client }
    }

    fn convert_messages(messages: Vec<ChatMessage>, tools: Vec<Tool>) -> GReq {
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
                ChatMessage::User { content } => {
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
                    let response = GToolResponse::new(tr.tool_call_id, tr.content);
                    g_req = g_req.append_message(GMsg::from(response));
                }
            }
        }
        g_req
    }

    fn build_options(req: &ChatRequest) -> ChatOptions {
        let mut options = ChatOptions::default();
        if let Some(max) = req.max_tokens {
            options = options.with_max_tokens(max);
        }
        if let Some(temp) = req.temperature {
            options = options.with_temperature(temp as f64);
        }
        options
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn stream_chat(&self, req: ChatRequest) -> Result<ChunkStream, LlmError> {
        let model = req.model.clone();
        let options = Self::build_options(&req);
        let g_req = Self::convert_messages(req.messages, req.tools);

        let stream_response = self
            .client
            .exec_chat_stream(&model, g_req, Some(&options))
            .await
            .map_err(|e| classify_genai_error(e.to_string()))?;

        let mut g_stream = stream_response.stream;
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<ChatChunk, LlmError>>(64);

        tokio::spawn(async move {
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
                        let current_args = tc
                            .tool_call
                            .fn_arguments
                            .as_str()
                            .unwrap_or("")
                            .to_string();
                        let prev_args =
                            args_seen.get(&call_id).cloned().unwrap_or_default();

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
                            let delta = current_args
                                .strip_prefix(&prev_args)
                                .unwrap_or(&current_args)
                                .to_string();
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
                        for id in args_seen.keys().cloned().collect::<Vec<_>>() {
                            if tx
                                .send(Ok(ChatChunk::ToolCallDone { id }))
                                .await
                                .is_err()
                            {
                                return;
                            }
                        }
                        let reason = if args_seen.is_empty() {
                            FinishReason::Stop
                        } else {
                            FinishReason::ToolUse
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
        });

        Ok(ReceiverStream::new(rx).boxed())
    }

    fn name(&self) -> &'static str {
        "anthropic"
    }

    fn supports_tools(&self) -> bool {
        true
    }
}

fn classify_genai_error(msg: String) -> LlmError {
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

#[allow(dead_code)]
fn anthropic_kind() -> AdapterKind {
    AdapterKind::Anthropic
}
