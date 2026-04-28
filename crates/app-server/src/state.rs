use std::sync::{Arc, RwLock};

use app_llm::LlmProvider;

/// Shared application state for axum handlers.
///
/// Internally an `Arc<AppStateInner>` so cloning (axum's `State` extractor
/// clones once per request) is cheap. The provider and default-model fields
/// sit behind `RwLock<Arc<...>>` so the `POST /settings` endpoint can swap
/// them without locking out in-flight `/chat` streams: we read-lock just
/// long enough to clone the inner `Arc`, then drop the guard before any
/// `.await`.
#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    llm: RwLock<Arc<dyn LlmProvider>>,
    default_model: RwLock<Arc<String>>,
}

impl AppState {
    pub fn new(llm: Arc<dyn LlmProvider>, default_model: String) -> Self {
        Self {
            inner: Arc::new(AppStateInner {
                llm: RwLock::new(llm),
                default_model: RwLock::new(Arc::new(default_model)),
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
        *self
            .inner
            .llm
            .write()
            .expect("provider lock poisoned") = llm;
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
}
