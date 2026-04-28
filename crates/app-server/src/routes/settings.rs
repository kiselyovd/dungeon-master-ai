use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};

use app_llm::{AnthropicProvider, OpenAICompatProvider};

use crate::error::AppError;
use crate::state::AppState;

/// Tagged union of provider configurations the user can pick in Settings.
///
/// The discriminator is `kind`; on the wire we use kebab-case, matching the
/// `LlmProvider::name()` strings each provider returns. New variants slot in
/// here when M4 ships local mistralrs.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ProviderConfig {
    Anthropic {
        api_key: String,
        model: Option<String>,
    },
    OpenaiCompat {
        base_url: String,
        api_key: String,
        model: String,
    },
}

#[derive(Debug, Serialize)]
pub struct ActiveProviderInfo {
    pub kind: String,
    pub default_model: String,
}

#[derive(Debug, Serialize)]
pub struct ProvidersInfo {
    pub available: Vec<&'static str>,
    pub active: ActiveProviderInfo,
}

const DEFAULT_ANTHROPIC_MODEL: &str = "claude-haiku-4-5-20251001";

pub async fn get_providers(State(state): State<AppState>) -> Json<ProvidersInfo> {
    Json(ProvidersInfo {
        available: vec!["anthropic", "openai-compat"],
        active: ActiveProviderInfo {
            kind: state.provider().name().to_string(),
            default_model: state.default_model(),
        },
    })
}

pub async fn post_settings(
    State(state): State<AppState>,
    Json(config): Json<ProviderConfig>,
) -> Result<Json<ActiveProviderInfo>, AppError> {
    match config {
        ProviderConfig::Anthropic { api_key, model } => {
            if api_key.trim().is_empty() {
                return Err(AppError::BadRequest("api_key must not be empty".into()));
            }
            let model = model.unwrap_or_else(|| DEFAULT_ANTHROPIC_MODEL.to_string());
            if model.trim().is_empty() {
                return Err(AppError::BadRequest("model must not be empty".into()));
            }
            state.set_provider(Arc::new(AnthropicProvider::new(api_key)));
            state.set_default_model(model);
        }
        ProviderConfig::OpenaiCompat {
            base_url,
            api_key,
            model,
        } => {
            if base_url.trim().is_empty() {
                return Err(AppError::BadRequest("base_url must not be empty".into()));
            }
            if model.trim().is_empty() {
                return Err(AppError::BadRequest("model must not be empty".into()));
            }
            state.set_provider(Arc::new(OpenAICompatProvider::new(base_url, api_key)));
            state.set_default_model(model);
        }
    }

    Ok(Json(ActiveProviderInfo {
        kind: state.provider().name().to_string(),
        default_model: state.default_model(),
    }))
}
