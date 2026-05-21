use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use app_domain::srd::retriever::SrdRetriever;
use app_llm::LlmProvider;
use sqlx::SqlitePool;

use crate::agent::orchestrator::AgentConfig;
use crate::local_runtime::{
    probe_real, LocalRuntime, ProbeConfig, ProcessSidecarLauncher, RegistrySnapshot, RuntimeRegistry,
};
use crate::models::DownloadManager;
use crate::routes::local_mode::LocalModeConfig;
use crate::secrets::{InMemorySecretsRepo, SecretsRepo};

/// Shared application state for axum handlers.
///
/// Internally an `Arc<AppStateInner>` so cloning (axum's `State` extractor
/// clones once per request) is cheap. All three provider slots (chat, image,
/// video) are consolidated into a single `registry: RwLock<Arc<ProviderRegistry>>`
/// so that `POST /settings/v2` can install a fully-built registry in one
/// atomic write, eliminating the torn-state window that 3 separate set_*
/// calls would leave behind. In-flight `/chat` streams read-lock just long
/// enough to clone the inner `Arc`, then drop the guard before any `.await`.
#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    registry: RwLock<Arc<crate::providers::ProviderRegistry>>,
    default_model: RwLock<Arc<String>>,
    db: SqlitePool,
    agent_config: RwLock<AgentConfig>,
    srd_retriever: RwLock<Option<Arc<SrdRetriever>>>,
    /// Shared base URL for the Python sidecar that hosts BOTH image and video
    /// backends (single port, single GPU mutex, PipelineDispatcher hot-swaps).
    /// Populated by `runtime_start` after the sidecar spawn succeeds; cleared
    /// by `runtime_stop`. Read by `build_image_provider` /
    /// `build_video_provider` in /settings/v2 to construct LocalImage /
    /// LocalVideo sidecar providers without the frontend having to know the
    /// dynamically-discovered port.
    media_sidecar_url: RwLock<Option<String>>,
    local_mode_config: RwLock<LocalModeConfig>,
    download_manager: Arc<DownloadManager>,
    runtime_registry: Arc<RuntimeRegistry>,
    models_dir: RwLock<PathBuf>,
    secrets_repo: RwLock<Arc<dyn SecretsRepo>>,
}

impl AppState {
    pub fn new(llm: Arc<dyn LlmProvider>, default_model: String, db: SqlitePool) -> Self {
        let models_dir = std::env::temp_dir().join("dmai-models");
        let download_manager = Arc::new(DownloadManager::new(models_dir.clone()));
        let sidecar_launcher = Arc::new(ProcessSidecarLauncher::from_current_exe());
        let probe_cfg = ProbeConfig {
            max_attempts: 8,
            initial_delay: Duration::from_millis(250),
        };
        let llm_runtime = Arc::new(LocalRuntime::new(
            sidecar_launcher.clone(),
            probe_real(probe_cfg),
        ));
        let image_runtime = Arc::new(LocalRuntime::new(
            sidecar_launcher.clone(),
            probe_real(probe_cfg),
        ));
        let runtime_registry = Arc::new(RuntimeRegistry::new(llm_runtime, image_runtime));
        let initial_registry = Arc::new(crate::providers::ProviderRegistry::new(llm));
        Self {
            inner: Arc::new(AppStateInner {
                registry: RwLock::new(initial_registry),
                default_model: RwLock::new(Arc::new(default_model)),
                db,
                agent_config: RwLock::new(AgentConfig::default()),
                srd_retriever: RwLock::new(None),
                media_sidecar_url: RwLock::new(None),
                local_mode_config: RwLock::new(LocalModeConfig::default()),
                download_manager,
                runtime_registry,
                models_dir: RwLock::new(models_dir),
                secrets_repo: RwLock::new(Arc::new(InMemorySecretsRepo::default())),
            }),
        }
    }

    /// Snapshot the current chat provider. The lock is released before the
    /// caller can `.await` on the returned `Arc` - critical, otherwise a
    /// long-running chat stream would block subsequent provider swaps.
    pub fn provider(&self) -> Arc<dyn LlmProvider> {
        self.inner
            .registry
            .read()
            .expect("registry lock poisoned")
            .chat
            .clone()
    }

    pub fn set_provider(&self, llm: Arc<dyn LlmProvider>) {
        let mut guard = self.inner.registry.write().expect("registry lock poisoned");
        let new_reg = crate::providers::ProviderRegistry {
            chat: llm,
            image: guard.image.clone(),
            video: guard.video.clone(),
        };
        *guard = Arc::new(new_reg);
    }

    pub fn default_model(&self) -> String {
        self.inner
            .default_model
            .read()
            .expect("model lock poisoned")
            .as_str()
            .to_string()
    }

    pub fn set_default_model(&self, model: String) {
        *self
            .inner
            .default_model
            .write()
            .expect("model lock poisoned") = Arc::new(model);
    }

