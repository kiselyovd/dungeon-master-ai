//! Inward-owned contracts for image and video generation adapters.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc;

/// Version shared by the Rust adapter and Python media sidecar.
pub const MEDIA_CONTRACT_VERSION: u16 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaContractCapabilities {
    pub contract_version: u16,
    pub supports_video_init_image: bool,
    pub supports_video_cancellation: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImagePrompt {
    pub content_prompt: String,
    pub style_preset: String,
    pub scene_id: Option<String>,
    pub npc_ids: Vec<String>,
    #[serde(default)]
    pub backend_preset: Option<String>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageBytes {
    pub data: Vec<u8>,
    pub mime_type: String,
}

#[derive(Debug, Error)]
pub enum ImageError {
    #[error("provider error: {0}")]
    Provider(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("timeout after {secs}s")]
    Timeout { secs: u64 },
    #[error("authentication failed")]
    Auth,
    #[error("media service degraded with code {code}")]
    Degraded { code: &'static str },
    #[error("media operation cancelled")]
    Cancelled,
}

#[async_trait]
pub trait ImageProvider: Send + Sync {
    async fn generate(&self, prompt: ImagePrompt) -> Result<ImageBytes, ImageError>;
    fn estimated_seconds(&self) -> u32;
    fn cost_per_image(&self) -> f32;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoCapabilities {
    pub duration_range_secs: (u32, u32),
    pub max_resolution: (u32, u32),
    pub supports_image_init: bool,
    pub avg_seconds_per_clip: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoPrompt {
    pub text: String,
    #[serde(default)]
    pub init_image_b64: Option<String>,
    #[serde(default = "default_resolution")]
    pub resolution: (u32, u32),
    #[serde(default = "default_frame_count")]
    pub frame_count: u32,
    #[serde(default)]
    pub seed: Option<u64>,
}

fn default_resolution() -> (u32, u32) {
    (704, 480)
}

fn default_frame_count() -> u32 {
    96
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VideoEvent {
    Started {
        estimated_seconds: u32,
    },
    Progress {
        percent: f32,
        eta_seconds: u32,
    },
    Done {
        mp4_bytes: Vec<u8>,
        duration_seconds: f32,
    },
    Error {
        message: String,
    },
    Cancelled,
    Degraded {
        code: String,
    },
}

pub struct VideoStream {
    pub events: mpsc::Receiver<VideoEvent>,
}

impl VideoStream {
    pub fn from_events(events: mpsc::Receiver<VideoEvent>) -> Self {
        Self { events }
    }
}

#[derive(Debug, Error)]
pub enum VideoError {
    #[error("backend not running")]
    BackendNotRunning,
    #[error("provider error: {0}")]
    Provider(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("timeout")]
    Timeout,
    #[error("init image is not supported by this video backend")]
    UnsupportedInitImage,
    #[error("video operation cancelled")]
    Cancelled,
    #[error("video service degraded with code {code}")]
    Degraded { code: &'static str },
}

#[async_trait]
pub trait VideoProvider: Send + Sync {
    async fn generate(&self, prompt: VideoPrompt) -> Result<VideoStream, VideoError>;
    fn capabilities(&self) -> VideoCapabilities;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_contract_is_versioned_and_matches_sidecar_claims() {
        let caps = MediaContractCapabilities {
            contract_version: MEDIA_CONTRACT_VERSION,
            supports_video_init_image: false,
            supports_video_cancellation: true,
        };
        assert_eq!(caps.contract_version, 1);
        assert!(!caps.supports_video_init_image);
        assert!(caps.supports_video_cancellation);
    }

    #[test]
    fn video_prompt_defaults_and_events_keep_stable_wire_shapes() {
        let prompt: VideoPrompt =
            serde_json::from_value(serde_json::json!({ "text": "a cat" })).expect("valid prompt");
        assert_eq!(prompt.resolution, (704, 480));
        assert_eq!(prompt.frame_count, 96);

        let event = VideoEvent::Done {
            mp4_bytes: vec![1, 2, 3],
            duration_seconds: 4.0,
        };
        let value = serde_json::to_value(event).expect("serializable event");
        assert_eq!(value["type"], "done");
        assert_eq!(value["duration_seconds"], 4.0);
    }

    #[tokio::test]
    async fn dropping_a_video_stream_closes_the_adapter_channel() {
        let (events_tx, events_rx) = mpsc::channel(1);
        let closed = tokio::spawn(async move {
            events_tx.closed().await;
        });
        drop(VideoStream::from_events(events_rx));
        closed.await.expect("closed watcher completes");
    }
}
