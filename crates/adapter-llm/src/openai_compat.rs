//! Outbound OpenAI-compatible provider for endpoints that speak the
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

use crate::genai_common::{
    build_chat_options, classify_genai_error, convert_messages, pump_genai_stream,
};
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
        // Start with reasoning options if present, then layer in max_tokens/temperature.
        // The shared pump consumes cumulative tool arguments and stable IDs.
        // Without capture, genai emits raw deltas and fallback IDs on continuations.
        let mut options = build_chat_options(req.reasoning)
            .unwrap_or_default()
            .with_capture_tool_calls(true);
        if let Some(max) = req.max_tokens {
            options = options.with_max_tokens(max);
        }
        if let Some(temp) = req.temperature {
            options = options.with_temperature(temp as f64);
        }
        options
    }
    async fn stream_with_schema(
        &self,
        req: ChatRequest,
        schema: Option<serde_json::Value>,
    ) -> Result<ChunkStream, LlmError> {
        let tool_round = !req.tools.is_empty();
        let model = req.model.clone();
        let mut options = Self::build_options(&req);
        if let Some(schema) = schema {
            options = options
                .with_response_format(genai::chat::JsonSpec::new("player_intention", schema));
        }
        let mut g_req = convert_messages(req.messages, req.tools);
        if let Some(system) = req.system_prompt {
            g_req = g_req.with_system(system);
        }

        // genai 0.6.5 drops all but the first tool in a single OpenAI SSE delta.
        // Read complete tool-enabled rounds through its existing non-stream API.
        // Plain text-only chat keeps incremental streaming below.
        if tool_round {
            let response = self
                .client
                .exec_chat(&model, g_req, Some(&options))
                .await
                .map_err(|error| classify_genai_error(error.to_string()))?;
            let chunks = complete_round_chunks(response)?;
            return Ok(futures::stream::iter(chunks.into_iter().map(Ok)).boxed());
        }

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
}

#[async_trait]
impl LlmProvider for OpenAICompatProvider {
    async fn stream_chat(&self, req: ChatRequest) -> Result<ChunkStream, LlmError> {
        self.stream_with_schema(req, None).await
    }

    async fn stream_chat_json(
        &self,
        req: ChatRequest,
        schema: serde_json::Value,
    ) -> Result<ChunkStream, LlmError> {
        self.stream_with_schema(req, Some(schema)).await
    }

