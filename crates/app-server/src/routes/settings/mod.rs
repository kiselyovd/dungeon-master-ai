pub mod v2;

pub use v2::{
    BehaviorConfig, ChatConfig, ImageConfig, ImagePreset, ReasoningBudget, SceneTransitions,
    SettingsConfigV2, VideoConfig, VideoMode,
};

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};

use app_llm::{AnthropicProvider, MistralrsLocalProvider, OpenAICompatProvider};

use crate::error::AppError;
use crate::image::replicate::ReplicateProvider;
use crate::models::manifest::{lookup, ModelId};
use crate::providers::catalog::{find_chat_entry, find_entry_any_modality};
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
    LocalMistralrs {
        model_id: ModelId,
        port: u16,
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
        available: vec!["anthropic", "openai-compat", "local-mistralrs"],
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
            state
                .secrets_repo()
                .set("anthropic_api_key", &api_key)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
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
            if !api_key.trim().is_empty() {
                state
                    .secrets_repo()
                    .set("openai_compat_api_key", &api_key)
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;
            }
            state.set_provider(Arc::new(OpenAICompatProvider::new(base_url, api_key)));
            state.set_default_model(model);
        }
        ProviderConfig::LocalMistralrs { model_id, port } => {
            let manifest = lookup(&model_id)
                .ok_or_else(|| AppError::BadRequest("unknown model_id".into()))?;
            let model_name = manifest.hf_filename.to_string();
            state.set_provider(Arc::new(MistralrsLocalProvider::new(
                port,
                model_name.clone(),
            )));
            state.set_default_model(model_name);
        }
    }

    Ok(Json(ActiveProviderInfo {
        kind: state.provider().name().to_string(),
        default_model: state.default_model(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct AgentSettingsRequest {
    pub system_prompt: Option<String>,
    pub temperature: Option<f32>,
    pub replicate_api_key: Option<String>,
}

pub async fn post_agent_settings(
    State(state): State<AppState>,
    Json(req): Json<AgentSettingsRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut config = state.agent_config();

    if let Some(sp) = req.system_prompt {
        config.system_prompt = sp;
    }

    if let Some(temp) = req.temperature {
        if !(0.0..=2.0).contains(&temp) {
            return Err(AppError::BadRequest(
                "temperature must be between 0.0 and 2.0".into(),
            ));
        }
        config.temperature = temp;
    }

    state.set_agent_config(config);

    if let Some(key) = req.replicate_api_key {
        if !key.trim().is_empty() {
            state
                .secrets_repo()
                .set("replicate_api_key", &key)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            state.set_image_provider(Arc::new(ReplicateProvider::new(key)));
        }
    }

    Ok(Json(serde_json::json!({ "status": "ok" })))
}

/// M7-DM: v2 settings endpoint. Validates the payload, wires the agent-side
/// fields (tool_availability + behavior knobs) into the live AgentConfig, then
/// reconstructs the chat LlmProvider from `chat.providers[activeProviderId]`
/// (mirrors the legacy `POST /settings` dispatch for the 3 known kinds).
/// Image and video provider rebuild from `image.providers` / `video.providers`
/// is a follow-up M7.5-DM step; for now those modalities keep using whichever
/// provider was set via `POST /agent-settings` (Replicate) or never set at all.
pub async fn post_settings_v2(
    State(state): State<AppState>,
    Json(cfg): Json<SettingsConfigV2>,
) -> Result<Json<serde_json::Value>, AppError> {
    validate_settings_v2(&cfg)?;
    let mut agent_cfg = state.agent_config();
    agent_cfg.tool_availability = crate::agent::tools::ToolAvailability {
        image: cfg.image.enabled,
        video: cfg.video.enabled,
    };
    agent_cfg.system_prompt = cfg.behavior.system_prompt.clone();
    agent_cfg.temperature = cfg.behavior.temperature;
    agent_cfg.max_rounds = cfg.behavior.agent_max_rounds as usize;
    state.set_agent_config(agent_cfg);
    let (provider, model_name) = build_chat_provider(&cfg.chat, state.secrets_repo()).await?;
    state.set_provider(provider);
    state.set_default_model(model_name);
    Ok(Json(serde_json::json!({ "status": "ok" })))
}

/// Per-provider config carried inside `chat.providers[active_provider_id]`.
/// One variant per chat provider known to the catalog. Mirrors the legacy
/// `ProviderConfig` enum but reads from v2 shape: the model name comes from
/// `chat.active_model_id` for cloud providers, while local-mistralrs nests
/// `model_id` (ModelId enum) + `port` here so the manifest lookup works.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AnthropicSlice {
    api_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OpenAICompatSlice {
    base_url: String,
    #[serde(default)]
    api_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalMistralrsSlice {
    model_id: ModelId,
    port: u16,
}

async fn build_chat_provider(
    chat: &ChatConfig,
    secrets: Arc<dyn crate::secrets::SecretsRepo>,
) -> Result<(Arc<dyn app_llm::LlmProvider>, String), AppError> {
    let slice = chat
        .providers
        .get(&chat.active_provider_id)
        .ok_or_else(|| {
            AppError::BadRequest(format!(
                "providers.{} config missing",
                chat.active_provider_id
            ))
        })?;
    match chat.active_provider_id.as_str() {
        "anthropic" => {
            let cfg: AnthropicSlice = serde_json::from_value(slice.clone())
                .map_err(|e| AppError::BadRequest(format!("invalid anthropic slice: {e}")))?;
            if cfg.api_key.trim().is_empty() {
                return Err(AppError::BadRequest("api_key must not be empty".into()));
            }
            if chat.active_model_id.trim().is_empty() {
                return Err(AppError::BadRequest("active_model_id must not be empty".into()));
            }
            secrets
                .set("anthropic_api_key", &cfg.api_key)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            let provider: Arc<dyn app_llm::LlmProvider> =
                Arc::new(AnthropicProvider::new(cfg.api_key));
            Ok((provider, chat.active_model_id.clone()))
        }
        "openai-compat" => {
            let cfg: OpenAICompatSlice = serde_json::from_value(slice.clone())
                .map_err(|e| AppError::BadRequest(format!("invalid openai-compat slice: {e}")))?;
            if cfg.base_url.trim().is_empty() {
                return Err(AppError::BadRequest("base_url must not be empty".into()));
            }
            if chat.active_model_id.trim().is_empty() {
                return Err(AppError::BadRequest("active_model_id must not be empty".into()));
            }
            if !cfg.api_key.trim().is_empty() {
                secrets
                    .set("openai_compat_api_key", &cfg.api_key)
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;
            }
            let provider: Arc<dyn app_llm::LlmProvider> = Arc::new(OpenAICompatProvider::new(
                cfg.base_url,
                cfg.api_key,
            ));
            Ok((provider, chat.active_model_id.clone()))
        }
        "local-mistralrs" => {
            let cfg: LocalMistralrsSlice = serde_json::from_value(slice.clone()).map_err(|e| {
                AppError::BadRequest(format!("invalid local-mistralrs slice: {e}"))
            })?;
            let manifest = lookup(&cfg.model_id)
                .ok_or_else(|| AppError::BadRequest("unknown model_id".into()))?;
            let model_name = manifest.hf_filename.to_string();
            let provider: Arc<dyn app_llm::LlmProvider> = Arc::new(MistralrsLocalProvider::new(
                cfg.port,
                model_name.clone(),
            ));
            Ok((provider, model_name))
        }
        other => Err(AppError::BadRequest(format!(
            "unknown chat provider: {other}"
        ))),
    }
}

pub fn validate_settings_v2(cfg: &SettingsConfigV2) -> Result<(), AppError> {
    // Temperature gate mirrors the legacy /agent-settings range so the v2
    // path can't smuggle in out-of-spec values that the LLM provider rejects.
    if !(0.0..=2.0).contains(&cfg.behavior.temperature) {
        return Err(AppError::BadRequest(
            "temperature must be between 0.0 and 2.0".into(),
        ));
    }
    // Reasoning gate: rejecting requests where the user toggled reasoning on
    // for a model whose curated capabilities say no.
    if cfg.chat.reasoning_enabled {
        if let Some(entry) = find_chat_entry(&cfg.chat.active_provider_id) {
            if let Some(model) = entry
                .curated_models
                .iter()
                .find(|m| m.model_id == cfg.chat.active_model_id)
            {
                if !model.capabilities.reasoning {
                    return Err(AppError::BadRequest(format!(
                        "reasoning not supported by {}/{}",
                        cfg.chat.active_provider_id, cfg.chat.active_model_id
                    )));
                }
            }
        }
    }
    // Vision gate: same pattern.
    if cfg.chat.vision_enabled {
        if let Some(entry) = find_chat_entry(&cfg.chat.active_provider_id) {
            if let Some(model) = entry
                .curated_models
                .iter()
                .find(|m| m.model_id == cfg.chat.active_model_id)
            {
                if !model.capabilities.vision_input {
                    return Err(AppError::BadRequest(format!(
                        "vision_input not supported by {}/{}",
                        cfg.chat.active_provider_id, cfg.chat.active_model_id
                    )));
                }
            }
        }
    }
    // License-restricted mode: block Fast (SAI NC) and Quality (FLUX-dev NC)
    // image presets.
    if cfg.behavior.license_restricted_mode {
        match cfg.image.preset {
            ImagePreset::Fast | ImagePreset::Quality => {
                return Err(AppError::BadRequest(
                    "preset blocked by license_restricted_mode".into(),
                ));
            }
            _ => {}
        }
    }
    // Validate provider ids exist in catalog (any modality).
    if find_entry_any_modality(&cfg.chat.active_provider_id).is_none() {
        return Err(AppError::BadRequest(format!(
            "unknown chat provider: {}",
            cfg.chat.active_provider_id
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn baseline() -> SettingsConfigV2 {
        serde_json::from_value(json!({
            "chat": {
                "active_provider_id": "anthropic",
                "active_model_id": "claude-haiku-4-5-20251001",
                "providers": { "anthropic": { "api_key": "sk-test" } },
                "vision_enabled": false,
                "reasoning_enabled": false,
                "reasoning_budget": "medium",
            },
            "image": {
                "enabled": true,
                "active_provider_id": "local-sdxl-lightning",
                "active_model_id": "sdxl-lightning-4step",
                "providers": {},
                "preset": "balanced",
                "style_lora": null,
            },
            "video": {
                "enabled": false,
                "active_provider_id": "local-ltx-video",
                "active_model_id": "ltx-video-0.9.6-distilled",
                "providers": {},
                "mode": "prerecorded",
            },
            "behavior": {
                "system_prompt": "DM",
                "temperature": 0.7,
                "ui_language": "en",
                "narration_language": "en",
                "license_restricted_mode": false,
                "agent_max_rounds": 8,
                "scene_transitions": "auto",
            },
        }))
        .expect("baseline parse")
    }

    #[test]
    fn validate_baseline_ok() {
        assert!(validate_settings_v2(&baseline()).is_ok());
    }

    #[test]
    fn validate_rejects_quality_preset_when_license_restricted() {
        let mut cfg = baseline();
        cfg.behavior.license_restricted_mode = true;
        cfg.image.preset = ImagePreset::Quality;
        let err = validate_settings_v2(&cfg).unwrap_err();
        match err {
            AppError::BadRequest(msg) => assert!(msg.contains("license_restricted_mode")),
            other => panic!("expected BadRequest, got {other:?}"),
        }
    }

    #[test]
    fn validate_rejects_unknown_chat_provider() {
        let mut cfg = baseline();
        cfg.chat.active_provider_id = "not-a-real-provider".into();
        let err = validate_settings_v2(&cfg).unwrap_err();
        match err {
            AppError::BadRequest(msg) => assert!(msg.contains("unknown chat provider")),
            other => panic!("expected BadRequest, got {other:?}"),
        }
    }

    #[test]
    fn validate_allows_balanced_when_license_restricted() {
        let mut cfg = baseline();
        cfg.behavior.license_restricted_mode = true;
        cfg.image.preset = ImagePreset::Balanced;
        assert!(validate_settings_v2(&cfg).is_ok());
    }
}
