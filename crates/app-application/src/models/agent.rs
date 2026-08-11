use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::chat::{ChatMessage, MessagePart};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentTurnRequest {
    pub campaign_id: Uuid,
    pub session_id: Uuid,
    pub player_message: String,
    pub history: Vec<ChatMessage>,
    pub images: Vec<MessagePart>,
    pub board: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    TextDelta {
        text: String,
    },
    ToolCallStart {
        id: String,
        tool_name: String,
        round: usize,
    },
    ToolCallResult {
        id: String,
        tool_name: String,
        args: Value,
        result: Value,
        is_error: bool,
        round: usize,
        handled_by: String,
    },
    ReasoningText {
        text: String,
    },
    ImageGenerated {
        tool_call_id: String,
        round: usize,
        mime_type: String,
        image_b64: String,
        kind: String,
    },
    VideoGenerated {
        tool_call_id: String,
        round: usize,
        mime_type: String,
        video_b64: String,
        kind: String,
    },
    AgentDone {
        total_rounds: usize,
    },
}
