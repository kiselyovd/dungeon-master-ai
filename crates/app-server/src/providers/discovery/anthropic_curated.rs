//! Anthropic doesn't expose a /models endpoint we can rely on in the OAuth
//! adapter we use, so the "discovery" source returns the same hardcoded
//! Claude 4 models as the catalog. Keeping the source uniform means the
//! frontend's Discover button can render a consistent UI for all providers.

use async_trait::async_trait;
use chrono::Utc;

use app_llm::Capabilities;

use super::types::{
    DiscoverParams, DiscoveryError, DiscoveryResult, DiscoverySource, ModelSource,
    ResolvedModelEntry,
};

pub struct AnthropicCurated;

const fn caps_all() -> Capabilities {
    Capabilities {
        vision_input: true,
        reasoning: true,
        tool_calls: true,
        streaming: true,
    }
}

#[async_trait]
impl DiscoverySource for AnthropicCurated {
    async fn discover(&self, _params: DiscoverParams) -> Result<DiscoveryResult, DiscoveryError> {
        let models = vec![
            ResolvedModelEntry {
                model_id: "claude-opus-4-7".to_string(),
                display_name: "Claude Opus 4.7".to_string(),
                capabilities: caps_all(),
                source: ModelSource::Curated,
                context_length: Some(1_000_000),
                price_per_million_input: Some(15.0),
                price_per_million_output: Some(75.0),
            },
            ResolvedModelEntry {
                model_id: "claude-sonnet-4-6".to_string(),
                display_name: "Claude Sonnet 4.6".to_string(),
                capabilities: caps_all(),
                source: ModelSource::Curated,
                context_length: Some(200_000),
                price_per_million_input: Some(3.0),
                price_per_million_output: Some(15.0),
            },
            ResolvedModelEntry {
                model_id: "claude-haiku-4-5-20251001".to_string(),
                display_name: "Claude Haiku 4.5".to_string(),
                capabilities: caps_all(),
                source: ModelSource::Curated,
                context_length: Some(200_000),
                price_per_million_input: Some(0.8),
                price_per_million_output: Some(4.0),
            },
        ];
        Ok(DiscoveryResult {
            models,
            cached_at: Utc::now(),
            source: ModelSource::Curated,
            next_cursor: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_three_claude_4_models() {
        let result = AnthropicCurated
            .discover(DiscoverParams::for_provider("anthropic"))
            .await
            .unwrap();
        assert_eq!(result.models.len(), 3);
        assert!(result.models.iter().any(|m| m.model_id.contains("opus-4")));
    }
}
