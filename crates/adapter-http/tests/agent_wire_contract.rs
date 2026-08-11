use adapter_http::sse::agent_event_to_wire;
use app_application::models::agent::AgentEvent;
use serde_json::json;

#[test]
fn maps_every_agent_event_to_the_stable_sse_wire_contract() {
    let cases = [
        (
            AgentEvent::ReasoningText {
                text: "plan".into(),
            },
            "reasoning_text",
            json!({ "text": "plan" }),
        ),
        (
            AgentEvent::ImageGenerated {
                tool_call_id: "img-1".into(),
                round: 1,
                mime_type: "image/png".into(),
                image_b64: "aQ==".into(),
                kind: "map".into(),
            },
            "image_generated",
            json!({
                "tool_call_id": "img-1",
                "round": 1,
                "mime_type": "image/png",
                "image_b64": "aQ==",
                "kind": "map",
            }),
        ),
        (
            AgentEvent::TextDelta {
                text: "hello".into(),
            },
            "text_delta",
            json!({ "text": "hello" }),
        ),
        (
            AgentEvent::ToolCallStart {
                id: "call-1".into(),
                tool_name: "roll_dice".into(),
                round: 2,
            },
            "tool_call_start",
            json!({ "id": "call-1", "tool_name": "roll_dice", "round": 2 }),
        ),
        (
            AgentEvent::ToolCallResult {
                id: "call-1".into(),
                tool_name: "roll_dice".into(),
                args: json!({ "dice": "2d6" }),
                result: json!({ "total": 7 }),
                is_error: false,
                round: 2,
                handled_by: "engine".into(),
            },
            "tool_call_result",
            json!({
                "id": "call-1",
                "tool_name": "roll_dice",
                "args": { "dice": "2d6" },
                "result": { "total": 7 },
                "is_error": false,
                "round": 2,
                "handled_by": "engine",
            }),
        ),
        (
            AgentEvent::VideoGenerated {
                tool_call_id: "vid-1".into(),
                round: 3,
                mime_type: "video/mp4".into(),
                video_b64: "dg==".into(),
                kind: "chat".into(),
            },
            "video_generated",
            json!({
                "tool_call_id": "vid-1",
                "round": 3,
                "mime_type": "video/mp4",
                "video_b64": "dg==",
                "kind": "chat",
            }),
        ),
        (
            AgentEvent::AgentDone { total_rounds: 4 },
            "agent_done",
            json!({ "total_rounds": 4 }),
        ),
    ];

    for (event, expected_name, expected_payload) in cases {
        let (name, payload) = agent_event_to_wire(event);
        assert_eq!(name, expected_name);
        assert_eq!(payload, expected_payload);
    }
}
