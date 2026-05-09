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
use genai::chat::ChatOptions;
use genai::resolver::{AuthData, Endpoint, ServiceTargetResolver};
use genai::{Client, ModelIden, ServiceTarget};
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;

use crate::genai_common::{classify_genai_error, convert_messages, pump_genai_stream};
use crate::provider::{ChatChunk, ChatRequest, ChunkStream, LlmError, LlmProvider};

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
        let g_req = convert_messages(req.messages, req.tools);

        let stream_response = self
            .client
            .exec_chat_stream(&model, g_req, Some(&options))
            .await
            .map_err(|e| classify_genai_error(e.to_string()))?;

        let g_stream = stream_response.stream;
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<ChatChunk, LlmError>>(64);

        tokio::spawn(pump_genai_stream(g_stream, tx));

        Ok(ReceiverStream::new(rx).boxed())
    }

    fn name(&self) -> &'static str {
        "openai-compat"
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn supports_vision(&self) -> bool {
        true
    }
}
