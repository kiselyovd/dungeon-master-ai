//! Video generation provider trait + streaming progress events.
//!
//! Unlike images (single PNG response), video generation is long (20-60s
//! on RTX 3080 for LTX-Video distilled) and benefits from a progress channel
//! so the UI can render a percent bar + ETA + cancel.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoCapabilities {
    pub duration_range_secs: (u32, u32),
    pub max_resolution: (u32, u32),
    pub supports_image_init: bool,
    pub avg_seconds_per_clip: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

pub struct VideoStream {
    pub events: mpsc::Receiver<VideoEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    fn video_capabilities_struct_roundtrips() {
        let c = VideoCapabilities {
            duration_range_secs: (3, 8),
            max_resolution: (704, 480),
            supports_image_init: true,
            avg_seconds_per_clip: 24,
        };
        let v = serde_json::to_value(&c).unwrap();
        let back: VideoCapabilities = serde_json::from_value(v).unwrap();
        assert_eq!(back.duration_range_secs, c.duration_range_secs);
        assert_eq!(back.supports_image_init, c.supports_image_init);
    }

    #[test]
    fn video_prompt_defaults_match_ltx_v096() {
        let p: VideoPrompt =
            serde_json::from_value(serde_json::json!({ "text": "a cat" })).unwrap();
        assert_eq!(p.resolution, (704, 480));
        assert_eq!(p.frame_count, 96);
    }

    #[test]
    fn video_event_done_serialises_with_type_tag() {
        let evt = VideoEvent::Done {
            mp4_bytes: vec![1, 2, 3],
            duration_seconds: 4.0,
        };
        let v = serde_json::to_value(&evt).unwrap();
        assert_eq!(v["type"], "done");
        assert_eq!(v["duration_seconds"], 4.0);
    }
}