    pub fn db(&self) -> &SqlitePool {
        &self.inner.db
    }

    pub fn agent_config(&self) -> AgentConfig {
        self.inner
            .agent_config
            .read()
            .expect("agent config lock poisoned")
            .clone()
    }

    pub fn set_agent_config(&self, config: AgentConfig) {
        *self
            .inner
            .agent_config
            .write()
            .expect("agent config lock poisoned") = config;
    }

    pub fn srd_retriever(&self) -> Option<Arc<SrdRetriever>> {
        self.inner
            .srd_retriever
            .read()
            .expect("srd lock poisoned")
            .clone()
    }

    pub fn set_srd_retriever(&self, retriever: Arc<SrdRetriever>) {
        *self.inner.srd_retriever.write().expect("srd lock poisoned") = Some(retriever);
    }

    pub fn image_provider(&self) -> Option<Arc<dyn crate::image::provider::ImageProvider>> {
        self.inner
            .registry
            .read()
            .expect("registry lock poisoned")
            .image
            .clone()
    }

    pub fn set_image_provider(&self, provider: Arc<dyn crate::image::provider::ImageProvider>) {
        let mut guard = self.inner.registry.write().expect("registry lock poisoned");
        let new_reg = crate::providers::ProviderRegistry {
            chat: guard.chat.clone(),
            image: Some(provider),
            video: guard.video.clone(),
        };
        *guard = Arc::new(new_reg);
    }

    pub fn clear_image_provider(&self) {
        let mut guard = self.inner.registry.write().expect("registry lock poisoned");
        let new_reg = crate::providers::ProviderRegistry {
            chat: guard.chat.clone(),
            image: None,
            video: guard.video.clone(),
        };
        *guard = Arc::new(new_reg);
    }

    pub fn video_provider(&self) -> Option<Arc<dyn crate::video::VideoProvider>> {
        self.inner
            .registry
            .read()
            .expect("registry lock poisoned")
            .video
            .clone()
    }

    pub fn set_video_provider(&self, provider: Arc<dyn crate::video::VideoProvider>) {
        let mut guard = self.inner.registry.write().expect("registry lock poisoned");
        let new_reg = crate::providers::ProviderRegistry {
            chat: guard.chat.clone(),
            image: guard.image.clone(),
            video: Some(provider),
        };
        *guard = Arc::new(new_reg);
    }

    pub fn clear_video_provider(&self) {
        let mut guard = self.inner.registry.write().expect("registry lock poisoned");
        let new_reg = crate::providers::ProviderRegistry {
            chat: guard.chat.clone(),
            image: guard.image.clone(),
            video: None,
        };
        *guard = Arc::new(new_reg);
    }

    /// Atomic registry swap. Used by `POST /settings/v2` to install a brand-new
    /// registry in one write under one lock, avoiding torn state across the
    /// chat/image/video boundary that 3 separate set_* calls would leave behind
    /// (a reader between calls could observe new chat with old image).
    pub fn swap_registry(&self, new_registry: crate::providers::ProviderRegistry) {
        *self.inner.registry.write().expect("registry lock poisoned") = Arc::new(new_registry);
    }

    /// Read-only snapshot of the full registry. Used by tests and future
    /// atomic-read consumers that need a consistent view of all three slots.
    pub fn registry(&self) -> Arc<crate::providers::ProviderRegistry> {
        self.inner
            .registry
            .read()
            .expect("registry lock poisoned")
            .clone()
    }

    pub fn media_sidecar_url(&self) -> Option<String> {
        self.inner
            .media_sidecar_url
            .read()
            .expect("media sidecar url lock poisoned")
            .clone()
    }

    pub fn set_media_sidecar_url(&self, url: Option<String>) {
        *self
            .inner
            .media_sidecar_url
            .write()
            .expect("media sidecar url lock poisoned") = url;
    }

    pub fn local_mode_config(&self) -> LocalModeConfig {
        self.inner
            .local_mode_config
            .read()
            .expect("local mode config lock poisoned")
            .clone()
    }

    pub fn set_local_mode_config(&self, cfg: LocalModeConfig) {
        *self
            .inner
            .local_mode_config
            .write()
            .expect("local mode config lock poisoned") = cfg;
    }

    pub fn download_manager(&self) -> Arc<DownloadManager> {
        self.inner.download_manager.clone()
    }

    pub fn runtime_registry(&self) -> Arc<RuntimeRegistry> {
        self.inner.runtime_registry.clone()
    }

    pub async fn runtime_status(&self) -> RegistrySnapshot {
        self.runtime_registry().status().await
    }

    pub fn models_dir(&self) -> PathBuf {
        self.inner
            .models_dir
            .read()
            .expect("models dir lock poisoned")
            .clone()
    }

    pub fn set_models_dir(&self, dir: PathBuf) {
        *self
            .inner
            .models_dir
            .write()
            .expect("models dir lock poisoned") = dir;
    }

