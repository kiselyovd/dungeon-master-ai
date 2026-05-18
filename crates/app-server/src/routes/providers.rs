use axum::Json;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use app_llm::{
    AnthropicProvider, Capabilities, LlmProvider, MistralrsLocalProvider, OpenAICompatProvider,
};

use crate::providers::catalog::{
    CHAT_CATALOG, IMAGE_CATALOG, ProviderCatalogEntry, VIDEO_CATALOG, find_chat_entry,
    find_entry_any_modality,
};
use crate::providers::discovery::{
    AnthropicCurated, DiscoverParams, DiscoveryError, DiscoveryResult, DiscoverySource,
    HfHubSearch, OpenAIV1Models, ReplicateSearch, merge_recommended, recommended_for,
};

#[derive(Serialize)]
pub struct CatalogResponse {
    pub chat: &'static [ProviderCatalogEntry],
    pub image: &'static [ProviderCatalogEntry],
    pub video: &'static [ProviderCatalogEntry],
}

pub async fn get_catalog() -> Json<CatalogResponse> {
    Json(CatalogResponse {
        chat: CHAT_CATALOG,
        image: IMAGE_CATALOG,
        video: VIDEO_CATALOG,
    })
}

#[derive(Deserialize)]
pub struct CapsQuery {
    pub model: String,
}

pub async fn get_caps(
    Path(id): Path<String>,
    Query(q): Query<CapsQuery>,
) -> Result<Json<Capabilities>, StatusCode> {
    let entry = find_entry_any_modality(&id).ok_or(StatusCode::NOT_FOUND)?;
    if let Some(curated) = entry
        .curated_models
        .iter()
        .find(|m| m.model_id == q.model)
    {
        return Ok(Json(curated.capabilities));
    }
    // Fall back to provider-specific inference for chat providers; image/video
    // catalog entries declare capabilities exclusively via curated_models, so
    // a non-curated model id on those modalities returns conservative defaults.
    let caps = match id.as_str() {
        "anthropic" => AnthropicProvider::new(String::new()).capabilities_for_model(&q.model),
        "openai-compat" => {
            OpenAICompatProvider::new(String::new(), String::new()).capabilities_for_model(&q.model)
        }
        "local-mistralrs" => {
            MistralrsLocalProvider::new(0, q.model.clone()).capabilities_for_model(&q.model)
        }
        _ => Capabilities::default(),
    };
    // Silence unused warning when no chat provider matches.
    let _ = find_chat_entry(&id);
    Ok(Json(caps))
}

pub async fn post_discover(
    Json(params): Json<DiscoverParams>,
) -> Result<Json<DiscoveryResult>, StatusCode> {
    let provider_id = params.provider_id.clone();
    let result = dispatch_discovery(&provider_id, params).await;
    match result {
        Ok(r) => Ok(Json(r)),
        Err(DiscoveryError::UnsupportedProvider(_)) => Err(StatusCode::NOT_FOUND),
        Err(DiscoveryError::Unauthorized) => Err(StatusCode::UNAUTHORIZED),
        Err(DiscoveryError::RateLimit) => Err(StatusCode::TOO_MANY_REQUESTS),
        Err(_) => Err(StatusCode::BAD_GATEWAY),
    }
}

async fn dispatch_discovery(
    provider_id: &str,
    params: DiscoverParams,
) -> Result<DiscoveryResult, DiscoveryError> {
    let mut result = match provider_id {
        "anthropic" => AnthropicCurated.discover(params).await?,
        "openai" | "openai-compat" => OpenAIV1Models::default().discover(params).await?,
        "local-mistralrs" => HfHubSearch::default().discover(params).await?,
        "replicate" => ReplicateSearch::default().discover(params).await?,
        unknown => return Err(DiscoveryError::UnsupportedProvider(unknown.to_string())),
    };
    // Prepend per-provider hardcoded Recommended models (deduped by
    // model_id) so the frontend's "Recommended" section always has
    // sensible starter picks for openai-compat and local-mistralrs.
    // Anthropic already returns all Curated so the merge is a no-op
    // for it; Replicate returns empty recommended.
    let recommended = recommended_for(provider_id);
    if !recommended.is_empty() {
        let discovered = std::mem::take(&mut result.models);
        result.models = merge_recommended(recommended, discovered);
    }
    Ok(result)
}
