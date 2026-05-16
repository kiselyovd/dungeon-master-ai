use std::sync::Arc;

use app_llm::LlmProvider;

use crate::image::provider::ImageProvider;

/// Container for the active providers across the three modalities the app
/// supports. Chat is always present (the app cannot function without one);
/// image and video are `Option` because the user can disable either modality
/// in Settings.
///
/// All slots are `Arc<dyn _>` so swaps from `POST /settings` don't tear down
/// `AppState`. The video slot is wired in Task F.3.
pub struct ProviderRegistry {
    pub chat: Arc<dyn LlmProvider>,
    pub image: Option<Arc<dyn ImageProvider>>,
}

impl ProviderRegistry {
    pub fn new(chat: Arc<dyn LlmProvider>) -> Self {
        Self { chat, image: None }
    }

    pub fn with_image(mut self, image: Arc<dyn ImageProvider>) -> Self {
        self.image = Some(image);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_llm::MockProvider;

    #[test]
    fn registry_carries_chat_provider() {
        let chat: Arc<dyn LlmProvider> = Arc::new(MockProvider::new(vec![]));
        let reg = ProviderRegistry::new(chat);
        assert_eq!(reg.chat.name(), "mock");
        assert!(reg.image.is_none());
    }
}
