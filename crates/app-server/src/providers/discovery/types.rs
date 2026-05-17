use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use app_llm::Capabilities;

/// Where a discovered model entry came from. The frontend renders different
/// section headers per source ("Discovered (API)" vs "Discovered (HF Hub)").
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ModelSource {
    Curated,
    DiscoveredApi,
    DiscoveredHfHub,
    CustomHf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedModelEntry {
    pub model_id: String,
    pub display_name: String,
    pub capabilities: Capabilities,
    pub source: ModelSource,
    #[serde(default)]
    pub context_length: Option<u32>,
    #[serde(default)]
    pub price_per_million_input: Option<f32>,
    #[serde(default)]
    pub price_per_million_output: Option<f32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DiscoverParams {
    pub provider_id: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub search_query: Option<String>,
    #[serde(default)]
    pub cursor: Option<String>,
}

impl DiscoverParams {
    pub fn for_provider(id: impl Into<String>) -> Self {
        Self {
            provider_id: id.into(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DiscoveryResult {
    pub models: Vec<ResolvedModelEntry>,
    pub cached_at: DateTime<Utc>,
    pub source: ModelSource,
    #[serde(default)]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("network error: {0}")]
    Network(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("rate limit")]
    RateLimit,
    #[error("provider error: {0}")]
    Provider(String),
    #[error("unsupported provider for discovery: {0}")]
    UnsupportedProvider(String),
}

#[async_trait]
pub trait DiscoverySource: Send + Sync {
    async fn discover(&self, params: DiscoverParams) -> Result<DiscoveryResult, DiscoveryError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_result_serialises_with_iso_timestamp() {
        let r = DiscoveryResult {
            models: vec![],
            cached_at: Utc::now(),
            source: ModelSource::Curated,
            next_cursor: None,
        };
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains("\"cached_at\""));
        assert!(s.contains("\"source\":\"curated\""));
    }
}