    pub fn secrets_repo(&self) -> Arc<dyn SecretsRepo> {
        self.inner
            .secrets_repo
            .read()
            .expect("secrets lock poisoned")
            .clone()
    }

    pub fn set_secrets_repo(&self, repo: Arc<dyn SecretsRepo>) {
        *self
            .inner
            .secrets_repo
            .write()
            .expect("secrets lock poisoned") = repo;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_llm::MockProvider;
    use async_trait::async_trait;
    use sqlx::SqlitePool;

    use crate::image::provider::{ImageBytes, ImageError, ImagePrompt, ImageProvider};
    use crate::video::provider::{
        VideoCapabilities, VideoError, VideoPrompt, VideoProvider, VideoStream,
    };

    /// Dummy ImageProvider whose `generate` is unused: the registry tests only
    /// need a distinguishable Arc<dyn ImageProvider> identity, never a call.
    struct DummyImageProvider;

    #[async_trait]
    impl ImageProvider for DummyImageProvider {
        async fn generate(&self, _prompt: ImagePrompt) -> Result<ImageBytes, ImageError> {
            unreachable!("DummyImageProvider::generate must not be called in registry tests")
        }
        fn estimated_seconds(&self) -> u32 {
            0
        }
        fn cost_per_image(&self) -> f32 {
            0.0
        }
    }

    /// Dummy VideoProvider whose `generate` is unused for the same reason.
    struct DummyVideoProvider;

    #[async_trait]
    impl VideoProvider for DummyVideoProvider {
        async fn generate(&self, _prompt: VideoPrompt) -> Result<VideoStream, VideoError> {
            unreachable!("DummyVideoProvider::generate must not be called in registry tests")
        }
        fn capabilities(&self) -> VideoCapabilities {
            VideoCapabilities {
                duration_range_secs: (0, 0),
                max_resolution: (0, 0),
                supports_image_init: false,
                avg_seconds_per_clip: 0,
            }
        }
    }

    async fn test_state() -> AppState {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        AppState::new(Arc::new(MockProvider::new(vec![])), "mock".into(), pool)
    }

    #[tokio::test]
    async fn registry_initial_state_has_chat_no_media() {
        let s = test_state().await;
        let reg = s.registry();
        assert_eq!(reg.chat.name(), "mock");
        assert!(reg.image.is_none());
        assert!(reg.video.is_none());
    }

    #[tokio::test]
    async fn set_provider_preserves_image_video() {
        let s = test_state().await;

        // Pre-populate both media slots so we can prove they survive a chat
        // swap (the original test left them None, so a bug that NULLED them
        // would have gone undetected).
        let image: Arc<dyn ImageProvider> = Arc::new(DummyImageProvider);
        let video: Arc<dyn VideoProvider> = Arc::new(DummyVideoProvider);
        s.set_image_provider(image.clone());
        s.set_video_provider(video.clone());

        // Swap chat via the legacy setter with a distinguishable active model.
        let new_chat: Arc<dyn LlmProvider> =
            Arc::new(MockProvider::new(vec![]).with_active_model("post-swap"));
        s.set_provider(new_chat.clone());

        let reg = s.registry();
        // Chat actually moved to the new instance.
        assert!(Arc::ptr_eq(&reg.chat, &new_chat));
        assert_eq!(reg.chat.active_model(), "post-swap");
        // Image and video Arc identities are preserved bit-for-bit.
        let kept_image = reg.image.as_ref().expect("image slot lost");
        let kept_video = reg.video.as_ref().expect("video slot lost");
        assert!(Arc::ptr_eq(kept_image, &image));
        assert!(Arc::ptr_eq(kept_video, &video));
    }

    #[tokio::test]
    async fn swap_registry_replaces_all_three_atomically() {
        let s = test_state().await;
        // Sanity: pre-swap there is no media.
        assert!(s.image_provider().is_none());
        assert!(s.video_provider().is_none());

        // Build a brand-new registry that populates ALL three slots so we can
        // prove the swap installs every slot, not just chat.
        let chat: Arc<dyn LlmProvider> =
            Arc::new(MockProvider::new(vec![]).with_active_model("new-chat"));
        let image: Arc<dyn ImageProvider> = Arc::new(DummyImageProvider);
        let video: Arc<dyn VideoProvider> = Arc::new(DummyVideoProvider);
        let new_reg = crate::providers::ProviderRegistry {
            chat: chat.clone(),
            image: Some(image.clone()),
            video: Some(video.clone()),
        };
        s.swap_registry(new_reg);

        let reg = s.registry();
        assert!(Arc::ptr_eq(&reg.chat, &chat));
        assert_eq!(reg.chat.active_model(), "new-chat");
        let installed_image = reg.image.as_ref().expect("image slot empty after swap");
        let installed_video = reg.video.as_ref().expect("video slot empty after swap");
        assert!(Arc::ptr_eq(installed_image, &image));
        assert!(Arc::ptr_eq(installed_video, &video));
    }
}
