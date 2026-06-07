pub mod v2;

pub use v2::{
    BehaviorConfig, ChatConfig, ImageConfig, ImagePreset, ReasoningBudget, SceneTransitions,
    SettingsConfigV2, VideoConfig, VideoMode,
};

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

use app_llm::{MistralrsLocalProvider, OpenAICompatProvider};

use crate::error::AppError;
use crate::image::replicate::ReplicateProvider;
use crate::image::stub::LocalImageSidecarProvider;
use crate::license::is_oss_license;
use crate::models::manifest::{manifest_for, ModelId, ModelKind, ModelManifest};
use crate::providers::catalog::{
    find_chat_entry, find_entry_any_modality, IMAGE_CATALOG, VIDEO_CATALOG,
};
use crate::state::AppState;
use crate::video::LocalVideoSidecarProvider;

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
    let mut agent_cfg = state.agent_config();
    agent_cfg.tool_availability = crate::agent::tools::ToolAvailability {
        image: cfg.image.enabled,
        video: cfg.video.enabled,
    };
    agent_cfg.system_prompt = cfg.behavior.system_prompt.clone();
    agent_cfg.temperature = cfg.behavior.temperature;
    agent_cfg.max_rounds = cfg.behavior.agent_max_rounds as usize;
    agent_cfg.reasoning_enabled = cfg.chat.reasoning_enabled;
    agent_cfg.reasoning_budget = match cfg.chat.reasoning_budget {
        crate::routes::settings::v2::ReasoningBudget::Low => app_llm::ReasoningSpec::Low,
        crate::routes::settings::v2::ReasoningBudget::Medium => app_llm::ReasoningSpec::Medium,
        crate::routes::settings::v2::ReasoningBudget::High => app_llm::ReasoningSpec::High,
    };

    let license_restricted = cfg.behavior.license_restricted_mode;

    // Build the full registry to completion BEFORE taking the write lock.
    // If any sub-build fails, the prior registry stays untouched.
    let (chat_provider, model_name) =
        build_chat_provider(&cfg.chat, state.secrets_repo(), license_restricted).await?;
    // The orchestrator sends `agent_config().model` verbatim as the chat request
    // model. It must track the resolved active model, not the stale default
    // ("claude-haiku-4-5-20251001"): for the local runtime, mistralrs 404s on any
    // id it did not load, so a stale/wrong model silently kills every DM turn.
    agent_cfg.model = model_name.clone();
    // Image/video are OPTIONAL enhancements: a build failure (e.g. the media
    // sidecar is still starting) must NOT block the mandatory chat provider from
    // applying. Degrade gracefully by disabling that modality and logging, so
    // saving a working chat provider never 400s on an unavailable image runtime.
    // AutoSwap VRAM strategy: have the local image provider unload SDXL after
    // each generation so the LLM runtime regains VRAM (both resident max a 10 GB
    // card and the LLM decode thrashes). KeepBothLoaded keeps it resident.
    let image_auto_unload = matches!(
        state.local_mode_config().vram_strategy,
        crate::routes::local_mode::VramStrategy::AutoSwap
    );
    let image_provider = match build_image_provider(
        &cfg.image,
        state.secrets_repo(),
        state.media_sidecar_url(),
        license_restricted,
        image_auto_unload,
    )
    .await
    {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "image provider unavailable; disabling image for this config");
            None
        }
    };
    let video_provider = match build_video_provider(
        &cfg.video,
        state.media_sidecar_url(),
        license_restricted,
    ) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "video provider unavailable; disabling video for this config");
            None
        }
    };

    // license_restricted_no_compat: surfaces to UI when the user toggled
    // restriction on but their active image/video provider got filtered out.
    // Lets the frontend show "no compatible OSS providers for this modality".
    let no_compat = license_restricted
        && ((cfg.image.enabled && image_provider.is_none())
            || (cfg.video.enabled && video_provider.is_none()));

    let new_registry = crate::providers::ProviderRegistry {
        chat: chat_provider,
        image: image_provider,
        video: video_provider,
    };

    // All builds succeeded - now persist agent config + atomically swap registry.
    state.set_agent_config(agent_cfg);
    state.swap_registry(new_registry);
    state.set_default_model(model_name);

    Ok(Json(serde_json::json!({
        "status": "ok",
        "license_restricted_no_compat": no_compat,
    })))
}

