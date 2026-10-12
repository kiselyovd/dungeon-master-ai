use std::sync::Arc;

use app_application::models::settings::{ChatConfig, ImageConfig, SettingsConfigV2, VideoConfig};
use app_application::settings::{
    PreparedSettings, SecretMutation, SettingsCommit, SettingsFactory,
};
use app_llm::{MistralrsLocalProvider, OpenAICompatProvider};
use async_trait::async_trait;
use serde::Deserialize;

use crate::image::replicate::ReplicateProvider;
use crate::image::stub::LocalImageSidecarProvider;
use crate::license::is_oss_license;
use crate::models::manifest::{manifest_for, ModelId, ModelKind, ModelManifest};
use crate::providers::catalog::{find_chat_entry, IMAGE_CATALOG, VIDEO_CATALOG};
use crate::state::{AppState, PreparedRuntimeSettings};
use crate::video::LocalVideoSidecarProvider;

type PreparedChatProvider = (Arc<dyn app_llm::LlmProvider>, String, Vec<SecretMutation>);
type PreparedImageProvider = (
    Option<Arc<dyn crate::image::provider::ImageProvider>>,
    Vec<SecretMutation>,
);

pub struct ServerSettingsFactory {
    state: AppState,
}

pub struct AppStateSettingsCommit {
    state: AppState,
}

impl AppStateSettingsCommit {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

impl SettingsCommit<PreparedRuntimeSettings> for AppStateSettingsCommit {
    fn commit(&self, snapshot: PreparedRuntimeSettings) -> Result<u64, &'static str> {
        Ok(self.state.commit_runtime_settings(snapshot))
    }
}

