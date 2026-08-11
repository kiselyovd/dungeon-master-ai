//! Temporary compatibility facade for the inward LLM contract and adapters.

pub mod mistralrs_provider {
    pub use adapter_llm::mistralrs_provider::*;
}
pub mod mock {
    pub use adapter_llm::mock::*;
}
pub mod openai_compat {
    pub use adapter_llm::openai_compat::*;
}
pub mod provider {
    pub use adapter_llm::provider::*;
}
pub mod retry {
    pub use adapter_llm::retry::*;
}
pub mod sidecar_launcher;

pub use adapter_llm::{
    default_policy, is_retryable, Capabilities, ChatChunk, ChatMessage, ChatRequest, ChunkStream,
    FinishReason, LlmError, LlmProvider, MessagePart, MistralrsLocalProvider, MockProvider,
    OpenAICompatProvider, ReasoningSpec, RetryableProvider, Tool, ToolCall, ToolResult,
};
pub use sidecar_launcher::{
    MockSidecarLauncher, NullSidecarLauncher, SidecarError, SidecarHandle, SidecarLauncher,
    SpawnSpec,
};
