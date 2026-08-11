//! Adapter-local facade for the inward-owned LLM contract.

pub use app_application::models::chat::{
    Capabilities, ChatChunk, ChatMessage, ChatRequest, FinishReason, MessagePart, ReasoningSpec,
    Tool, ToolCall, ToolResult,
};
pub use app_application::ports::llm::{ChunkStream, LlmError, LlmProvider};

pub(crate) fn to_genai_effort(spec: ReasoningSpec) -> genai::chat::ReasoningEffort {
    match spec {
        ReasoningSpec::Low => genai::chat::ReasoningEffort::Low,
        ReasoningSpec::Medium => genai::chat::ReasoningEffort::Medium,
        ReasoningSpec::High => genai::chat::ReasoningEffort::High,
    }
}
