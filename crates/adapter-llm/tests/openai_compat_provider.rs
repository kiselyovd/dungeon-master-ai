//! Integration test for OpenAICompatProvider against a wiremock server that
//! mimics the OpenAI Chat Completions streaming API.
//!
//! This is the protocol used by LM Studio, Ollama (`/v1/`), llama.cpp server,
//! vLLM, mistral.rs server, OpenRouter, Groq, DeepSeek, Together, etc.

use adapter_llm::{
    ChatChunk, ChatMessage, ChatRequest, FinishReason, LlmProvider, OpenAICompatProvider,
};
use futures::StreamExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn sse_body() -> String {
    // Two content deltas + a finish event + the [DONE] sentinel.
    [
        r#"data: {"choices":[{"delta":{"content":"Hello"},"index":0,"finish_reason":null}]}"#,
        r#"data: {"choices":[{"delta":{"content":", world"},"index":0,"finish_reason":null}]}"#,
        r#"data: {"choices":[{"delta":{},"index":0,"finish_reason":"stop"}]}"#,
        r#"data: [DONE]"#,
        "",
    ]
    .join("\n\n")
}

#[tokio::test]
async fn fragmented_native_tool_call_keeps_its_id_and_exact_arguments() {
    let server = MockServer::start().await;
    let frames = [
        serde_json::json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"native-call","type":"function","function":{"name":"adventure_action","arguments":""}}]},"finish_reason":null}]}),
        serde_json::json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"action\":"}}]},"finish_reason":null}]}),
        serde_json::json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"kind\":\"move\","}}]},"finish_reason":null}]}),
        serde_json::json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"location\":\"quay\"}}"}}]},"finish_reason":null}]}),
        serde_json::json!({"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}),
    ];
    let body = frames
        .iter()
        .map(|frame| format!("data: {frame}\n\n"))
        .collect::<String>()
        + "data: [DONE]\n\n";
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(body),
        )
        .mount(&server)
        .await;
    let provider = OpenAICompatProvider::new(server.uri(), "fixture-key".into());
    let mut stream = provider
        .stream_chat(ChatRequest {
            messages: vec![ChatMessage::user_text("Иду на причал")],
            model: "dmai-bonsai-eval".into(),
            max_tokens: Some(256),
            temperature: Some(1.0),
            tools: vec![],
            system_prompt: None,
            reasoning: None,
        })
        .await
        .unwrap();
    let mut starts = vec![];
    let mut arguments = String::new();
    let mut ends = vec![];
    while let Some(chunk) = stream.next().await {
        match chunk.unwrap() {
            ChatChunk::ToolCallStart { id, name } => starts.push((id, name)),
            ChatChunk::ToolCallArgsDelta { id, args_fragment } => {
                assert_eq!(id, "native-call");
                arguments.push_str(&args_fragment)
            }
            ChatChunk::ToolCallDone { id } => ends.push(id),
            _ => {}
        }
    }
    assert_eq!(
        starts,
        vec![("native-call".into(), "adventure_action".into())]
    );
    assert_eq!(ends, vec!["native-call"]);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&arguments).unwrap(),
        serde_json::json!({"action":{"kind":"move","location":"quay"}})
    );
}

#[tokio::test]
async fn openai_compat_streams_text_then_done() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_body()),
        )
        .mount(&server)
        .await;

    let provider = OpenAICompatProvider::new(server.uri(), "sk-test".into());
    let req = ChatRequest {
        messages: vec![ChatMessage::user_text("hi")],
        model: "qwen3-1.7b".into(),
        max_tokens: Some(32),
        temperature: Some(0.0),
        tools: Vec::new(),
        system_prompt: None,
        reasoning: None,
    };

    let mut stream = provider.stream_chat(req).await.expect("stream opens");
    let mut text = String::new();
    let mut saw_done = false;
    while let Some(chunk) = stream.next().await {
        match chunk.expect("chunk") {
            ChatChunk::TextDelta { text: t } => text.push_str(&t),
            ChatChunk::Done {
                reason: FinishReason::Stop,
            } => {
                saw_done = true;
            }
            ChatChunk::Done { .. } => {
                saw_done = true;
            }
            ChatChunk::ThinkingDelta { .. } => {
                // Allowed - reasoning models may emit thinking content.
            }
            ChatChunk::ToolCallStart { .. }
            | ChatChunk::ToolCallArgsDelta { .. }
            | ChatChunk::ToolCallDone { .. } => {
                panic!("unexpected tool-call chunk in legacy text-only test");
            }
        }
    }

    assert_eq!(
        text, "Hello, world",
        "unexpected accumulated text: {text:?}"
    );
    assert!(saw_done, "stream did not emit Done");
}