impl ServerSettingsFactory {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl SettingsFactory<PreparedRuntimeSettings> for ServerSettingsFactory {
    async fn prepare(
        &self,
        cfg: SettingsConfigV2,
    ) -> Result<PreparedSettings<PreparedRuntimeSettings>, &'static str> {
        let mut agent_cfg = self.state.agent_config();
        agent_cfg.tool_availability = crate::agent::tools::ToolAvailability {
            image: cfg.image.enabled,
            video: cfg.video.enabled,
        };
        agent_cfg.system_prompt = cfg.behavior.system_prompt.clone();
        agent_cfg.temperature = cfg.behavior.temperature;
        agent_cfg.max_rounds = cfg.behavior.agent_max_rounds as usize;
        agent_cfg.reasoning_enabled = cfg.chat.reasoning_enabled;
        agent_cfg.reasoning_budget = match cfg.chat.reasoning_budget {
            app_application::models::settings::ReasoningBudget::Low => app_llm::ReasoningSpec::Low,
            app_application::models::settings::ReasoningBudget::Medium => {
                app_llm::ReasoningSpec::Medium
            }
            app_application::models::settings::ReasoningBudget::High => {
                app_llm::ReasoningSpec::High
            }
        };
        let restricted = cfg.behavior.license_restricted_mode;
        let (mut chat, model, mut mutations) = build_chat_provider(&cfg.chat, restricted)?;
        if cfg.chat.active_provider_id == "local-mistralrs" {
            let control = self.state.runtime_control();
            let status = control
                .status()
                .await
                .map_err(|_| "chat_runtime_unavailable")?;
            if !matches!(
                status.state,
                app_application::models::local_models::RuntimeState::Running
                    | app_application::models::local_models::RuntimeState::Degraded
            ) || status.llm_port.is_none()
            {
                return Err("chat_runtime_unavailable");
            }
            let runtime_id = status.model_id.ok_or("chat_runtime_unavailable")?;
            let slice: LocalMistralrsSlice = serde_json::from_value(
                cfg.chat
                    .providers
                    .get("local-mistralrs")
                    .cloned()
                    .ok_or("chat_provider_config_missing")?,
            )
            .map_err(|_| "chat_provider_config_invalid")?;
            let identity = serde_json::to_string(&slice.model_id)
                .map_err(|_| "chat_provider_config_invalid")?;
            if runtime_model_identity(&self.state, &runtime_id).as_deref()
                != Some(identity.as_str())
            {
                return Err("chat_runtime_model_changed");
            }
            // Native reports the real loaded model and chooses the current port.
            // A stale client port can never redirect the `default` alias.
            chat = Arc::new(
                adapter_llm::native_runtime::NativeRuntimeMistralProvider::new(
                    control,
                    runtime_id,
                    model.clone(),
                    identity,
                ),
            );
            chat.verify_execution_environment()
                .await
                .map_err(|error| match error {
                    app_llm::LlmError::RuntimeChanged => "chat_runtime_model_changed",
                    _ => "chat_runtime_unavailable",
                })?;
        }
        agent_cfg.model = model.clone();
        let auto_unload = matches!(
            self.state.local_mode_config().vram_strategy,
            crate::control_services::local_mode::VramStrategy::AutoSwap
        );
        let (image, image_mutations) = build_image_provider(
            &cfg.image,
            self.state.media_sidecar_url(),
            restricted,
            auto_unload,
        )?;
        mutations.extend(image_mutations);
        let video =
            match build_video_provider(&cfg.video, self.state.media_sidecar_url(), restricted) {
                Ok(provider) => provider,
                Err(code) => {
                    tracing::warn!(code, "video provider unavailable; disabling video");
                    None
                }
            };
        let no_compat = restricted
            && ((cfg.image.enabled && image.is_none()) || (cfg.video.enabled && video.is_none()));
        Ok(PreparedSettings {
            snapshot: PreparedRuntimeSettings {
                registry: crate::providers::ProviderRegistry { chat, image, video },
                default_model: model,
                agent_config: agent_cfg,
            },
            secret_mutations: mutations,
            license_restricted_no_compat: no_compat,
        })
    }
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

fn build_chat_provider(
    chat: &ChatConfig,
    restricted: bool,
) -> Result<PreparedChatProvider, &'static str> {
    if restricted
        && find_chat_entry(&chat.active_provider_id)
            .is_some_and(|entry| !is_oss_license(entry.license))
    {
        return Err("chat_license_restricted");
    }
    let slice = chat
        .providers
        .get(&chat.active_provider_id)
        .ok_or("chat_provider_config_missing")?;
    match chat.active_provider_id.as_str() {
        "openai-compat" => {
            let cfg: OpenAICompatSlice = serde_json::from_value(slice.clone())
                .map_err(|_| "chat_provider_config_invalid")?;
            if cfg.base_url.trim().is_empty() || chat.active_model_id.trim().is_empty() {
                return Err("chat_provider_config_invalid");
            }
            let mut mutations = vec![SecretMutation::Delete {
                key: "anthropic_api_key".into(),
            }];
            if !cfg.api_key.trim().is_empty() {
                mutations.push(SecretMutation::Set {
                    key: "openai_compat_api_key".into(),
                    value: cfg.api_key.clone(),
                });
            }
            let raw: Arc<dyn app_llm::LlmProvider> =
                Arc::new(OpenAICompatProvider::new(cfg.base_url, cfg.api_key));
            Ok((
                Arc::new(app_llm::RetryableProvider::new(raw)),
                chat.active_model_id.clone(),
                mutations,
            ))
        }
        "local-mistralrs" => {
            let cfg: LocalMistralrsSlice = serde_json::from_value(slice.clone())
                .map_err(|_| "chat_provider_config_invalid")?;
            let manifest = manifest_for(&cfg.model_id).ok_or("chat_model_unknown")?;
            let model = mistralrs_wire_model(manifest);
            Ok((
                Arc::new(MistralrsLocalProvider::new(cfg.port, model.clone())),
                model,
                Vec::new(),
            ))
        }
        _ => Err("chat_provider_unknown"),
    }
}

pub(crate) fn mistralrs_wire_model(manifest: &ModelManifest) -> String {
    match manifest.kind {
        ModelKind::AutoIsq { .. } => manifest.hf_repo.to_string(),
        _ => manifest.hf_filename.to_string(),
    }
}

