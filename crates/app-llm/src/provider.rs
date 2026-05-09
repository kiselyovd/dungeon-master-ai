use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// Schema definition for a single tool the LLM may call.
/// Passed to `ChatRequest.tools` so the provider can include it in the request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tool {
    pub name: String,
    pub description: String,
    /// JSON Schema object describing the arguments.
    pub parameters: Value,
}

/// A completed tool-call emitted by the LLM (assembled from streaming chunks).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub args: Value,
}

/// A tool result to feed back to the LLM in the next round.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolResult {
    pub tool_call_id: String,
    /// JSON string of the result returned by the engine.
    pub content: String,
    pub is_error: bool,
}

/// A single segment of a chat message body. User turns are multi-part to
/// support images alongside text; non-user roles still serialize through the
/// `Text` variant only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessagePart {
    Text {
        text: String,
    },
    Image {
        /// MIME type, e.g. `image/png`, `image/jpeg`, `image/webp`.
        mime: String,
        /// Base64-encoded payload, no `data:` prefix.
        data_b64: String,
        /// Optional original filename for display.
        name: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub model: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    /// Tool schemas to include. Empty = no tool-calling.
    #[serde(default)]
    pub tools: Vec<Tool>,
    /// Optional system prompt override. If None, provider uses its default.
    #[serde(default)]
    pub system_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum ChatMessage {
    System {
        content: String,
    },
    User {
        content: String,
    },
    Assistant {
        content: String,
    },
    /// Assistant turn that included tool-call blocks (alongside optional text).
    AssistantWithToolCalls {
        content: Option<String>,
        tool_calls: Vec<ToolCall>,
    },
    /// A tool result injected back after the engine executed a tool-call.
    ToolResult(ToolResult),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatChunk {
    TextDelta { text: String },
    /// The LLM started a tool-call block; args will follow as deltas.
    ToolCallStart { id: String, name: String },
    /// Streaming fragment of the JSON args for an in-progress tool-call.
    ToolCallArgsDelta { id: String, args_fragment: String },
    /// The LLM finished the tool-call args. Consumer should now parse + execute.
    ToolCallDone { id: String },
    Done { reason: FinishReason },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    ToolUse,
    Length,
    Error,
}

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
    async fn stream_chat(&self, req: ChatRequest) -> Result<ChunkStream, LlmError>;

    fn name(&self) -> &'static str;

    /// Whether this provider supports tool-calling. Providers that return false
    /// will receive an empty tools list; the orchestrator will not call
    /// `stream_chat` with tool-call enabled requests.
    fn supports_tools(&self) -> bool {
        true
    }

    /// Whether this provider's model accepts image content parts. Default
    /// `false`; providers that wrap genai (Anthropic, OpenAI-compat,
    /// mistralrs) override to `true`. Used today only for tests and reading
    /// the capability matrix; not enforced server-side.
    fn supports_vision(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn message_part_text_serde_round_trip() {
        let p = MessagePart::Text { text: "hi".into() };
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(v, json!({"type": "text", "text": "hi"}));
        let back: MessagePart = serde_json::from_value(v).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn message_part_image_serde_round_trip() {
        let p = MessagePart::Image {
            mime: "image/png".into(),
            data_b64: "aGVsbG8=".into(),
            name: Some("greeting.png".into()),
        };
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(
            v,
            json!({
                "type": "image",
                "mime": "image/png",
                "data_b64": "aGVsbG8=",
                "name": "greeting.png"
            })
        );
        let back: MessagePart = serde_json::from_value(v).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn message_part_image_name_optional_serializes_null() {
        let p = MessagePart::Image {
            mime: "image/jpeg".into(),
            data_b64: "x".into(),
            name: None,
        };
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(
            v,
            json!({"type": "image", "mime": "image/jpeg", "data_b64": "x", "name": null})
        );
    }
}