    fn name(&self) -> &'static str {
        "openai-compat"
    }

    fn capabilities_for_model(&self, model_id: &str) -> Capabilities {
        if let Some(caps) = self.capabilities_override {
            return caps;
        }
        let lc = model_id.to_ascii_lowercase();
        let vision_input = lc.contains("gpt-4o") || lc.contains("gpt-5") || lc.starts_with("o4");
        let reasoning = lc.starts_with("o1")
            || lc.starts_with("o3")
            || lc.starts_with("o4")
            || lc.contains("gpt-5");
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

fn complete_round_chunks(response: genai::chat::ChatResponse) -> Result<Vec<ChatChunk>, LlmError> {
    use crate::provider::FinishReason;
    use genai::chat::StopReason;
    let native_calls = response.tool_calls();
    let (all_text, embedded_thinking) = separate_embedded_thinking(&response.texts().join(""));
    let legacy_calls = if native_calls.is_empty() {
        crate::genai_common::completed_text_tool_calls(&all_text)
    } else {
        vec![]
    };
    let legacy = !legacy_calls.is_empty();
    let calls = if legacy {
        legacy_calls.iter().collect()
    } else {
        native_calls
    };
    let finish = match &response.stop_reason {
        Some(StopReason::Completed(_) | StopReason::StopSequence(_)) => {
            if calls.is_empty() {
                FinishReason::Stop
            } else {
                FinishReason::ToolUse
            }
        }
        Some(StopReason::ToolCall(_)) if !calls.is_empty() => FinishReason::ToolUse,
        Some(StopReason::ToolCall(_)) => {
            return Err(LlmError::InvalidResponse(
                "provider_tool_calls_missing".into(),
            ))
        }
        Some(StopReason::MaxTokens(_)) => {
            return Err(LlmError::InvalidResponse(
                "provider_output_truncated".into(),
            ))
        }
        Some(StopReason::ContentFilter(_)) => {
            return Err(LlmError::InvalidResponse("provider_output_filtered".into()))
        }
        Some(StopReason::Other(_)) | None => {
            return Err(LlmError::InvalidResponse(
                "provider_finish_reason_unknown".into(),
            ))
        }
    };
    if finish == FinishReason::ToolUse {
        let mut identities = std::collections::HashSet::new();
        if calls.iter().any(|call| {
            call.call_id.trim().is_empty()
                || call.fn_name.trim().is_empty()
                || !identities.insert(call.call_id.as_str())
        }) {
            return Err(LlmError::InvalidResponse(
                "provider_tool_identity_invalid".into(),
            ));
        }
    }
    let mut chunks = Vec::new();
    if let Some(reasoning) = &response.reasoning_content {
        if !reasoning.is_empty() {
            chunks.push(ChatChunk::ThinkingDelta {
                text: reasoning.clone(),
            });
        }
    }
    if !embedded_thinking.is_empty() {
        chunks.push(ChatChunk::ThinkingDelta {
            text: embedded_thinking,
        });
    }
    let public_text = if !calls.is_empty() && all_text.contains("<|tool_call>call:") {
        all_text.split("<|tool_call>call:").next().unwrap_or("")
    } else {
        &all_text
    };
    if finish == FinishReason::Stop && public_text.trim().is_empty() {
        return Err(LlmError::InvalidResponse(
            "provider_public_response_missing".into(),
        ));
    }
    // Tool-bearing prose is a plan, not an engine-confirmed outcome. Only the
    // subsequent final response may narrate results to the player.
    if finish == FinishReason::Stop && !public_text.is_empty() {
        chunks.push(ChatChunk::TextDelta {
            text: public_text.to_owned(),
        });
    }
    // Only a validated complete batch reaches this delivery boundary.
    if finish == FinishReason::ToolUse {
        for call in calls {
            chunks.push(ChatChunk::ToolCallStart {
                id: call.call_id.clone(),
                name: call.fn_name.clone(),
            });
            chunks.push(ChatChunk::ToolCallArgsDelta {
                id: call.call_id.clone(),
                args_fragment: call.fn_arguments.to_string(),
            });
            chunks.push(ChatChunk::ToolCallDone {
                id: call.call_id.clone(),
            });
        }
    }
    chunks.push(ChatChunk::Done { reason: finish });
    Ok(chunks)
}

/// Some compatible runtimes emit a duplicated closing tag in content after a
/// reasoning-budget cut. Treat the reserved prefix as thinking, never narration.
fn separate_embedded_thinking(text: &str) -> (String, String) {
    if let Some(close) = text.rfind("</think>") {
        let hidden = text[..close].replace("<think>", "");
        let remainder = &text[close + "</think>".len()..];
        let (visible, extra) = separate_embedded_thinking(remainder);
        return (visible, hidden + &extra);
    }
    if let Some(open) = text.find("<think>") {
        return (
            text[..open].to_owned(),
            text[open + "<think>".len()..].to_owned(),
        );
    }
    (text.to_owned(), String::new())
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
    fn openai_compat_o1_o3_o4_gpt5_report_reasoning_true() {
        let p = OpenAICompatProvider::new("http://x".into(), "k".into());
        for model in ["o1", "o1-mini", "o3", "o3-mini", "o4-mini", "gpt-5"] {
            assert!(
                p.capabilities_for_model(model).reasoning,
                "expected reasoning=true for {model}"
            );
        }
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
