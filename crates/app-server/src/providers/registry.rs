use std::sync::Arc;

use app_llm::LlmProvider;

use crate::image::provider::ImageProvider;
use crate::video::provider::VideoProvider;

/// Container for the active providers across the three modalities the app
/// supports. Chat is always present (the app cannot function without one);
/// image and video are `Option` because the user can disable either modality
/// in Settings.
///
/// All slots are `Arc<dyn _>` so swaps from `POST /settings` don't tear down
/// `AppState`.
pub struct ProviderRegistry {
    pub chat: Arc<dyn LlmProvider>,
    pub image: Option<Arc<dyn ImageProvider>>,
    pub video: Option<Arc<dyn VideoProvider>>,
}

impl ProviderRegistry {
    pub fn new(chat: Arc<dyn LlmProvider>) -> Self {
        Self {
            chat,
            image: None,
            video: None,
        }
    }

    pub fn with_image(mut self, image: Arc<dyn ImageProvider>) -> Self {
        self.image = Some(image);
        self
    }

    pub fn with_video(mut self, video: Arc<dyn VideoProvider>) -> Self {
        self.video = Some(video);
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

    struct TestVideoProvider;

    #[async_trait::async_trait]
    impl crate::video::provider::VideoProvider for TestVideoProvider {
        async fn generate(
            &self,
            _prompt: crate::video::provider::VideoPrompt,
        ) -> Result<crate::video::provider::VideoStream, crate::video::provider::VideoError>
        {
            Err(crate::video::provider::VideoError::Provider("test".into()))
        }

        fn capabilities(&self) -> crate::video::provider::VideoCapabilities {
            crate::video::provider::VideoCapabilities {
                duration_range_secs: (1, 1),
                max_resolution: (1, 1),
                supports_image_init: false,
                avg_seconds_per_clip: 1,
            }
        }
    }

    #[test]
    fn registry_carries_video_provider() {
        let chat: Arc<dyn LlmProvider> = Arc::new(MockProvider::new(vec![]));
        let video: Arc<dyn crate::video::provider::VideoProvider> = Arc::new(TestVideoProvider);
        let reg = ProviderRegistry::new(chat).with_video(video);
        assert!(reg.video.is_some());
        assert_eq!(
            reg.video
                .as_ref()
                .unwrap()
                .capabilities()
                .avg_seconds_per_clip,
            1
        );
    }

    #[test]
    fn registry_new_has_no_video() {
        let chat: Arc<dyn LlmProvider> = Arc::new(MockProvider::new(vec![]));
        let reg = ProviderRegistry::new(chat);
        assert!(reg.video.is_none());
    }
}