fn runtime_model_identity(state: &AppState, id: &str) -> Option<String> {
    let parsed = serde_json::from_value::<ModelId>(serde_json::Value::String(id.into()))
        .ok()
        .or_else(|| serde_json::from_str::<ModelId>(id).ok())
        .or_else(|| crate::control_services::local_llm::model_id_for_wire(id));
    if let Some(parsed) = parsed {
        return serde_json::to_string(&parsed).ok();
    }
    let file =
        crate::hf::manifest::load_or_init(&crate::control_services::hf::user_manifest_path(state))
            .ok()?;
    let entry = file
        .entries
        .into_iter()
        .find(|entry| entry.system.id == id)?;
    if entry.system.hf_filename.is_empty() {
        return None;
    }
    serde_json::to_string(&ModelId::Custom {
        hf_repo: entry.system.hf_repo,
        gguf_filename: entry.system.hf_filename,
        mmproj_filename: None,
    })
    .ok()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplicateImageSlice {
    api_key: String,
}

fn build_image_provider(
    image: &ImageConfig,
    sidecar_url: Option<String>,
    restricted: bool,
    auto_unload: bool,
) -> Result<PreparedImageProvider, &'static str> {
    if !image.enabled {
        return Ok((None, Vec::new()));
    }
    if restricted
        && IMAGE_CATALOG
            .iter()
            .find(|entry| entry.id == image.active_provider_id)
            .is_some_and(|entry| !is_oss_license(entry.license))
    {
        return Ok((Some(crate::image::bundled_image_provider()), Vec::new()));
    }
    match image.active_provider_id.as_str() {
        "replicate" => {
            let slice = image
                .providers
                .get("replicate")
                .ok_or("image_provider_config_missing")?;
            let cfg: ReplicateImageSlice = serde_json::from_value(slice.clone())
                .map_err(|_| "image_provider_config_invalid")?;
            if cfg.api_key.trim().is_empty() {
                return Err("image_api_key_missing");
            }
            let mutation = SecretMutation::Set {
                key: "replicate_api_key".into(),
                value: cfg.api_key.clone(),
            };
            let raw: Arc<dyn crate::image::provider::ImageProvider> =
                Arc::new(ReplicateProvider::new(cfg.api_key));
            let primary: Arc<dyn crate::image::provider::ImageProvider> =
                Arc::new(crate::image::RetryableImageProvider::new(raw));
            Ok((
                Some(Arc::new(crate::image::ResilientImageProvider::new(
                    primary,
                    crate::image::bundled_image_provider(),
                ))),
                vec![mutation],
            ))
        }
        id if id.starts_with("local-") => {
            let Some(url) = sidecar_url else {
                return Ok((Some(crate::image::bundled_image_provider()), Vec::new()));
            };
            let primary: Arc<dyn crate::image::provider::ImageProvider> =
                Arc::new(LocalImageSidecarProvider::new(url).with_auto_unload(auto_unload));
            Ok((
                Some(Arc::new(crate::image::ResilientImageProvider::new(
                    primary,
                    crate::image::bundled_image_provider(),
                ))),
                Vec::new(),
            ))
        }
        _ => Err("image_provider_unknown"),
    }
}

fn build_video_provider(
    video: &VideoConfig,
    sidecar_url: Option<String>,
    restricted: bool,
) -> Result<Option<Arc<dyn crate::video::VideoProvider>>, &'static str> {
    if !video.enabled {
        return Ok(None);
    }
    if restricted
        && VIDEO_CATALOG
            .iter()
            .find(|entry| entry.id == video.active_provider_id)
            .is_some_and(|entry| !is_oss_license(entry.license))
    {
        return Ok(None);
    }
    match video.active_provider_id.as_str() {
        id if id.starts_with("local-") => Ok(Some(Arc::new(LocalVideoSidecarProvider::new(
            sidecar_url.ok_or("media_sidecar_unavailable")?,
        )))),
        _ => Err("video_provider_unknown"),
    }
}
