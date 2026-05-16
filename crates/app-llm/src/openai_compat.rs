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
use crate::provider::{Capabilities, ChatChunk, ChatRequest, ChunkStream, LlmError, LlmProvider};

pub struct OpenAICompatProvider {
    client: Client,
    default_model: String,
    capabilities_override: Option<Capabilities>,
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

        Self {
            client,
            default_model: String::new(),
            capabilities_override: None,
        }
    }

    /// Chainable: set the model id used by `active_model()` and `capabilities()`.
    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// Chainable: user-supplied capability override (settings UI disclosure
    /// for openai-compat where the inferred caps are wrong for a custom model).
    pub fn with_capabilities_override(mut self, caps: Option<Capabilities>) -> Self {
        self.capabilities_override = caps;
        self
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

    fn capabilities_for_model(&self, model_id: &str) -> Capabilities {
        if let Some(caps) = self.capabilities_override {
            return caps;
        }
        let lc = model_id.to_ascii_lowercase();
        let vision_input =
            lc.contains("gpt-4o") || lc.contains("gpt-5") || lc.starts_with("o4");
        let reasoning =
            lc.starts_with("o1") || lc.starts_with("o3") || lc.starts_with("o4");
        Capabilities {
            vision_input,
            reasoning,
            tool_calls: true,
            streaming: true,
        }
    }

    fn active_model(&self) -> &str {
        &self.default_model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_compat_unknown_model_conservative_defaults() {
        let p = OpenAICompatProvider::new("http://x".into(), "k".into());
        let caps = p.capabilities_for_model("custom-fine-tune-v0.42");
        assert!(!caps.vision_input);
        assert!(!caps.reasoning);
        assert!(caps.tool_calls);
        assert!(caps.streaming);
    }

    #[test]
    fn openai_compat_infers_o_series_reasoning() {
        let p = OpenAICompatProvider::new("http://x".into(), "k".into());
        assert!(p.capabilities_for_model("o3-mini").reasoning);
        assert!(p.capabilities_for_model("o4-mini").reasoning);
    }

    #[test]
    fn openai_compat_infers_vision_for_known_models() {
        let p = OpenAICompatProvider::new("http://x".into(), "k".into());
        assert!(p.capabilities_for_model("gpt-4o").vision_input);
        assert!(p.capabilities_for_model("gpt-5").vision_input);
        assert!(!p.capabilities_for_model("text-davinci-003").vision_input);
    }

    #[test]
    fn openai_compat_capabilities_override_wins() {
        let p = OpenAICompatProvider::new("http://x".into(), "k".into())
            .with_capabilities_override(Some(Capabilities {
                vision_input: true,
                reasoning: true,
                tool_calls: true,
                streaming: true,
            }));
        let caps = p.capabilities_for_model("custom-tiny-model");
        assert!(caps.vision_input);
        assert!(caps.reasoning);
    }

    #[test]
    fn openai_compat_active_model_is_settable() {
        let p =
            OpenAICompatProvider::new("http://x".into(), "k".into()).with_default_model("gpt-5");
        assert_eq!(p.active_model(), "gpt-5");
    }
}
