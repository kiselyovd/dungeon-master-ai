//! Retry wrapper for ImageProvider. Mirrors `app_llm::RetryableProvider`.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use backon::{ExponentialBuilder, Retryable};

use crate::image::provider::{ImageBytes, ImageError, ImagePrompt, ImageProvider};

pub fn is_retryable(err: &ImageError) -> bool {
    match err {
        ImageError::Network(_) => true,
        ImageError::Timeout { .. } => true,
        ImageError::Provider(_) => true,
        ImageError::Auth => false,
    }
}

pub fn default_policy() -> ExponentialBuilder {
    ExponentialBuilder::default()
        .with_min_delay(Duration::from_millis(250))
        .with_factor(2.0)
        .with_jitter()
        .with_max_times(3)
}

pub struct RetryableImageProvider {
    inner: Arc<dyn ImageProvider>,
    policy: ExponentialBuilder,
}

impl RetryableImageProvider {
    pub fn new(inner: Arc<dyn ImageProvider>) -> Self {
        Self {
            inner,
            policy: default_policy(),
        }
    }
    pub fn with_policy(inner: Arc<dyn ImageProvider>, policy: ExponentialBuilder) -> Self {
        Self { inner, policy }
    }
}

#[async_trait]
impl ImageProvider for RetryableImageProvider {
    async fn generate(&self, prompt: ImagePrompt) -> Result<ImageBytes, ImageError> {
        let inner = self.inner.clone();
        let op = move || {
            let inner = inner.clone();
            let prompt = prompt.clone();
            async move { inner.generate(prompt).await }
        };
        op.retry(self.policy)
            .when(|e: &ImageError| is_retryable(e))
            .await
    }

    fn estimated_seconds(&self) -> u32 {
        self.inner.estimated_seconds()
    }
    fn cost_per_image(&self) -> f32 {
        self.inner.cost_per_image()
    }
}
