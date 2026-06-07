//! `MistralrsLocalProvider` is a thin wrapper over `OpenAICompatProvider`,
//! pinned to a localhost port and tagged with a distinct provider name so the
//! Settings UI and tracing can distinguish it from BYO-OpenAI-compat targets.
//!
//! NOTE (M9-DM, 2026-05-19): mistralrs-server 0.6.x does not surface
//! `reasoning_content` for Qwen3 thinking models in the OpenAI-compat SSE
//! delta. The `chat_template_kwargs: { enable_thinking: true }` body field is
//! not wired here for that reason. Revisit on mistralrs 0.7+ or after an
//! upstream PR enables a dedicated thinking channel. Until then, every
//! local-mistralrs catalog entry pins `caps.reasoning = false`.

use crate::openai_compat::OpenAICompatProvider;
use crate::provider::{Capabilities, ChatRequest, ChunkStream, LlmError, LlmProvider};
use async_trait::async_trait;

pub struct MistralrsLocalProvider {
    inner: OpenAICompatProvider,
    base_url: String,
    model: String,
}

impl MistralrsLocalProvider {
    pub fn new(port: u16, model: String) -> Self {
        // genai joins the relative suffix "chat/completions" onto the endpoint
        // via `reqwest::Url::join`. That join REPLACES the last path segment
        // unless the base ends in a slash: ".../v1" -> ".../chat/completions"
        // (404), but ".../v1/" -> ".../v1/chat/completions" (200). The trailing
        // slash is load-bearing - without it every DM turn 404s silently
        // (verified live against mistralrs-server 0.6.x).
        let base_url = format!("http://127.0.0.1:{port}/v1/");
        let inner = OpenAICompatProvider::new(base_url.clone(), "not-used".into());
        Self {
            inner,
            base_url,
            model,
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn model(&self) -> &str {
        &self.model
    }
}

#[async_trait]
impl LlmProvider for MistralrsLocalProvider {
    fn name(&self) -> &'static str {
        "local-mistralrs"
    }

    async fn stream_chat(&self, mut req: ChatRequest) -> Result<ChunkStream, LlmError> {
        if req.model.is_empty() {
            req.model = self.model.clone();
        }
        self.inner.stream_chat(req).await
    }

    fn capabilities_for_model(&self, model_id: &str) -> Capabilities {
        let lc = model_id.to_ascii_lowercase();
        // Gemma 4 E-variants (the only Gemma we ship) are multimodal, emit a
        // distinct `reasoning_content` channel (mistralrs surfaces it - verified
        // live, unlike the Qwen3 limitation noted at the top of this file), and
        // support function calling. Recognise the family explicitly so the agent
        // offers tools (image/map generation) and the UI shows vision/reasoning.
        let is_gemma = lc.contains("gemma");
        // Qwen3.x family is uniformly VL+thinking in upstream naming as of
        // 2026; bare "qwen3.5-*-instruct" without -vl is also a vision model.
        // Older Qwen2.5 family ships separate -vl variants for vision.
        let vision_input =
            lc.contains("qwen3") || lc.contains("-vl") || lc.contains("vision") || is_gemma;
        let reasoning = lc.contains("qwen3") || is_gemma; // built-in thinking channel
        let tool_calls = lc.contains("instruct") || lc.contains("qwen3") || is_gemma;
        Capabilities {
            vision_input,
            reasoning,
            tool_calls,
            streaming: true,
        }
    }

    fn active_model(&self) -> &str {
        &self.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_name_is_local_mistralrs() {
        let p = MistralrsLocalProvider::new(37000, "qwen3.5-4b".into());
        assert_eq!(p.name(), "local-mistralrs");
    }

    #[test]
    fn base_url_is_loopback_v1_with_trailing_slash() {
        // The trailing slash is required so genai's `Url::join("chat/completions")`
        // preserves the `/v1/` segment instead of replacing it (404 otherwise).
        let p = MistralrsLocalProvider::new(37000, "qwen3.5-4b".into());
        assert_eq!(p.base_url(), "http://127.0.0.1:37000/v1/");
    }

    #[test]
    fn model_returned_via_accessor() {
        let p = MistralrsLocalProvider::new(37000, "qwen3.5-4b".into());
        assert_eq!(p.model(), "qwen3.5-4b");
    }

    #[test]
    fn qwen3_5_caps_all_true_vision_reasoning_tools() {
        let p = MistralrsLocalProvider::new(37000, "qwen3.5-4b".into());
        let caps = p.capabilities_for_model("qwen3.5-4b");
        assert!(caps.vision_input);
        assert!(caps.reasoning);
        assert!(caps.tool_calls);
        assert!(caps.streaming);
    }

    #[test]
    fn qwen2_5_non_vl_no_vision() {
        let p = MistralrsLocalProvider::new(37000, "qwen2.5-7b-instruct".into());
        let caps = p.capabilities_for_model("qwen2.5-7b-instruct");
        assert!(!caps.vision_input);
        assert!(caps.tool_calls);
    }

    #[test]
    fn qwen2_5_vl_has_vision() {
        let p = MistralrsLocalProvider::new(37000, "qwen2.5-vl-7b-instruct".into());
        assert!(
            p.capabilities_for_model("qwen2.5-vl-7b-instruct")
                .vision_input
        );
    }

    #[test]
    fn active_model_is_constructor_arg() {
        let p = MistralrsLocalProvider::new(37000, "qwen3.5-9b".into());
        assert_eq!(p.active_model(), "qwen3.5-9b");
    }

    #[test]
    fn gemma4_caps_vision_reasoning_and_tools_true() {
        // Gemma 4 E-it is multimodal + thinking + tool-calling. Without this the
        // DM agent never offers tools, so scene/map image generation is dead.
        let p = MistralrsLocalProvider::new(37000, "google/gemma-4-E2B-it".into());
        let caps = p.capabilities_for_model("google/gemma-4-E2B-it");
        assert!(caps.tool_calls, "gemma must support tool calls");
        assert!(caps.vision_input, "gemma E-it is multimodal");
        assert!(caps.reasoning, "gemma emits a reasoning channel");
        assert!(caps.streaming);
    }
}
