//! Shared trait + types for image generation providers.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Input to any image generation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImagePrompt {
    /// 30-word content description (LLM-generated).
    pub content_prompt: String,
    /// One of: dark_fantasy, portrait, map.
    pub style_preset: String,
    /// Optional scene id for cache keying.
    pub scene_id: Option<String>,
    /// NPC ids present in the scene (sorted before cache key hash).
    pub npc_ids: Vec<String>,
}

/// Raw image bytes returned by a provider.
#[derive(Debug, Clone)]
pub struct ImageBytes {
    pub data: Vec<u8>,
    pub mime_type: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("provider error: {0}")]
    Provider(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("timeout after {secs}s")]
    Timeout { secs: u64 },
    #[error("authentication failed")]
    Auth,
}

#[async_trait]
pub trait ImageProvider: Send + Sync {
    async fn generate(&self, prompt: ImagePrompt) -> Result<ImageBytes, ImageError>;

    /// Estimated generation time in seconds (for frontend status messages).
    fn estimated_seconds(&self) -> u32;

    /// Approximate cost per image in USD (for cost tracking).
    fn cost_per_image(&self) -> f32;
}
