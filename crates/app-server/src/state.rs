use std::sync::Arc;

use app_llm::LlmProvider;

#[derive(Clone)]
pub struct AppState {
    pub llm: Arc<dyn LlmProvider>,
    pub default_model: String,
}
