use adapter_llm::{
    ChatChunk, ChatMessage, ChatRequest, FinishReason, LlmProvider, OpenAICompatProvider, Tool,
};
use futures::StreamExt;
use serde_json::{json, Value};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn request() -> ChatRequest {
    ChatRequest {
        messages: vec![ChatMessage::user_text("Inspect and remember")],
        model: "dmai-bonsai-eval".into(),
        max_tokens: Some(256),
        temperature: Some(0.0),
        tools: vec![
            Tool {
                name: "first".into(),
                description: "First action".into(),
                parameters: json!({"type":"object"}),
            },
            Tool {
                name: "second".into(),
                description: "Second action".into(),
                parameters: json!({"type":"object"}),
            },
        ],
        system_prompt: Some("CANONICAL DM CONTEXT".into()),
        reasoning: None,
    }
}

#[tokio::test]
async fn multiple_tools_in_one_provider_frame_keep_both_ids_arguments_and_order() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).and(path("/chat/completions")).respond_with(|request:&wiremock::Request| {
        let body:Value=serde_json::from_slice(&request.body).unwrap();
        let calls=vec![json!({"id":"call-first","type":"function","function":{"name":"first","arguments":"{\"name\":\"Мила\"}"}}),json!({"id":"call-second","type":"function","function":{"name":"second","arguments":"{\"value\":2}"}})];
        if body["stream"]==true {
            let calls:Vec<_>=calls.into_iter().enumerate().map(|(index,mut call)|{call["index"]=json!(index);call}).collect();
            let frame=json!({"choices":[{"index":0,"delta":{"tool_calls":calls},"finish_reason":null}]});
            let finish=json!({"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]});
            ResponseTemplate::new(200).insert_header("content-type","text/event-stream").set_body_string(format!("data: {frame}\n\ndata: {finish}\n\ndata: [DONE]\n\n"))
        } else {
            ResponseTemplate::new(200).set_body_json(json!({"id":"tools","object":"chat.completion","model":"dmai-bonsai-eval","choices":[{"index":0,"message":{"role":"assistant","content":null,"tool_calls":calls},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}))
        }
    }).mount(&server).await;
    let provider = OpenAICompatProvider::new(server.uri(), "fixture-key".into());
    let chunks = provider
        .stream_chat(request())
        .await
        .unwrap()
        .collect::<Vec<_>>()
        .await;
    let starts: Vec<_> = chunks
        .iter()
        .map(|chunk| chunk.as_ref().unwrap())
        .filter_map(|chunk| {
            if let ChatChunk::ToolCallStart { id, name } = chunk {
                Some((id.as_str(), name.as_str()))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        starts,
        vec![("call-first", "first"), ("call-second", "second")]
    );
    let arguments: Vec<_> = chunks
        .iter()
        .filter_map(|chunk| {
            if let Ok(ChatChunk::ToolCallArgsDelta { id, args_fragment }) = chunk {
                Some((
                    id.as_str(),
                    serde_json::from_str::<Value>(args_fragment).unwrap(),
                ))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        arguments,
        vec![
            ("call-first", json!({"name":"Мила"})),
            ("call-second", json!({"value":2}))
        ]
    );
    assert!(matches!(
        chunks.last(),
        Some(Ok(ChatChunk::Done {
            reason: FinishReason::ToolUse
        }))
    ));
}

async fn completed_response(
    message: Value,
    finish: Value,
) -> Result<Vec<ChatChunk>, adapter_llm::LlmError> {
    let server = MockServer::start().await;
    Mock::given(method("POST")).and(path("/chat/completions")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":"finish-test","object":"chat.completion","model":"dmai-bonsai-eval","choices":[{"index":0,"message":message,"finish_reason":finish}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}))).mount(&server).await;
    let provider = OpenAICompatProvider::new(server.uri(), "fixture-key".into());
    let stream = provider.stream_chat(request()).await?;
    let chunks = stream
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let requests = server.received_requests().await.unwrap();
    let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_ne!(body["stream"], true);
    assert!(body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["role"] == "system"
            && m["content"]
                .as_str()
                .is_some_and(|text| text.contains("CANONICAL DM CONTEXT"))));
    Ok(chunks)
}
fn call(id: &str) -> Value {
    json!({"id":id,"type":"function","function":{"name":"first","arguments":"{\"value\":1}"}})
}

#[tokio::test]
async fn truncated_filtered_or_unterminated_batches_do_not_execute_tools() {
    for finish in ["length", "content_filter"] {
        assert!(completed_response(json!({"role":"assistant","content":"unconfirmed success","tool_calls":[call("call-a")]}),json!(finish)).await.is_err());
    }
    for finish in [Value::Null, json!("cancelled")] {
        assert!(completed_response(
            json!({"role":"assistant","content":"not final","tool_calls":[call("call-a")]}),
            finish
        )
        .await
        .is_err());
    }
}
#[tokio::test]
async fn invalid_or_duplicate_tool_identity_is_rejected_before_any_chunk() {
    for calls in [vec![call("same"), call("same")], vec![call("")]] {
        assert!(completed_response(
            json!({"role":"assistant","content":null,"tool_calls":calls}),
            json!("tool_calls")
        )
        .await
        .is_err());
    }
    assert!(completed_response(
        json!({"role":"assistant","content":"no call"}),
        json!("tool_calls")
    )
    .await
    .is_err());
}
#[tokio::test]
async fn complete_text_and_reasoning_keep_the_existing_event_contract() {
    let chunks=completed_response(json!({"role":"assistant","content":"Confirmed narration","reasoning_content":"private reasoning"}),json!("stop")).await.unwrap();
    assert!(chunks.contains(&ChatChunk::TextDelta {
        text: "Confirmed narration".into()
    }));
    assert!(chunks.contains(&ChatChunk::ThinkingDelta {
        text: "private reasoning".into()
    }));
    assert_eq!(
        chunks.last(),
        Some(&ChatChunk::Done {
            reason: FinishReason::Stop
        })
    );
}
#[tokio::test]
async fn legacy_local_model_tool_markup_is_normalized_in_complete_rounds() {
    let chunks=completed_response(json!({"role":"assistant","content":"<|tool_call>call:first{value:1}<|tool_call>call:second{value:2}"}),json!("stop")).await.unwrap();
    let names: Vec<_> = chunks
        .iter()
        .filter_map(|chunk| {
            if let ChatChunk::ToolCallStart { name, .. } = chunk {
                Some(name.as_str())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(names, vec!["first", "second"]);
    assert!(!chunks
        .iter()
        .any(|chunk| matches!(chunk,ChatChunk::TextDelta {text} if text.contains("<|tool_call>"))));
    assert_eq!(
        chunks.last(),
        Some(&ChatChunk::Done {
            reason: FinishReason::ToolUse
        })
    );
}

#[tokio::test]
async fn mixed_native_calls_and_legacy_markup_do_not_duplicate_or_publish_tool_syntax() {
    let chunks=completed_response(json!({"role":"assistant","content":"Checking. <|tool_call>call:first{value:1}","tool_calls":[call("canonical-call")]}),json!("tool_calls")).await.unwrap();
    let starts: Vec<_> = chunks
        .iter()
        .filter_map(|chunk| {
            if let ChatChunk::ToolCallStart { id, .. } = chunk {
                Some(id.as_str())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(starts, vec!["canonical-call"]);
    assert!(chunks.contains(&ChatChunk::TextDelta {
        text: "Checking. ".into()
    }));
    assert!(!chunks
        .iter()
        .any(|chunk| matches!(chunk,ChatChunk::TextDelta {text} if text.contains("<|tool_call>"))));
}

#[tokio::test]
async fn waiting_complete_tool_response_is_cancellable_without_publishing_chunks() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).and(path("/chat/completions")).respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(2)).set_body_json(json!({"id":"slow","object":"chat.completion","model":"dmai-bonsai-eval","choices":[{"index":0,"message":{"role":"assistant","content":null,"tool_calls":[call("slow-call")]},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}))).mount(&server).await;
    let provider = OpenAICompatProvider::new(server.uri(), "fixture-key".into());
    let cancelled = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        provider.stream_chat(request()),
    )
    .await;
    assert!(cancelled.is_err());
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}
