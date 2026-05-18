use async_trait::async_trait;
use futures::stream::StreamExt;
use genai::Client;
use genai::ModelIden;
use genai::adapter::AdapterKind;
use genai::chat::ChatOptions;
use genai::resolver::{AuthData, AuthResolver};
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;

use crate::genai_common::{build_chat_options, classify_genai_error, convert_messages, pump_genai_stream};
use crate::provider::{Capabilities, ChatChunk, ChatRequest, ChunkStream, LlmError, LlmProvider};

pub const DEFAULT_ANTHROPIC_MODEL: &str = "claude-haiku-4-5-20251001";

pub struct AnthropicProvider {
    client: Client,
    default_model: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String) -> Self {
        Self::with_default_model(api_key, DEFAULT_ANTHROPIC_MODEL.to_string())
    }

    pub fn with_default_model(api_key: String, default_model: String) -> Self {
        let key = Arc::new(api_key);
        let resolver =
            AuthResolver::from_resolver_fn(move |_iden: ModelIden| -> Result<
                Option<AuthData>,
                genai::resolver::Error,
            > {
                Ok(Some(AuthData::from_single(key.as_str().to_string())))
            });
        let client = Client::builder().with_auth_resolver(resolver).build();
        Self {
            client,
            default_model,
        }
    }

    fn build_options(req: &ChatRequest) -> ChatOptions {
        // Start with reasoning options if present, then layer in max_tokens/temperature.
        let mut options = build_chat_options(req.reasoning).unwrap_or_default();
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
        "anthropic"
    }

    fn capabilities_for_model(&self, model_id: &str) -> Capabilities {
        let lc = model_id.to_ascii_lowercase();
        if lc.contains("opus-4")
            || lc.contains("sonnet-4")
            || lc.contains("haiku-4")
        {
            Capabilities {
                vision_input: true,
                reasoning: true,
                tool_calls: true,
                streaming: true,
            }
        } else {
            Capabilities {
                vision_input: false,
                reasoning: false,
                tool_calls: true,
                streaming: true,
            }
        }
    }

    fn active_model(&self) -> &str {
        &self.default_model
    }
}

#[allow(dead_code)]
fn anthropic_kind() -> AdapterKind {
    AdapterKind::Anthropic
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anthropic_caps_opus_4_7_has_reasoning() {
        let p = AnthropicProvider::new("dummy".into());
        let caps = p.capabilities_for_model("claude-opus-4-7");
        assert!(caps.reasoning, "claude-opus-4-7 must report reasoning=true");
    }

    #[test]
    fn anthropic_caps_claude_4_models_all_true() {
        let p = AnthropicProvider::new("dummy".into());
        for id in [
            "claude-opus-4-7",
            "claude-sonnet-4-6",
            "claude-haiku-4-5-20251001",
        ] {
            let caps = p.capabilities_for_model(id);
            assert!(caps.vision_input, "vision for {id}");
            assert!(caps.reasoning, "reasoning for {id}");
            assert!(caps.tool_calls, "tools for {id}");
            assert!(caps.streaming, "streaming for {id}");
        }
    }

    #[test]
    fn anthropic_caps_unknown_model_conservative_default() {
        let p = AnthropicProvider::new("dummy".into());
        let caps = p.capabilities_for_model("claude-something-from-the-future");
        assert!(!caps.vision_input);
        assert!(!caps.reasoning);
        assert!(caps.tool_calls);
        assert!(caps.streaming);
    }

    #[test]
    fn anthropic_active_model_is_default_haiku() {
        let p = AnthropicProvider::new("dummy".into());
        assert_eq!(p.active_model(), "claude-haiku-4-5-20251001");
    }
}
