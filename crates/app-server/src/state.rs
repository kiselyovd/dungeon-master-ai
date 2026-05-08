use std::sync::{Arc, RwLock};

use app_domain::srd::retriever::SrdRetriever;
use app_llm::LlmProvider;
use sqlx::SqlitePool;

use crate::agent::orchestrator::AgentConfig;

/// Shared application state for axum handlers.
///
/// Internally an `Arc<AppStateInner>` so cloning (axum's `State` extractor
/// clones once per request) is cheap. The provider, default-model, and
/// image-provider fields sit behind `RwLock<Arc<...>>` so the `POST /settings`
/// and `POST /agent-settings` endpoints can swap them without locking out
/// in-flight `/chat` streams: we read-lock just long enough to clone the
/// inner `Arc`, then drop the guard before any `.await`.
#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    llm: RwLock<Arc<dyn LlmProvider>>,
    default_model: RwLock<Arc<String>>,
    db: SqlitePool,
    agent_config: RwLock<AgentConfig>,
    srd_retriever: RwLock<Option<Arc<SrdRetriever>>>,
    image_provider: RwLock<Option<Arc<dyn crate::image::provider::ImageProvider>>>,
}

impl AppState {
    pub fn new(llm: Arc<dyn LlmProvider>, default_model: String, db: SqlitePool) -> Self {
        Self {
            inner: Arc::new(AppStateInner {
                llm: RwLock::new(llm),
                default_model: RwLock::new(Arc::new(default_model)),
                db,
                agent_config: RwLock::new(AgentConfig::default()),
                srd_retriever: RwLock::new(None),
                image_provider: RwLock::new(None),
            }),
        }
    }

    /// Snapshot the current provider. The lock is released before the caller
    /// can `.await` on the returned `Arc` - critical, otherwise a long-running
    /// chat stream would block subsequent provider swaps.
    pub fn provider(&self) -> Arc<dyn LlmProvider> {
        self.inner
            .llm
            .read()
            .expect("provider lock poisoned")
            .clone()
    }

    pub fn set_provider(&self, llm: Arc<dyn LlmProvider>) {
        *self.inner.llm.write().expect("provider lock poisoned") = llm;
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
        *self
            .inner
            .srd_retriever
            .write()
            .expect("srd lock poisoned") = Some(retriever);
    }

    pub fn image_provider(&self) -> Option<Arc<dyn crate::image::provider::ImageProvider>> {
        self.inner
            .image_provider
            .read()
            .expect("image provider lock poisoned")
            .clone()
    }

    pub fn set_image_provider(&self, provider: Arc<dyn crate::image::provider::ImageProvider>) {
        *self
            .inner
            .image_provider
            .write()
            .expect("image provider lock poisoned") = Some(provider);
    }
}
