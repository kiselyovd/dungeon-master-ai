use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub args: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub content: String,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessagePart {
    Text {
        text: String,
    },
    Image {
        mime: String,
        data_b64: String,
        name: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub model: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    #[serde(default)]
    pub tools: Vec<Tool>,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub reasoning: Option<ReasoningSpec>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningSpec {
    Low,
    Medium,
    High,
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
    AssistantWithToolCalls {
        content: Option<String>,
        tool_calls: Vec<ToolCall>,
    },
    ToolResult(ToolResult),
}

impl ChatMessage {
    pub fn user_text(text: impl Into<String>) -> Self {
        Self::User {
            parts: vec![MessagePart::Text { text: text.into() }],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatChunk {
    TextDelta { text: String },
    ThinkingDelta { text: String },
    ToolCallStart { id: String, name: String },
    ToolCallArgsDelta { id: String, args_fragment: String },
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
