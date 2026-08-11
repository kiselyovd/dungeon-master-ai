use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImageGenerationRequest {
    pub content_prompt: String,
    pub style_preset: String,
    pub scene_id: Option<String>,
    pub npc_ids: Vec<String>,
    pub backend_preset: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoGenerationRequest {
    pub content_prompt: String,
    pub init_image_b64: Option<String>,
    pub duration_seconds: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedMedia {
    pub data: Vec<u8>,
    pub mime_type: String,
}

#[derive(Debug, Error)]
pub enum MediaError {
    #[error("media provider failed with code {code}")]
    Provider { code: &'static str },
    #[error("media operation timed out after {seconds}s")]
    Timeout { seconds: u64 },
    #[error("media operation cancelled")]
    Cancelled,
}

#[async_trait]
pub trait ImageGenerator: Send + Sync {
    async fn generate(&self, request: ImageGenerationRequest)
        -> Result<GeneratedMedia, MediaError>;
}

#[async_trait]
pub trait VideoGenerator: Send + Sync {
    async fn generate(&self, request: VideoGenerationRequest)
        -> Result<GeneratedMedia, MediaError>;
}
