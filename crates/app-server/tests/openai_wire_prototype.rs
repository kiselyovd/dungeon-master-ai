#[path = "support/openai_wire_prototype.rs"]
mod wire;
use app_application::models::chat::{ChatChunk, ChatMessage, ChatRequest};
use serde_json::json;

#[test]
fn one_frame_can_contain_multiple_tools_with_index_order_and_fragmented_unicode() {
    let frame = json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":1,"id":"second","function":{"name":"query_rules","arguments":"{\"query\":\"бб\"}"}},{"index":0,"id":"first","function":{"name":"query_rules","arguments":"{\"query\":\"аа\"}"}}]},"finish_reason":null}]});
    let bytes=format!("data: {frame}\r\n\r\ndata: {{\"choices\":[{{\"index\":0,\"delta\":{{}},\"finish_reason\":\"tool_calls\"}}]}}\r\n\r\ndata: [DONE]\r\n\r\n");
    let mut decoder = wire::OpenAiDecoder::default();
    let mut chunks = Vec::new();
    for byte in bytes.as_bytes() {
        chunks.extend(decoder.push(&[*byte]).unwrap());
    }
    decoder.finish().unwrap();
    let completed = chunks
        .iter()
        .filter_map(|chunk| {
            if let ChatChunk::ToolCallDone { id } = chunk {
                Some(id.as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(completed, vec!["first", "second"]);
    assert!(chunks.iter().any(|chunk|matches!(chunk,ChatChunk::ToolCallArgsDelta{id,args_fragment}if id=="second"&&args_fragment.contains("бб"))));
}
#[test]
fn incomplete_stream_fails_and_system_context_is_serialized() {
    let decoder = wire::OpenAiDecoder::default();
    assert!(decoder.finish().is_err());
    let body = wire::request_body(ChatRequest {
        messages: vec![ChatMessage::user_text("Иду на причал")],
        model: "bonsai".into(),
        max_tokens: Some(2048),
        temperature: Some(1.0),
        tools: vec![],
        system_prompt: Some("AUTHORED CANON".into()),
        reasoning: None,
    });
    assert_eq!(body["messages"][0]["content"], "AUTHORED CANON");
}

#[test]
fn duplicate_call_ids_fail_before_any_completed_tool_is_delivered() {
    let mut decoder = wire::OpenAiDecoder::default();
    let frame = json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"same","function":{"name":"first","arguments":"{}"}},{"index":1,"id":"same","function":{"name":"second","arguments":"{}"}}]},"finish_reason":null}]});
    assert!(decoder
        .push(format!("data: {frame}\n\n").as_bytes())
        .unwrap()
        .is_empty());
    assert!(decoder.push(b"data: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"tool_calls\"}]}\n\n").is_err());
}
