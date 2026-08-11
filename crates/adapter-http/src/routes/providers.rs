use axum::extract::{Extension, Path, Query};
use axum::Json;
use serde::Deserialize;
use serde_json::Value;

use crate::services::{HttpServiceError, HttpServices, ProviderDiscoveryCommand};

#[derive(Debug, Deserialize)]
pub struct CapsQuery {
    pub model: String,
}

#[derive(Debug, Deserialize)]
pub struct DiscoverRequest {
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

pub async fn get_catalog(
    Extension(services): Extension<HttpServices>,
) -> Result<Json<Value>, HttpServiceError> {
    services.settings.catalog().await.map(Json)
}

pub async fn get_caps(
    Extension(services): Extension<HttpServices>,
    Path(provider_id): Path<String>,
    Query(query): Query<CapsQuery>,
) -> Result<Json<Value>, HttpServiceError> {
    services
        .settings
        .capabilities(provider_id, query.model)
        .await?
        .map(Json)
        .ok_or(HttpServiceError::NotFound)
}

pub async fn post_discover(
    Extension(services): Extension<HttpServices>,
    Json(request): Json<DiscoverRequest>,
) -> Result<Json<Value>, HttpServiceError> {
    services
        .settings
        .discover(ProviderDiscoveryCommand {
            provider_id: request.provider_id,
            base_url: request.base_url,
            api_key: request.api_key,
            search_query: request.search_query,
            cursor: request.cursor,
        })
        .await
        .map(Json)
}
