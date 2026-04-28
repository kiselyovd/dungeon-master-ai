//! LLM provider abstraction for the dungeon-master-ai backend.

pub mod anthropic;
pub mod mock;
pub mod openai_compat;
pub mod provider;

pub use anthropic::AnthropicProvider;
pub use mock::MockProvider;
pub use openai_compat::OpenAICompatProvider;
pub use provider::{
    ChatChunk, ChatMessage, ChatRequest, ChunkStream, FinishReason, LlmError, LlmProvider,
};
