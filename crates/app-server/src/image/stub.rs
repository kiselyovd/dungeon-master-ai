//! Local image sidecar provider.
//!
//! HTTP-loopback adapter to the Python FastAPI sidecar (see `sidecar/app.py`).
//! The sidecar accepts JSON `{prompt, seed, steps, backend, ...}` and returns
//! `{image_b64|video_b64, mime}`. We decode base64 here and surface raw bytes
//! through the shared `ImageProvider` trait so callers (the agent's
//! `generate_image` tool, M4.5 multimodal flows) do not need to know whether
//! the bytes came from a cloud Replicate response or a local Python process.
//!
//! The `backend` field selects which dispatcher slot handles the request:
//! fast (SDXL-Turbo), balanced (SDXL-Lightning), quality (Nunchaku-FLUX),
//! quality-oss (Z-Image-Turbo), or ltx-video.

use std::time::Duration;

use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;

use crate::image::provider::{ImageBytes, ImageError, ImagePrompt, ImageProvider};

const REQUEST_TIMEOUT_SECS: u64 = 120;
const DEFAULT_STEPS: u32 = 4;

pub struct LocalImageSidecarProvider {
    base_url: String,
    client: reqwest::Client,
}

impl LocalImageSidecarProvider {
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .expect("reqwest client");
        Self {
            base_url: base_url.into(),
            client,
        }
    }
}

#[async_trait]
impl ImageProvider for LocalImageSidecarProvider {
    async fn generate(&self, prompt: ImagePrompt) -> Result<ImageBytes, ImageError> {
        // backend_preset is propagated to the Python dispatcher so it picks the
        // right backend slot (fast=SDXL-Turbo, balanced=SDXL-Lightning,
        // quality=Nunchaku-FLUX, quality-oss=Z-Image-Turbo, ltx-video).
        // Falling back to 'fast' keeps M4/M5 behavior when the field is absent.
        let backend = prompt
            .backend_preset
            .clone()
            .unwrap_or_else(|| "fast".to_string());
        let body = serde_json::json!({
            "prompt": prompt.content_prompt,
            "seed": 0,
            "steps": DEFAULT_STEPS,
            "backend": backend,
        });
        let resp = self
            .client
            .post(format!("{}/generate", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| ImageError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(ImageError::Provider(format!(
                "sidecar returned {}",
                resp.status()
            )));
        }

        let payload: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ImageError::Provider(e.to_string()))?;
        let b64 = payload
            .get("image_b64")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ImageError::Provider("response missing image_b64".into()))?;
        let mime = payload
            .get("mime")
            .and_then(|v| v.as_str())
            .unwrap_or("image/png")
            .to_string();
        let data = B64
            .decode(b64)
            .map_err(|e| ImageError::Provider(format!("base64 decode: {e}")))?;
        Ok(ImageBytes {
            data,
            mime_type: mime,
        })
    }

    fn estimated_seconds(&self) -> u32 {
        8
    }

    fn cost_per_image(&self) -> f32 {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD as B64;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn generates_image_via_sidecar_http() {
        let server = MockServer::start().await;
        let png = b"PNG-bytes";
        let b64 = B64.encode(png);
        Mock::given(method("POST"))
            .and(path("/generate"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"image_b64": b64, "mime": "image/png"})),
            )
            .mount(&server)
            .await;

        let provider = LocalImageSidecarProvider::new(server.uri());
        let result = provider
            .generate(ImagePrompt {
                content_prompt: "a tavern at dusk".into(),
                style_preset: "dark_fantasy".into(),
                scene_id: Some("scene-1".into()),
                npc_ids: vec![],
                backend_preset: Some("balanced".into()),
            })
            .await
            .unwrap();
        assert_eq!(result.data, png);
        assert_eq!(result.mime_type, "image/png");
    }

    #[tokio::test]
    async fn surfaces_5xx_as_provider_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/generate"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;
        let provider = LocalImageSidecarProvider::new(server.uri());
        let result = provider
            .generate(ImagePrompt {
                content_prompt: "x".into(),
                style_preset: "dark_fantasy".into(),
                scene_id: None,
                npc_ids: vec![],
                backend_preset: None,
            })
            .await;
        assert!(matches!(result, Err(ImageError::Provider(_))));
    }
}
