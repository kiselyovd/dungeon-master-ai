//! OpenAI-compatible provider: works with any endpoint that speaks the
//! `/v1/chat/completions` protocol. Tested targets:
//! - LM Studio (`http://localhost:1234/v1`)
//! - Ollama (`http://localhost:11434/v1`)
//! - llama.cpp server (`http://localhost:8080/v1`)
//! - vLLM, mistral.rs server-mode
//! - OpenRouter, Groq, DeepSeek, Together, Fireworks (cloud)
//!
//! The user supplies `base_url`, `model`, and `api_key` from the Settings UI;
//! we lean on `genai`'s OpenAI adapter for the wire protocol.

use async_trait::async_trait;
use futures::stream::StreamExt;
use genai::adapter::AdapterKind;
use genai::chat::{
    ChatMessage as GMsg, ChatOptions, ChatRequest as GReq, ChatStreamEvent,
};
use genai::resolver::{AuthData, Endpoint, ServiceTargetResolver};
use genai::{Client, ModelIden, ServiceTarget};
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::warn;

use crate::provider::{
    ChatChunk, ChatMessage, ChatRequest, ChunkStream, FinishReason, LlmError, LlmProvider,
};

pub struct OpenAICompatProvider {
    client: Client,
}

impl OpenAICompatProvider {
    /// `base_url` should be the root that contains `/chat/completions`. Both
    /// `http://localhost:1234` and `http://localhost:1234/v1` are accepted -
    /// genai's OpenAI adapter appends the suffix it needs.
    pub fn new(base_url: String, api_key: String) -> Self {
        let endpoint_url = Arc::new(base_url);
        let key = Arc::new(api_key);

        let resolver = ServiceTargetResolver::from_resolver_fn(
            move |target: ServiceTarget| -> Result<ServiceTarget, genai::resolver::Error> {
                let ServiceTarget { model, .. } = target;
                let endpoint = Endpoint::from_owned(endpoint_url.as_str().to_string());
                let auth = AuthData::from_single(key.as_str().to_string());
                let model = ModelIden::new(AdapterKind::OpenAI, model.model_name);
                Ok(ServiceTarget {
                    endpoint,
                    auth,
                    model,
                })
            },
        );

        let client = Client::builder()
            .with_service_target_resolver(resolver)
            .build();

        Self { client }
    }

    fn convert_messages(messages: Vec<ChatMessage>) -> GReq {
        let mut g_req = GReq::default();
        for m in messages {
            match m {
                ChatMessage::System { content } => g_req = g_req.with_system(content),
                ChatMessage::User { content } => {
                    g_req = g_req.append_message(GMsg::user(content));
                }
                ChatMessage::Assistant { content } => {
                    g_req = g_req.append_message(GMsg::assistant(content));
                }
                ChatMessage::AssistantWithToolCalls { content, tool_calls: _ } => {
                    if let Some(text) = content {
                        g_req = g_req.append_message(GMsg::assistant(text));
                    }
                }
                ChatMessage::ToolResult(_) => {
                    // Full tool-result injection implemented in Phase B follow-up.
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
impl LlmProvider for OpenAICompatProvider {
    async fn stream_chat(&self, req: ChatRequest) -> Result<ChunkStream, LlmError> {
        let model = req.model.clone();
        let options = Self::build_options(&req);
        let g_req = Self::convert_messages(req.messages);

        let stream_response = self
            .client
            .exec_chat_stream(&model, g_req, Some(&options))
            .await
            .map_err(|e| classify_genai_error(e.to_string()))?;

        let mut g_stream = stream_response.stream;
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<ChatChunk, LlmError>>(64);

        tokio::spawn(async move {
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
                    Ok(ChatStreamEvent::ToolCallChunk(_)) => continue,
                    Ok(ChatStreamEvent::Start) => continue,
                    Ok(ChatStreamEvent::End(_)) => {
                        let _ = tx
                            .send(Ok(ChatChunk::Done {
                                reason: FinishReason::Stop,
                            }))
                            .await;
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
        "openai-compat"
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
