//! LLM provider abstraction for the dungeon-master-ai backend.

mod genai_common;
pub mod mistralrs_provider;
pub mod mock;
pub mod openai_compat;
pub mod provider;
pub mod retry;
pub mod sidecar_launcher;

pub use mistralrs_provider::MistralrsLocalProvider;
pub use mock::MockProvider;
pub use openai_compat::OpenAICompatProvider;
pub use provider::{
    Capabilities, ChatChunk, ChatMessage, ChatRequest, ChunkStream, FinishReason, LlmError,
    LlmProvider, MessagePart, ReasoningSpec, Tool, ToolCall, ToolResult,
};
pub use retry::{default_policy, is_retryable, RetryableProvider};
pub use sidecar_launcher::{
    MockSidecarLauncher, NullSidecarLauncher, SidecarError, SidecarHandle, SidecarLauncher,
    SpawnSpec,
};
