use app_llm::{ChatChunk, ChatMessage, Tool, ToolCall, ToolResult};
use serde_json::json;

#[test]
fn tool_roundtrip_serde() {
    let tool = Tool {
        name: "roll_dice".to_string(),
        description: "Roll dice expression".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "dice": { "type": "string" },
                "modifier": { "type": "integer" }
            },
            "required": ["dice"]
        }),
    };
    let encoded = serde_json::to_string(&tool).unwrap();
    let decoded: Tool = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded.name, "roll_dice");
}

#[test]
fn tool_call_chunk_serde() {
    let chunk = ChatChunk::ToolCallStart {
        id: "call_abc".to_string(),
        name: "roll_dice".to_string(),
    };
    let encoded = serde_json::to_string(&chunk).unwrap();
    assert!(encoded.contains("tool_call_start"));
}

#[test]
fn tool_call_args_chunk_serde() {
    let chunk = ChatChunk::ToolCallArgsDelta {
        id: "call_abc".to_string(),
        args_fragment: r#"{"dice":"2d6"#.to_string(),
    };
    let encoded = serde_json::to_string(&chunk).unwrap();
    assert!(encoded.contains("tool_call_args_delta"));
}

#[test]
fn assistant_with_tool_calls_message_serde() {
    let msg = ChatMessage::AssistantWithToolCalls {
        content: Some("Let me roll".to_string()),
        tool_calls: vec![ToolCall {
            id: "call_abc".to_string(),
            name: "roll_dice".to_string(),
            args: serde_json::json!({"dice": "2d6"}),
        }],
    };
    let encoded = serde_json::to_string(&msg).unwrap();
    let decoded: ChatMessage = serde_json::from_str(&encoded).unwrap();
    match decoded {
        ChatMessage::AssistantWithToolCalls { tool_calls, .. } => {
            assert_eq!(tool_calls[0].name, "roll_dice");
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn tool_result_message_serde() {
    let msg = ChatMessage::ToolResult(ToolResult {
        tool_call_id: "call_abc".to_string(),
        content: r#"{"rolls":[3,4],"total":7}"#.to_string(),
        is_error: false,
    });
    let encoded = serde_json::to_string(&msg).unwrap();
    let decoded: ChatMessage = serde_json::from_str(&encoded).unwrap();
    match decoded {
        ChatMessage::ToolResult(tr) => assert_eq!(tr.tool_call_id, "call_abc"),
        _ => panic!("wrong variant"),
    }
}
