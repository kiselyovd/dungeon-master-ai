use app_application::models::agent::AgentEvent;
use app_application::ports::media::ImageSource;
use axum::response::sse::Event;

/// Pure mapping from application events to the stable frontend SSE contract.
pub fn agent_event_to_wire(event: AgentEvent) -> (&'static str, serde_json::Value) {
    match event {
        AgentEvent::ReasoningText { text } => {
            ("reasoning_text", serde_json::json!({ "text": text }))
        }
        AgentEvent::ImageGenerated {
            tool_call_id,
            round,
            mime_type,
            image_b64,
            kind,
            source,
        } => {
            let (source, asset_id) = match source {
                ImageSource::Generated => ("generated", None),
                ImageSource::Bundled { asset_id } => ("bundled", Some(asset_id)),
            };
            let mut payload = serde_json::json!({
                "tool_call_id": tool_call_id,
                "round": round,
                "mime_type": mime_type,
                "image_b64": image_b64,
                "kind": kind,
                "source": source,
            });
            if let Some(asset_id) = asset_id {
                payload["asset_id"] = serde_json::Value::String(asset_id);
            }
            ("image_generated", payload)
        }
        AgentEvent::TextDelta { text } => ("text_delta", serde_json::json!({ "text": text })),
        AgentEvent::ToolCallStart {
            id,
            tool_name,
            round,
        } => (
            "tool_call_start",
            serde_json::json!({ "id": id, "tool_name": tool_name, "round": round }),
        ),
        AgentEvent::ToolCallResult {
            id,
            tool_name,
            args,
            result,
            is_error,
            round,
            handled_by,
        } => (
            "tool_call_result",
            serde_json::json!({
                "id": id,
                "tool_name": tool_name,
                "args": args,
                "result": result,
                "is_error": is_error,
                "round": round,
                "handled_by": handled_by,
            }),
        ),
        AgentEvent::VideoGenerated {
            tool_call_id,
            round,
            mime_type,
            video_b64,
            kind,
        } => (
            "video_generated",
            serde_json::json!({
                "tool_call_id": tool_call_id,
                "round": round,
                "mime_type": mime_type,
                "video_b64": video_b64,
                "kind": kind,
            }),
        ),
        AgentEvent::AgentDone { total_rounds } => (
            "agent_done",
            serde_json::json!({ "total_rounds": total_rounds }),
        ),
    }
}

pub fn agent_event_to_sse(event: AgentEvent) -> Event {
    let (event_name, payload) = agent_event_to_wire(event);
    Event::default()
        .event(event_name)
        .json_data(payload)
        .expect("application event is JSON serializable")
}
