use async_trait::async_trait;
use futures::stream::BoxStream;
use thiserror::Error;

use crate::models::chat::{Capabilities, ChatChunk, ChatRequest};

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("provider error: {0}")]
    Provider(String),
    #[error("rate limit exceeded")]
    RateLimit,
    #[error("authentication failed")]
    AuthFailure,
    #[error("network error: {0}")]
    Network(String),
    #[error("invalid response: {0}")]
    InvalidResponse(String),
}

pub type ChunkStream = BoxStream<'static, Result<ChatChunk, LlmError>>;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn stream_chat(&self, request: ChatRequest) -> Result<ChunkStream, LlmError>;

    fn name(&self) -> &'static str;

    fn capabilities_for_model(&self, model_id: &str) -> Capabilities;

    fn active_model(&self) -> &str;

    fn capabilities(&self) -> Capabilities {
        self.capabilities_for_model(self.active_model())
    }

    fn supports_tools(&self) -> bool {
        self.capabilities().tool_calls
    }

    fn supports_vision(&self) -> bool {
        self.capabilities().vision_input
    }
}
