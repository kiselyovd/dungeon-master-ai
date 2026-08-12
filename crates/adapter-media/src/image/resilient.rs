//! Primary image generation with a deterministic bundled fallback.

use std::sync::Arc;

use async_trait::async_trait;

use crate::image::provider::{ImageBytes, ImageError, ImagePrompt, ImageProvider, ImageSource};

pub struct ResilientImageProvider {
    primary: Arc<dyn ImageProvider>,
    fallback: Arc<dyn ImageProvider>,
}

impl ResilientImageProvider {
    pub fn new(primary: Arc<dyn ImageProvider>, fallback: Arc<dyn ImageProvider>) -> Self {
        Self { primary, fallback }
    }
}

fn fallback_error_class(error: &ImageError) -> Option<&'static str> {
    match error {
        ImageError::Provider(_) => Some("provider"),
        ImageError::Network(_) => Some("network"),
        ImageError::Timeout { .. } => Some("timeout"),
        ImageError::Degraded { .. } => Some("degraded"),
        ImageError::Auth | ImageError::Cancelled => None,
    }
}

#[async_trait]
impl ImageProvider for ResilientImageProvider {
    async fn generate(&self, prompt: ImagePrompt) -> Result<ImageBytes, ImageError> {
        match self.primary.generate(prompt.clone()).await {
            Ok(image) => Ok(image),
            Err(primary_error) => {
                let Some(error_class) = fallback_error_class(&primary_error) else {
                    return Err(primary_error);
                };
                let image =
                    self.fallback
                        .generate(prompt)
                        .await
                        .map_err(|_| ImageError::Degraded {
                            code: "bundled_fallback_failed",
                        })?;
                let asset_id = match &image.source {
                    ImageSource::Bundled { asset_id } => asset_id.as_str(),
                    ImageSource::Generated => "unexpected-generated-fallback",
                };
                tracing::warn!(
                    error_class,
                    asset_id,
                    "primary image generation failed; bundled fallback selected"
                );
                Ok(image)
            }
        }
    }

    fn estimated_seconds(&self) -> u32 {
        self.primary.estimated_seconds()
    }

    fn cost_per_image(&self) -> f32 {
        self.primary.cost_per_image()
    }
}
