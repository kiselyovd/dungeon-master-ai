//! Golden wire tests for the inward-owned chat contract.

use app_application::models::chat::{
    ChatChunk, ChatMessage, FinishReason, MessagePart, Tool, ToolCall, ToolResult,
};
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

#[test]
fn chat_message_variants_have_stable_tagged_shapes() {
    let cases = [
        (
            ChatMessage::System {
                content: "system".into(),
            },
            json!({ "role": "system", "content": "system" }),
        ),
        (
            ChatMessage::User {
                parts: vec![MessagePart::Text {
                    text: "hello".into(),
                }],
            },
            json!({ "role": "user", "parts": [{ "type": "text", "text": "hello" }] }),
        ),
        (
            ChatMessage::Assistant {
                content: "answer".into(),
            },
            json!({ "role": "assistant", "content": "answer" }),
        ),
        (
            ChatMessage::AssistantWithToolCalls {
                content: None,
                tool_calls: vec![ToolCall {
                    id: "call-1".into(),
                    name: "roll_dice".into(),
                    args: json!({ "dice": "1d20" }),
                }],
            },
            json!({
                "role": "assistant_with_tool_calls",
                "content": null,
                "tool_calls": [{ "id": "call-1", "name": "roll_dice", "args": { "dice": "1d20" } }],
            }),
        ),
        (
            ChatMessage::ToolResult(ToolResult {
                tool_call_id: "call-1".into(),
                content: "{\"total\":12}".into(),
                is_error: false,
            }),
            json!({
                "role": "tool_result",
                "tool_call_id": "call-1",
                "content": "{\"total\":12}",
                "is_error": false,
            }),
        ),
    ];

    for (message, expected) in cases {
        let encoded = serde_json::to_value(&message).unwrap();
        assert_eq!(encoded, expected);
        assert_eq!(
            serde_json::from_value::<ChatMessage>(encoded).unwrap(),
            message
        );
    }
}

#[test]
fn chat_chunk_variants_have_stable_tagged_shapes() {
    let cases = [
        (
            ChatChunk::TextDelta { text: "a".into() },
            json!({ "type": "text_delta", "text": "a" }),
        ),
        (
            ChatChunk::ThinkingDelta { text: "b".into() },
            json!({ "type": "thinking_delta", "text": "b" }),
        ),
        (
            ChatChunk::ToolCallStart {
                id: "call-1".into(),
                name: "roll_dice".into(),
            },
            json!({ "type": "tool_call_start", "id": "call-1", "name": "roll_dice" }),
        ),
        (
            ChatChunk::ToolCallArgsDelta {
                id: "call-1".into(),
                args_fragment: "{\"dice\":".into(),
            },
            json!({ "type": "tool_call_args_delta", "id": "call-1", "args_fragment": "{\"dice\":" }),
        ),
        (
            ChatChunk::ToolCallDone {
                id: "call-1".into(),
            },
            json!({ "type": "tool_call_done", "id": "call-1" }),
        ),
        (
            ChatChunk::Done {
                reason: FinishReason::ToolUse,
            },
            json!({ "type": "done", "reason": "tool_use" }),
        ),
    ];

    for (chunk, expected) in cases {
        let encoded = serde_json::to_value(&chunk).unwrap();
        assert_eq!(encoded, expected);
        assert_eq!(serde_json::from_value::<ChatChunk>(encoded).unwrap(), chunk);
    }
}

#[test]
fn tool_value_types_have_exact_round_trip_shapes() {
    let tool = Tool {
        name: "roll_dice".into(),
        description: "Roll dice".into(),
        parameters: json!({ "type": "object" }),
    };
    let call = ToolCall {
        id: "call-1".into(),
        name: "roll_dice".into(),
        args: json!({ "dice": "2d6" }),
    };
    let result = ToolResult {
        tool_call_id: "call-1".into(),
        content: "{\"total\":7}".into(),
        is_error: false,
    };

    let tool_json = json!({
        "name": "roll_dice",
        "description": "Roll dice",
        "parameters": { "type": "object" },
    });
    let call_json = json!({ "id": "call-1", "name": "roll_dice", "args": { "dice": "2d6" } });
    let result_json =
        json!({ "tool_call_id": "call-1", "content": "{\"total\":7}", "is_error": false });

    assert_eq!(serde_json::to_value(&tool).unwrap(), tool_json);
    assert_eq!(serde_json::from_value::<Tool>(tool_json).unwrap(), tool);
    assert_eq!(serde_json::to_value(&call).unwrap(), call_json);
    assert_eq!(serde_json::from_value::<ToolCall>(call_json).unwrap(), call);
    assert_eq!(serde_json::to_value(&result).unwrap(), result_json);
    assert_eq!(
        serde_json::from_value::<ToolResult>(result_json).unwrap(),
        result
    );
}
