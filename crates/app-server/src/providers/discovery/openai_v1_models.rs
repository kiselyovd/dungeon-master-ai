//! `GET {base_url}/v1/models` for OpenAI-compatible endpoints.

use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;

use super::capability_infer::infer_capabilities;
use super::types::{
    DiscoverParams, DiscoveryError, DiscoveryResult, DiscoverySource, ModelSource,
    ResolvedModelEntry,
};

#[derive(Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModelEntry>,
}

#[derive(Deserialize)]
struct OpenAIModelEntry {
    id: String,
    #[serde(default)]
    owned_by: Option<String>,
}

pub struct OpenAIV1Models {
    client: reqwest::Client,
}

impl OpenAIV1Models {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

impl Default for OpenAIV1Models {
    fn default() -> Self {
        Self::new(reqwest::Client::new())
    }
}

#[async_trait]
impl DiscoverySource for OpenAIV1Models {
    async fn discover(&self, params: DiscoverParams) -> Result<DiscoveryResult, DiscoveryError> {
        let base_url = params
            .base_url
            .as_deref()
            .ok_or_else(|| DiscoveryError::Provider("base_url required".into()))?;
        let trimmed = base_url.trim_end_matches('/');
        let url = if trimmed.ends_with("/v1") {
            format!("{trimmed}/models")
        } else {
            format!("{trimmed}/v1/models")
        };
        let mut req = self.client.get(&url);
        if let Some(key) = params.api_key.as_deref() {
            if !key.is_empty() {
                req = req.bearer_auth(key);
            }
        }
        let resp = req
            .send()
            .await
            .map_err(|e| DiscoveryError::Network(e.to_string()))?;
        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(DiscoveryError::Unauthorized);
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(DiscoveryError::RateLimit);
        }
        if !status.is_success() {
            return Err(DiscoveryError::Provider(format!(
                "remote returned {status}"
            )));
        }
        let body: OpenAIModelsResponse = resp
            .json()
            .await
            .map_err(|e| DiscoveryError::Provider(e.to_string()))?;
        let models: Vec<ResolvedModelEntry> = body
            .data
            .into_iter()
            .map(|m| {
                let caps = infer_capabilities(&params.provider_id, &m.id, &[]);
                let owned_by = m.owned_by.unwrap_or_default();
                let display_name = if owned_by.is_empty() {
                    m.id.clone()
                } else {
                    format!("{} ({})", m.id, owned_by)
                };
                ResolvedModelEntry {
                    model_id: m.id,
                    display_name,
                    capabilities: caps,
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
    async fn parses_data_array_and_infers_caps() {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    { "id": "gpt-5", "owned_by": "openai" },
                    { "id": "o3-mini", "owned_by": "openai" }
                ]
            })))
            .mount(&mock)
            .await;
        let source = OpenAIV1Models::default();
        let result = source
            .discover(DiscoverParams {
                provider_id: "openai-compat".into(),
                base_url: Some(mock.uri()),
                api_key: Some("dummy".into()),
                search_query: None,
                cursor: None,
            })
            .await
            .unwrap();
        let gpt5 = result
            .models
            .iter()
            .find(|m| m.model_id == "gpt-5")
            .unwrap();
        assert!(gpt5.capabilities.vision_input);
        let o3 = result
            .models
            .iter()
            .find(|m| m.model_id == "o3-mini")
            .unwrap();
        assert!(o3.capabilities.reasoning);
    }

    #[tokio::test]
    async fn unauthorized_status_maps_to_typed_error() {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock)
            .await;
        let source = OpenAIV1Models::default();
        let err = source
            .discover(DiscoverParams {
                provider_id: "openai-compat".into(),
                base_url: Some(mock.uri()),
                api_key: Some("bad".into()),
                ..Default::default()
            })
            .await
            .unwrap_err();
        assert!(matches!(err, DiscoveryError::Unauthorized));
    }
}
