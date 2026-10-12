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
        let primary = self
            .primary
            .generate(prompt.clone())
            .await
            .and_then(|image| {
                if valid_image(&image, &prompt) {
                    Ok(image)
                } else {
                    Err(ImageError::Degraded {
                        code: "primary_image_invalid",
                    })
                }
            });
        match primary {
            Ok(image) => Ok(image),
            Err(primary_error) => {
                let Some(error_class) = fallback_error_class(&primary_error) else {
                    return Err(primary_error);
                };
                let image = self.fallback.generate(prompt.clone()).await.map_err(|_| {
                    ImageError::Degraded {
                        code: "bundled_fallback_failed",
                    }
                })?;
                if !valid_image(&image, &prompt) {
                    return Err(ImageError::Degraded {
                        code: "bundled_fallback_invalid",
                    });
                }
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

fn valid_image(image: &ImageBytes, prompt: &ImagePrompt) -> bool {
    if image.data.len() > 16 * 1024 * 1024 {
        return false;
    }
    let bytes = &image.data;
    let signature = match image.mime_type.as_str() {
        "image/webp" => {
            bytes.len() >= 20
                && bytes.starts_with(b"RIFF")
                && &bytes[8..12] == b"WEBP"
                && u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize + 8
                    == bytes.len()
        }
        "image/png" => {
            bytes.len() >= 24
                && bytes.starts_with(b"\x89PNG\r\n\x1a\n")
                && &bytes[12..16] == b"IHDR"
        }
        "image/jpeg" => {
            bytes.len() >= 4
                && bytes.starts_with(&[0xff, 0xd8, 0xff])
                && bytes.ends_with(&[0xff, 0xd9])
        }
        _ => false,
    };
    if !signature {
        return false;
    }
    match &image.source {
        ImageSource::Generated => true,
        ImageSource::Bundled { asset_id } => super::bundled::bundled_assets().iter().any(|asset| {
            asset.id == asset_id
                && asset.category == super::bundled::category_for_style(&prompt.style_preset)
                && asset.bytes == bytes.as_slice()
        }),
    }
}
