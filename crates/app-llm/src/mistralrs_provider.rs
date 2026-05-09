//! `MistralrsLocalProvider` is a thin wrapper over `OpenAICompatProvider`,
//! pinned to a localhost port and tagged with a distinct provider name so the
//! Settings UI and tracing can distinguish it from BYO-OpenAI-compat targets.

use crate::openai_compat::OpenAICompatProvider;
use crate::provider::{ChatRequest, ChunkStream, LlmError, LlmProvider};
use async_trait::async_trait;

pub struct MistralrsLocalProvider {
    inner: OpenAICompatProvider,
    base_url: String,
    model: String,
}

impl MistralrsLocalProvider {
    pub fn new(port: u16, model: String) -> Self {
        let base_url = format!("http://127.0.0.1:{port}/v1");
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

    fn supports_vision(&self) -> bool {
        true
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
    fn base_url_is_loopback_with_v1_suffix() {
        let p = MistralrsLocalProvider::new(37000, "qwen3.5-4b".into());
        assert_eq!(p.base_url(), "http://127.0.0.1:37000/v1");
    }

    #[test]
    fn model_returned_via_accessor() {
        let p = MistralrsLocalProvider::new(37000, "qwen3.5-4b".into());
        assert_eq!(p.model(), "qwen3.5-4b");
    }
}
