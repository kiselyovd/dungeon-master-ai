pub mod v2;

pub use v2::{
    BehaviorConfig, ChatConfig, ImageConfig, ImagePreset, ReasoningBudget, SceneTransitions,
    SettingsConfigV2, VideoConfig, VideoMode,
};

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use app_application::settings::{SettingsUpdateError, UpdateSettings};

use crate::error::AppError;
use crate::providers::catalog::{
    find_chat_entry, find_entry_any_modality, IMAGE_CATALOG, VIDEO_CATALOG,
};
#[cfg(test)]
use crate::providers::settings_factory::mistralrs_wire_model;
use crate::providers::settings_factory::{AppStateSettingsCommit, ServerSettingsFactory};
use crate::state::AppState;

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

pub async fn get_providers(State(state): State<AppState>) -> Json<ProvidersInfo> {
    Json(ProvidersInfo {
        available: vec!["openai-compat", "local-mistralrs"],
        active: ActiveProviderInfo {
            kind: state.provider().name().to_string(),
            default_model: state.default_model(),
        },
    })
}

/// M8-DM: v2 settings endpoint. Validates the payload, wires the agent-side
/// fields (tool_availability + behavior knobs) into the live AgentConfig, then
/// builds all three provider slots to completion before acquiring any lock.
/// If any sub-build fails, the prior registry stays untouched (no torn state).
/// On success, the full registry is installed atomically via `swap_registry`.
#[tracing::instrument(skip_all, fields(
    chat_provider = %cfg.chat.active_provider_id,
    image_provider = %cfg.image.active_provider_id,
    video_provider = %cfg.video.active_provider_id,
    license_restricted = cfg.behavior.license_restricted_mode,
))]
pub async fn post_settings_v2(
    State(state): State<AppState>,
    Json(cfg): Json<SettingsConfigV2>,
) -> Result<Json<serde_json::Value>, AppError> {
    validate_settings_v2(&cfg)?;
    let update = UpdateSettings::new(
        Arc::new(ServerSettingsFactory::new(state.clone())),
        state.secrets_repo(),
        Arc::new(AppStateSettingsCommit::new(state)),
    );
    let result = update.execute(cfg).await.map_err(|error| match error {
        SettingsUpdateError::Prepare { .. } => AppError::BadRequest(error.to_string()),
        SettingsUpdateError::Secret { .. } | SettingsUpdateError::Commit { .. } => {
            AppError::Internal(error.to_string())
        }
    })?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "license_restricted_no_compat": result.license_restricted_no_compat,
    })))
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
    // Validate provider ids exist in catalog (any modality for chat is
    // historically tolerant; image/video are tightened to their own catalogs
    // because /settings/v2 dispatches construction off active_provider_id).
    if find_entry_any_modality(&cfg.chat.active_provider_id).is_none() {
        return Err(AppError::BadRequest(format!(
            "unknown chat provider: {}",
            cfg.chat.active_provider_id
        )));
    }
    if !IMAGE_CATALOG
        .iter()
        .any(|e| e.id == cfg.image.active_provider_id)
    {
        return Err(AppError::BadRequest(format!(
            "unknown image provider: {}",
            cfg.image.active_provider_id
        )));
    }
    if !VIDEO_CATALOG
        .iter()
        .any(|e| e.id == cfg.video.active_provider_id)
    {
        return Err(AppError::BadRequest(format!(
            "unknown video provider: {}",
            cfg.video.active_provider_id
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::manifest::{manifest_for, ModelId};
    use serde_json::json;

    fn baseline() -> SettingsConfigV2 {
        serde_json::from_value(json!({
            "chat": {
                "active_provider_id": "openai-compat",
                "active_model_id": "anthropic/claude-haiku",
                "providers": { "openai-compat": { "base_url": "https://openrouter.ai/api/v1", "api_key": "sk-test" } },
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

    #[test]
    fn mistralrs_wire_model_autoisq_uses_hf_repo_not_glob() {
        // Gemma 4 is AutoIsq: hf_filename is the download glob "*". The wire id
        // must be the HF repo (what mistralrs registers + accepts), never "*".
        let m = manifest_for(&ModelId::Gemma4E2bIt).expect("gemma in manifest");
        assert_eq!(m.hf_filename, "*", "guard: AutoIsq filename is the glob");
        assert_eq!(mistralrs_wire_model(m), "google/gemma-4-E2B-it");
    }

    #[test]
    fn mistralrs_wire_model_gguf_uses_filename() {
        let m = manifest_for(&ModelId::Qwen3_5_4b).expect("qwen in manifest");
        assert_eq!(mistralrs_wire_model(m), "Qwen3.5-4B-Q4_K_M.gguf");
    }
}