/// Per-provider config carried inside `chat.providers[active_provider_id]`.
/// One variant per chat provider known to the catalog. Mirrors the legacy
/// `ProviderConfig` enum but reads from v2 shape: the model name comes from
/// `chat.active_model_id` for cloud providers, while local-mistralrs nests
/// `model_id` (ModelId enum) + `port` here so the manifest lookup works.
///
/// Cloud chat is served exclusively by the generic OpenAI-compatible provider
/// (OpenRouter recommended as base_url); native Anthropic was removed in M11
/// Batch D.5.
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
    license_restricted: bool,
) -> Result<(Arc<dyn app_llm::LlmProvider>, String), AppError> {
    // License-restricted mode: chat is mandatory, so a non-OSS active chat
    // provider is a hard error (not a silent filter as for image/video).
    if license_restricted {
        if let Some(entry) = find_chat_entry(&chat.active_provider_id) {
            if !is_oss_license(entry.license) {
                return Err(AppError::BadRequest(format!(
                    "chat provider '{}' blocked by license_restricted_mode (license: {})",
                    chat.active_provider_id, entry.license
                )));
            }
        }
    }
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
        "openai-compat" => {
            let cfg: OpenAICompatSlice = serde_json::from_value(slice.clone())
                .map_err(|e| AppError::BadRequest(format!("invalid openai-compat slice: {e}")))?;
            if cfg.base_url.trim().is_empty() {
                return Err(AppError::BadRequest("base_url must not be empty".into()));
            }
            if chat.active_model_id.trim().is_empty() {
                return Err(AppError::BadRequest(
                    "active_model_id must not be empty".into(),
                ));
            }
            // Native Anthropic was removed in M11 Batch D.5; purge any legacy
            // key so a stale secret can't linger after the cloud reconfigure.
            let _ = secrets.delete("anthropic_api_key").await;
            if !cfg.api_key.trim().is_empty() {
                secrets
                    .set("openai_compat_api_key", &cfg.api_key)
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;
            }
            let raw: Arc<dyn app_llm::LlmProvider> =
                Arc::new(OpenAICompatProvider::new(cfg.base_url, cfg.api_key));
            let provider: Arc<dyn app_llm::LlmProvider> =
                Arc::new(app_llm::RetryableProvider::new(raw));
            Ok((provider, chat.active_model_id.clone()))
        }
        "local-mistralrs" => {
            let cfg: LocalMistralrsSlice = serde_json::from_value(slice.clone())
                .map_err(|e| AppError::BadRequest(format!("invalid local-mistralrs slice: {e}")))?;
            let manifest = manifest_for(&cfg.model_id)
                .ok_or_else(|| AppError::BadRequest("unknown model_id".into()))?;
            let model_name = mistralrs_wire_model(manifest);
            let provider: Arc<dyn app_llm::LlmProvider> =
                Arc::new(MistralrsLocalProvider::new(cfg.port, model_name.clone()));
            Ok((provider, model_name))
        }
        other => Err(AppError::BadRequest(format!(
            "unknown chat provider: {other}"
        ))),
    }
}

/// Resolve the model id to send to a running mistralrs server's OpenAI-compat
/// endpoint. AutoIsq (safetensors auto-loader) models register under their HF
/// repo id (e.g. "google/gemma-4-E2B-it"); their manifest `hf_filename` is the
/// download glob "*", which mistralrs rejects at request time ("Requested model
/// '*' is not available. Use 'default'..."). GGUF models, launched from a local
/// file, register under that on-disk filename, so the filename is the right id.
fn mistralrs_wire_model(manifest: &ModelManifest) -> String {
    match manifest.kind {
        ModelKind::AutoIsq { .. } => manifest.hf_repo.to_string(),
        _ => manifest.hf_filename.to_string(),
    }
}

/// Cloud (Replicate) needs an api_key; the 4 local presets need only the
/// shared media sidecar URL (PipelineDispatcher routes by `preset` field).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplicateImageSlice {
    api_key: String,
}

async fn build_image_provider(
    image: &ImageConfig,
    secrets: Arc<dyn crate::secrets::SecretsRepo>,
    sidecar_url: Option<String>,
    license_restricted: bool,
    auto_unload: bool,
) -> Result<Option<Arc<dyn crate::image::provider::ImageProvider>>, AppError> {
    if !image.enabled {
        return Ok(None);
    }
    // License-restricted: filter non-OSS image providers silently. The frontend
    // surfaces this via the `license_restricted_no_compat` response field.
    if license_restricted {
        if let Some(entry) = IMAGE_CATALOG
            .iter()
            .find(|e| e.id == image.active_provider_id)
        {
            if !is_oss_license(entry.license) {
                return Ok(None);
            }
        }
    }
    match image.active_provider_id.as_str() {
        "replicate" => {
            let slice = image
                .providers
                .get("replicate")
                .ok_or_else(|| AppError::BadRequest("providers.replicate config missing".into()))?;
            let cfg: ReplicateImageSlice = serde_json::from_value(slice.clone())
                .map_err(|e| AppError::BadRequest(format!("invalid replicate slice: {e}")))?;
            if cfg.api_key.trim().is_empty() {
                return Err(AppError::BadRequest(
                    "replicate api_key must not be empty".into(),
                ));
            }
            secrets
                .set("replicate_api_key", &cfg.api_key)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            let raw: Arc<dyn crate::image::provider::ImageProvider> =
                Arc::new(ReplicateProvider::new(cfg.api_key));
            Ok(Some(Arc::new(crate::image::RetryableImageProvider::new(
                raw,
            ))))
        }
        id if id.starts_with("local-") => {
            let url = sidecar_url.ok_or_else(|| {
                AppError::BadRequest(
                    "media sidecar not running; start it via /local/runtime/start first".into(),
                )
            })?;
            Ok(Some(Arc::new(
                LocalImageSidecarProvider::new(url).with_auto_unload(auto_unload),
            )))
        }
        other => Err(AppError::BadRequest(format!(
            "unknown image provider: {other}"
        ))),
    }
}

fn build_video_provider(
    video: &VideoConfig,
    sidecar_url: Option<String>,
    license_restricted: bool,
) -> Result<Option<Arc<dyn crate::video::VideoProvider>>, AppError> {
    if !video.enabled {
        return Ok(None);
    }
    // License-restricted: filter non-OSS video providers silently. LTX-Video
    // is OpenRAIL-M (research-only), so it gets dropped in restricted mode.
    if license_restricted {
        if let Some(entry) = VIDEO_CATALOG
            .iter()
            .find(|e| e.id == video.active_provider_id)
        {
            if !is_oss_license(entry.license) {
                return Ok(None);
            }
        }
    }
    match video.active_provider_id.as_str() {
        id if id.starts_with("local-") => {
            let url = sidecar_url.ok_or_else(|| {
                AppError::BadRequest(
                    "media sidecar not running; start it via /local/runtime/start first".into(),
                )
            })?;
            Ok(Some(Arc::new(LocalVideoSidecarProvider::new(url))))
        }
        other => Err(AppError::BadRequest(format!(
            "unknown video provider: {other}"
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
