//! HF Hub search API for local-mistralrs (search GGUF models the user can
//! download via the Custom HF flow).

use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;

use super::capability_infer::infer_capabilities;
use super::types::{
    DiscoverParams, DiscoveryError, DiscoveryResult, DiscoverySource, ModelSource,
    ResolvedModelEntry,
};

const DEFAULT_ENDPOINT: &str = "https://huggingface.co";

#[derive(Deserialize)]
struct HfHubModelEntry {
    id: String,
    #[serde(default)]
    tags: Vec<String>,
}

pub struct HfHubSearch {
    endpoint: String,
    client: reqwest::Client,
}

impl Default for HfHubSearch {
    fn default() -> Self {
        Self::with_endpoint_override(DEFAULT_ENDPOINT.to_string())
    }
}

impl HfHubSearch {
    pub fn with_endpoint_override(endpoint: String) -> Self {
        Self {
            endpoint,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl DiscoverySource for HfHubSearch {
    async fn discover(&self, params: DiscoverParams) -> Result<DiscoveryResult, DiscoveryError> {
        let query = params.search_query.unwrap_or_default();
        let url = format!(
            "{}/api/models?search={}&filter=gguf&limit=50",
            self.endpoint.trim_end_matches('/'),
            urlencoding::encode(&query)
        );
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| DiscoveryError::Network(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(DiscoveryError::Provider(format!(
                "HF Hub returned {}",
                resp.status()
            )));
        }
        let entries: Vec<HfHubModelEntry> = resp
            .json()
            .await
            .map_err(|e| DiscoveryError::Provider(e.to_string()))?;
        let models: Vec<ResolvedModelEntry> = entries
            .into_iter()
            .map(|m| {
                let caps = infer_capabilities("local-mistralrs", &m.id, &m.tags);
                ResolvedModelEntry {
                    model_id: m.id.clone(),
                    display_name: m.id,
                    capabilities: caps,
                    source: ModelSource::DiscoveredHfHub,
                    context_length: None,
                    price_per_million_input: None,
                    price_per_million_output: None,
                }
            })
            .collect();
        Ok(DiscoveryResult {
            models,
            cached_at: Utc::now(),
            source: ModelSource::DiscoveredHfHub,
            next_cursor: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn parses_models_and_infers_vl_tag() {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "id": "Qwen/Qwen2.5-VL-7B-Instruct-GGUF",
                    "tags": ["vision-language", "conversational"]
                },
                {
                    "id": "TheBloke/Llama-3-8B-Instruct-GGUF",
                    "tags": ["conversational"]
                }
            ])))
            .mount(&mock)
            .await;
        let source = HfHubSearch::with_endpoint_override(mock.uri());
        let result = source
            .discover(DiscoverParams {
                provider_id: "local-mistralrs".into(),
                search_query: Some("qwen vl".into()),
                ..Default::default()
            })
            .await
            .unwrap();
        let qwen = result
            .models
            .iter()
            .find(|m| m.model_id.contains("Qwen2.5-VL"))
            .unwrap();
        assert!(qwen.capabilities.vision_input);
    }
}