#[tokio::test]
async fn interleaved_tool_arguments_remain_separate_with_or_without_reasoning() {
    let server = MockServer::start().await;
    let mut body = String::new();
    for (index, id, name, args) in [
        (0, Some("first"), Some("query_rules"), "{\"query\":\""),
        (1, Some("second"), Some("query_rules"), "{\"query\":\""),
        (0, None, None, "а"),
        (1, None, None, "б"),
        (0, None, None, "а\"}"),
        (1, None, None, "б\"}"),
    ] {
        let mut call = serde_json::json!({"index":index,"function":{"arguments":args}});
        if let Some(id) = id {
            call["id"] = id.into();
        }
        if let Some(name) = name {
            call["function"]["name"] = name.into();
        }
        body.push_str(&format!("data: {}\n\n",serde_json::json!({"choices":[{"index":0,"delta":{"tool_calls":[call]},"finish_reason":null}]})));
    }
    body.push_str("data: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"tool_calls\"}]}\n\ndata: [DONE]\n\n");
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(body),
        )
        .mount(&server)
        .await;
    let provider = OpenAICompatProvider::new(server.uri(), "fixture-key".into());
    for reasoning in [None, Some(adapter_llm::ReasoningSpec::Medium)] {
        let mut stream = provider
            .stream_chat(ChatRequest {
                messages: vec![ChatMessage::user_text("Два вопроса")],
                model: "dmai-bonsai-eval".into(),
                max_tokens: Some(256),
                temperature: Some(1.0),
                tools: vec![],
                system_prompt: None,
                reasoning,
            })
            .await
            .unwrap();
        let mut arguments = std::collections::BTreeMap::<String, String>::new();
        let mut starts = vec![];
        let mut ends = vec![];
        while let Some(chunk) = stream.next().await {
            match chunk.unwrap() {
                ChatChunk::ToolCallStart { id, name } => {
                    assert_eq!(name, "query_rules");
                    starts.push(id);
                }
                ChatChunk::ToolCallArgsDelta { id, args_fragment } => {
                    arguments.entry(id).or_default().push_str(&args_fragment)
                }
                ChatChunk::ToolCallDone { id } => ends.push(id),
                _ => {}
            }
        }
        assert_eq!(starts, vec!["first", "second"]);
        assert_eq!(ends, starts);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&arguments["first"]).unwrap(),
            serde_json::json!({"query":"аа"})
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&arguments["second"]).unwrap(),
            serde_json::json!({"query":"бб"})
        );
    }
}

#[tokio::test]
async fn openai_compat_provider_name_is_openai_compat() {
    let provider = OpenAICompatProvider::new("http://localhost:1234".into(), "sk-test".into());
    assert_eq!(provider.name(), "openai-compat");
}

#[tokio::test]
async fn dm_system_context_is_present_in_the_actual_wire_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_body()),
        )
        .mount(&server)
        .await;
    let provider = OpenAICompatProvider::new(server.uri(), "fixture-key".into());
    let mut stream=provider.stream_chat(ChatRequest{messages:vec![ChatMessage::user_text("Иду на причал")],model:"dmai-bonsai-eval".into(),max_tokens:Some(256),temperature:Some(1.0),tools:vec![],system_prompt:Some("CANONICAL HERO: current location town; available move quay; narrate only confirmed results".into()),reasoning:None}).await.unwrap();
    while let Some(chunk) = stream.next().await {
        chunk.unwrap();
    }
    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert!(
        body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|message| message["role"] == "system"
                && message["content"]
                    .as_str()
                    .is_some_and(|text| text.contains("CANONICAL HERO"))),
        "The model must receive the server's authored context"
    );
}

#[tokio::test]
async fn real_stream_terminal_metadata_cannot_turn_truncated_verifier_json_into_permission() {
    use app_application::agent::intentions::{
        LlmPlayerIntentionVerifier, PlayerIntentionCheck, PlayerIntentionVerifier,
    };
    use std::sync::Arc;
    for (reason, valid) in [
        (serde_json::json!("stop"), true),
        (serde_json::json!("length"), false),
        (serde_json::json!("content_filter"), false),
        (serde_json::Value::Null, false),
    ] {
        let server = MockServer::start().await;
        let content = serde_json::json!({"choices":[{"index":0,"delta":{"content":"{\"allowed\":true,\"further_requested\":false}"},"finish_reason":null}]});
        let finish = serde_json::json!({"choices":[{"index":0,"delta":{},"finish_reason":reason}]});
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(format!(
                        "data: {content}\n\ndata: {finish}\n\ndata: [DONE]\n\n"
                    )),
            )
            .mount(&server)
            .await;
        let provider = Arc::new(OpenAICompatProvider::new(server.uri(), "fixture".into()));
        let decision = LlmPlayerIntentionVerifier::new(provider, "dmai-qwen9b-eval".into())
            .assess(PlayerIntentionCheck {
                player_message: "Move to quay".into(),
                candidate: Some((
                    "adventure_action".into(),
                    serde_json::json!({"action":{"kind":"move","location":"quay"}}),
                )),
                resolved: vec![],
                conversation: vec![],
                public_context: serde_json::json!({}),
            })
            .await;
        assert_eq!(decision.is_ok(), valid, "finish reason {reason}");
    }
}
