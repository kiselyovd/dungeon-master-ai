//! Replicate cloud image generation provider.
//!
//! Hand-rolled reqwest calls (no Replicate SDK exists for Rust).
//! Flow: POST /v1/predictions -> poll GET /v1/predictions/{id} until completed.
//! Default model: stability-ai/sdxl@7762fd07... (SDXL 1.0).

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

use crate::image::provider::{ImageBytes, ImageError, ImagePrompt, ImageProvider};

const REPLICATE_API_BASE: &str = "https://api.replicate.com/v1";
const SDXL_MODEL: &str = "stability-ai/sdxl:7762fd07cf82c948538e41f63f77d685e02b063e37e496ce96467a993d0b73da";
const STYLE_PREAMBLE: &str = "dark fantasy oil painting, dramatic lighting, atmospheric, cinematic, Witcher 3 inspired, highly detailed";
const MAX_POLL_SECS: u64 = 90;
const POLL_INTERVAL_MS: u64 = 2500;

pub struct ReplicateProvider {
    api_key: String,
    client: Client,
}

impl ReplicateProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("reqwest client"),
        }
    }

    fn full_prompt(&self, prompt: &ImagePrompt) -> String {
        format!("{}, {}", STYLE_PREAMBLE, prompt.content_prompt)
    }
}

#[derive(Serialize)]
struct PredictionInput {
    prompt: String,
    width: u32,
    height: u32,
    num_outputs: u8,
}

#[derive(Serialize)]
struct CreatePredictionRequest {
    version: &'static str,
    input: PredictionInput,
}

#[derive(Deserialize)]
struct PredictionResponse {
    id: String,
    status: String,
    output: Option<Vec<String>>,
    error: Option<String>,
}

#[async_trait]
impl ImageProvider for ReplicateProvider {
    async fn generate(&self, prompt: ImagePrompt) -> Result<ImageBytes, ImageError> {
        let full_prompt = self.full_prompt(&prompt);
        info!("Replicate: generating image for prompt: {}", full_prompt);

        let body = CreatePredictionRequest {
            version: SDXL_MODEL,
            input: PredictionInput {
                prompt: full_prompt,
                width: 1024,
                height: 576,
                num_outputs: 1,
            },
        };

        let create_resp = self.client
            .post(format!("{REPLICATE_API_BASE}/predictions"))
            .header("Authorization", format!("Token {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ImageError::Network(e.to_string()))?;

        if !create_resp.status().is_success() {
            if create_resp.status().as_u16() == 401 {
                return Err(ImageError::Auth);
            }
            return Err(ImageError::Provider(format!(
                "prediction create failed: HTTP {}",
                create_resp.status()
            )));
        }

        let prediction: PredictionResponse = create_resp
            .json()
            .await
            .map_err(|e| ImageError::Provider(e.to_string()))?;

        let prediction_id = prediction.id;
        info!("Replicate: prediction {prediction_id} created, polling...");

        let deadline = std::time::Instant::now() + Duration::from_secs(MAX_POLL_SECS);
        loop {
            if std::time::Instant::now() >= deadline {
                return Err(ImageError::Timeout { secs: MAX_POLL_SECS });
            }
            sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;

            let poll_resp = self.client
                .get(format!("{REPLICATE_API_BASE}/predictions/{prediction_id}"))
                .header("Authorization", format!("Token {}", self.api_key))
                .send()
                .await
                .map_err(|e| ImageError::Network(e.to_string()))?;

            let status: PredictionResponse = poll_resp
                .json()
                .await
                .map_err(|e| ImageError::Provider(e.to_string()))?;

            match status.status.as_str() {
                "succeeded" => {
                    let url = status.output
                        .and_then(|o| o.into_iter().next())
                        .ok_or_else(|| ImageError::Provider("no output URL".into()))?;

                    info!("Replicate: image ready at {url}");

                    let img_resp = self.client
                        .get(&url)
                        .send()
                        .await
                        .map_err(|e| ImageError::Network(e.to_string()))?;

                    let bytes = img_resp.bytes().await
                        .map_err(|e| ImageError::Network(e.to_string()))?;

                    return Ok(ImageBytes {
                        data: bytes.to_vec(),
                        mime_type: "image/png".into(),
                    });
                }
                "failed" | "canceled" => {
                    return Err(ImageError::Provider(
                        status.error.unwrap_or_else(|| "prediction failed".into()),
                    ));
                }
                _ => {
                    warn!("Replicate: prediction {prediction_id} status: {}", status.status);
                }
            }
        }
    }

    fn estimated_seconds(&self) -> u32 {
        15
    }

    fn cost_per_image(&self) -> f32 {
        0.003
    }
}
