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
    #[error("native model runtime unavailable")]
    RuntimeUnavailable,
    #[error("native model runtime changed")]
    RuntimeChanged,
}

pub type ChunkStream = BoxStream<'static, Result<ChatChunk, LlmError>>;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn verify_execution_environment(&self) -> Result<(), LlmError> {
        Ok(())
    }
    async fn stream_chat(&self, request: ChatRequest) -> Result<ChunkStream, LlmError>;

    /// Request structured output where supported. Callers must still validate
    /// the schema and terminal event: providers may ignore this preference.
    async fn stream_chat_json(
        &self,
        request: ChatRequest,
        _schema: serde_json::Value,
    ) -> Result<ChunkStream, LlmError> {
        self.stream_chat(request).await
    }

    fn name(&self) -> &'static str;

    /// Private stable execution origin. Consumers remove URL credentials and
    /// hash it before durable storage. Never log this value. None makes interrupted replay
    /// unavailable, rather than guessing that an unrelated endpoint is equivalent.
    fn execution_origin(&self) -> Option<String> {
        None
    }

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
