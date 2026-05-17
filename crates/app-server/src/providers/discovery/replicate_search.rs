//! Replicate model catalog search with cursor pagination.

use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;

use app_llm::Capabilities;

use super::types::{
    DiscoverParams, DiscoveryError, DiscoveryResult, DiscoverySource, ModelSource,
    ResolvedModelEntry,
};

const DEFAULT_ENDPOINT: &str = "https://api.replicate.com";

#[derive(Deserialize)]
struct ReplicateModelsResponse {
    results: Vec<ReplicateModelEntry>,
    #[serde(default)]
    next: Option<String>,
}

#[derive(Deserialize)]
struct ReplicateModelEntry {
    owner: String,
    name: String,
    #[serde(default)]
    description: Option<String>,
}

pub struct ReplicateSearch {
    endpoint: String,
    client: reqwest::Client,
}

impl Default for ReplicateSearch {
    fn default() -> Self {
        Self::with_endpoint_override(DEFAULT_ENDPOINT.to_string())
    }
}

impl ReplicateSearch {
    pub fn with_endpoint_override(endpoint: String) -> Self {
        Self {
            endpoint,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl DiscoverySource for ReplicateSearch {
    async fn discover(&self, params: DiscoverParams) -> Result<DiscoveryResult, DiscoveryError> {
        let api_key = params
            .api_key
            .as_deref()
            .filter(|k| !k.is_empty())
            .ok_or_else(|| DiscoveryError::Provider("Replicate api_key required".into()))?;
        let mut url = format!("{}/v1/models", self.endpoint.trim_end_matches('/'));
        if let Some(cursor) = params.cursor.as_deref() {
            url.push_str(&format!("?cursor={}", urlencoding::encode(cursor)));
        }
        let resp = self
            .client
            .get(&url)
            .bearer_auth(api_key)
            .send()
            .await
            .map_err(|e| DiscoveryError::Network(e.to_string()))?;
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(DiscoveryError::Unauthorized);
        }
        if !resp.status().is_success() {
            return Err(DiscoveryError::Provider(format!(
                "Replicate returned {}",
                resp.status()
            )));
        }
        let body: ReplicateModelsResponse = resp
            .json()
            .await
            .map_err(|e| DiscoveryError::Provider(e.to_string()))?;
        let models: Vec<ResolvedModelEntry> = body
            .results
            .into_iter()
            .map(|m| {
                let model_id = format!("{}/{}", m.owner, m.name);
                ResolvedModelEntry {
                    model_id: model_id.clone(),
                    display_name: m.description.unwrap_or(model_id),
                    capabilities: Capabilities {
                        vision_input: false,
                        reasoning: false,
                        tool_calls: false,
                        streaming: false,
                    },
                    source: ModelSource::DiscoveredApi,
                    context_length: None,
                    price_per_million_input: None,
                    price_per_million_output: None,
                }
            })
            .collect();
        Ok(DiscoveryResult {
            models,
            cached_at: Utc::now(),
            source: ModelSource::DiscoveredApi,
            next_cursor: body.next,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn handles_cursor_pagination() {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "results": [
                    { "owner": "stability-ai", "name": "sdxl", "description": "SDXL" }
                ],
                "next": "page2"
            })))
            .mount(&mock)
            .await;
        let source = ReplicateSearch::with_endpoint_override(mock.uri());
        let result = source
            .discover(DiscoverParams {
                provider_id: "replicate".into(),
                api_key: Some("r8_xxx".into()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(result.models[0].model_id, "stability-ai/sdxl");
        assert_eq!(result.next_cursor.as_deref(), Some("page2"));
    }
}
