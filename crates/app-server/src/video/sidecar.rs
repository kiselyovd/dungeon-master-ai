//! HTTP adapter to the Python sidecar's POST /video/generate SSE endpoint.
//!
//! Same sidecar process as image generation (E.x dispatcher pattern); the
//! sidecar's `ltx-video` backend handles requests. Single port, single GPU
//! mutex - image and video serialise naturally.

use std::time::Duration;

use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use futures::StreamExt;
use tokio::sync::mpsc;

use crate::video::provider::{
    VideoCapabilities, VideoError, VideoEvent, VideoPrompt, VideoProvider, VideoStream,
};

const REQUEST_TIMEOUT_SECS: u64 = 300;

pub struct LocalVideoSidecarProvider {
    base_url: String,
    client: reqwest::Client,
}

impl LocalVideoSidecarProvider {
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
impl VideoProvider for LocalVideoSidecarProvider {
    async fn generate(&self, prompt: VideoPrompt) -> Result<VideoStream, VideoError> {
        let (tx, rx) = mpsc::channel::<VideoEvent>(64);
        let url = format!("{}/video/generate", self.base_url);
        let body = serde_json::json!({
            "prompt": prompt.text,
            "init_image_b64": prompt.init_image_b64,
            "resolution": prompt.resolution,
            "frame_count": prompt.frame_count,
            "seed": prompt.seed,
            "teacache_threshold": prompt.teacache_threshold,
        });
        let request = self.client.post(&url).json(&body).send();
        tokio::spawn(async move {
            let resp = match request.await {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx
                        .send(VideoEvent::Error {
                            message: e.to_string(),
                        })
                        .await;
                    return;
                }
            };
            if !resp.status().is_success() {
                let _ = tx
                    .send(VideoEvent::Error {
                        message: format!("sidecar returned {}", resp.status()),
                    })
                    .await;
                return;
            }
            let mut stream = resp.bytes_stream();
            let mut buffer = String::new();
            while let Some(chunk) = stream.next().await {
                let chunk = match chunk {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = tx
                            .send(VideoEvent::Error {
                                message: e.to_string(),
                            })
                            .await;
                        return;
                    }
                };
                buffer.push_str(&String::from_utf8_lossy(&chunk));
                while let Some(sep) = buffer.find("\n\n") {
                    let frame_text = buffer[..sep].to_string();
                    buffer = buffer[sep + 2..].to_string();
                    let data_line = frame_text.lines().find_map(|l| l.strip_prefix("data: "));
                    if let Some(data) = data_line {
                        if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(data) {
                            // mp4_bytes is sent as base64 string for SSE
                            // friendliness; decode here so the rest of the
                            // app sees raw bytes per VideoEvent::Done shape.
                            if let Some(b64) = value.get("mp4_bytes_b64").and_then(|v| v.as_str())
                            {
                                if let Ok(bytes) = B64.decode(b64) {
                                    value["mp4_bytes"] = serde_json::Value::Array(
                                        bytes
                                            .into_iter()
                                            .map(|b| serde_json::Value::from(b as u64))
                                            .collect(),
                                    );
                                }
                            }
                            if let Ok(evt) = serde_json::from_value::<VideoEvent>(value) {
                                if tx.send(evt).await.is_err() {
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        });
        Ok(VideoStream { events: rx })
    }

    fn capabilities(&self) -> VideoCapabilities {
        VideoCapabilities {
            duration_range_secs: (3, 8),
            max_resolution: (704, 480),
            supports_image_init: true,
            avg_seconds_per_clip: 24,
        }
    }
}
