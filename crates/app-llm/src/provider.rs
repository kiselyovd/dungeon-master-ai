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
    /// When `Some`, the provider asks the model to allocate a thinking budget
    /// before producing the final answer. Mapped to provider-specific shapes
    /// inside each adapter via genai's `ChatOptions::reasoning_effort`.
    #[serde(default)]
    pub reasoning: Option<ReasoningSpec>,
}

/// User-facing reasoning budget tier. Adapters translate to provider semantics:
/// OpenAI o-series + gpt-5 (and OpenRouter-routed reasoning models) -> `reasoning.effort`
/// Other providers -> no-op (capabilities.reasoning = false)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningSpec {
    Low,
    Medium,
    High,
}

impl ReasoningSpec {
    pub fn to_genai_effort(self) -> genai::chat::ReasoningEffort {
        match self {
            Self::Low => genai::chat::ReasoningEffort::Low,
            Self::Medium => genai::chat::ReasoningEffort::Medium,
            Self::High => genai::chat::ReasoningEffort::High,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum ChatMessage {
    System {
        content: String,
    },
    User {
        parts: Vec<MessagePart>,
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

impl ChatMessage {
    /// Convenience for the common single-text-part user turn.
    pub fn user_text(text: impl Into<String>) -> Self {
        Self::User {
            parts: vec![MessagePart::Text { text: text.into() }],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatChunk {
    TextDelta {
        text: String,
    },
    /// Thinking/reasoning delta from the model. Emitted by providers whose
    /// active model supports reasoning (OpenAI o-series, gpt-5, and
    /// OpenRouter-routed reasoning models). Consumers render this in a
    /// collapsible UI.
    ThinkingDelta {
        text: String,
    },
    /// The LLM started a tool-call block; args will follow as deltas.
    ToolCallStart {
        id: String,
        name: String,
    },
    /// Streaming fragment of the JSON args for an in-progress tool-call.
    ToolCallArgsDelta {
        id: String,
        args_fragment: String,
    },
    /// The LLM finished the tool-call args. Consumer should now parse + execute.
    ToolCallDone {
        id: String,
    },
    Done {
        reason: FinishReason,
    },
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

/// Per-model capability flags. Used by the catalog + UI to gate toggles and by
/// the agent loop to skip unsupported operations. Wired into `LlmProvider` via
/// `capabilities_for_model`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    #[serde(default)]
    pub vision_input: bool,
    #[serde(default)]
    pub reasoning: bool,
    #[serde(default)]
    pub tool_calls: bool,
    #[serde(default)]
    pub streaming: bool,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn stream_chat(&self, req: ChatRequest) -> Result<ChunkStream, LlmError>;

    fn name(&self) -> &'static str;

    /// Per-model capability lookup. Implementations consult their per-model
    /// table (e.g. catalog entry, name-pattern inference) and return the right
    /// caps for the requested model.
    fn capabilities_for_model(&self, model_id: &str) -> Capabilities;

    /// The model id this provider would use for the next request (its
    /// configured default). Used by `capabilities()` and by routers that need a
    /// concrete model name without re-passing it.
    fn active_model(&self) -> &str;

    /// Capabilities of the currently-active model. Default impl routes through
    /// `capabilities_for_model(self.active_model())`.
    fn capabilities(&self) -> Capabilities {
        self.capabilities_for_model(self.active_model())
    }

    /// Whether this provider supports tool-calling for the active model.
    /// Derived from `capabilities()`. The orchestrator passes an empty tools
    /// list when this returns false.
    fn supports_tools(&self) -> bool {
        self.capabilities().tool_calls
    }

    /// Whether this provider's active model accepts image content parts.
    /// Derived from `capabilities()`.
    fn supports_vision(&self) -> bool {
        self.capabilities().vision_input
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

    #[test]
    fn chat_message_user_text_helper() {
        let m = ChatMessage::user_text("hello");
        match m {
            ChatMessage::User { parts } => {
                assert_eq!(parts.len(), 1);
                assert!(matches!(&parts[0], MessagePart::Text { text } if text == "hello"));
            }
            _ => panic!("expected User"),
        }
    }

    #[test]
    fn capabilities_default_is_all_false() {
        let c = Capabilities::default();
        assert!(!c.vision_input);
        assert!(!c.reasoning);
        assert!(!c.tool_calls);
        assert!(!c.streaming);
    }

    #[test]
    fn reasoning_spec_to_genai_effort_maps_all_variants() {
        use genai::chat::ReasoningEffort;
        assert!(matches!(
            ReasoningSpec::Low.to_genai_effort(),
            ReasoningEffort::Low
        ));
        assert!(matches!(
            ReasoningSpec::Medium.to_genai_effort(),
            ReasoningEffort::Medium
        ));
        assert!(matches!(
            ReasoningSpec::High.to_genai_effort(),
            ReasoningEffort::High
        ));
    }

    #[test]
    fn reasoning_spec_serde_lowercase() {
        let v = serde_json::to_value(ReasoningSpec::Medium).unwrap();
        assert_eq!(v, serde_json::json!("medium"));
        let back: ReasoningSpec = serde_json::from_value(v).unwrap();
        assert_eq!(back, ReasoningSpec::Medium);
    }

    #[test]
    fn chat_request_reasoning_field_defaults_none() {
        let req: ChatRequest = serde_json::from_value(serde_json::json!({
            "messages": [],
            "model": "test",
        }))
        .unwrap();
        assert_eq!(req.reasoning, None);
    }

    #[test]
    fn capabilities_serde_round_trip() {
        let c = Capabilities {
            vision_input: true,
            reasoning: false,
            tool_calls: true,
            streaming: true,
        };
        let v = serde_json::to_value(c).unwrap();
        assert_eq!(
            v,
            json!({
                "vision_input": true,
                "reasoning": false,
                "tool_calls": true,
                "streaming": true
            })
        );
        let back: Capabilities = serde_json::from_value(v).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn chat_message_user_serde_array_shape() {
        let m = ChatMessage::User {
            parts: vec![
                MessagePart::Text {
                    text: "see this:".into(),
                },
                MessagePart::Image {
                    mime: "image/png".into(),
                    data_b64: "aGk=".into(),
                    name: None,
                },
            ],
        };
        let v = serde_json::to_value(&m).unwrap();
        assert_eq!(v["role"], "user");
        assert!(v["parts"].is_array());
        assert_eq!(v["parts"][0]["type"], "text");
        assert_eq!(v["parts"][1]["type"], "image");
        let back: ChatMessage = serde_json::from_value(v).unwrap();
        assert_eq!(back, m);
    }
}
