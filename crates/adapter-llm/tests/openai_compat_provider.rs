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
        let provider = Arc::new(adapter_llm::retry::RetryableProvider::new(Arc::new(
            OpenAICompatProvider::new(server.uri(), "fixture".into()),
        )));
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
        let requests = server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["response_format"]["type"], "json_schema");
        let spec = &body["response_format"]["json_schema"];
        assert_eq!(spec["name"], "player_intention");
        assert_eq!(spec["strict"], true);
        assert_eq!(spec["schema"]["additionalProperties"], false);
        assert_eq!(spec["schema"]["properties"]["allowed"]["type"], "boolean");
        assert_eq!(
            spec["schema"]["properties"]["further_requested"]["type"],
            "boolean"
        );
        assert!(body["tools"].is_null());
    }
}

#[tokio::test]
async fn no_candidate_schema_cannot_grant_permission_even_when_provider_ignores_it() {
    use app_application::agent::intentions::{
        LlmPlayerIntentionVerifier, PlayerIntentionCheck, PlayerIntentionVerifier,
    };
    use std::sync::Arc;
    for (reply, valid, pending) in [
        (serde_json::json!({"remaining_commands":[]}), true, false),
        (
            serde_json::json!({"remaining_commands":["Иду в архив"]}),
            true,
            true,
        ),
        (
            serde_json::json!({"allowed":true,"further_requested":false}),
            false,
            false,
        ),
        (
            serde_json::json!({"remaining_commands":["Атакую врага"]}),
            false,
            false,
        ),
        (serde_json::json!({"remaining_commands":[""]}), false, false),
        (
            serde_json::json!({"remaining_commands":vec!["Иду в архив";17]}),
            false,
            false,
        ),
    ] {
        let server = MockServer::start().await;
        let text = reply.to_string();
        let content = serde_json::json!({"choices":[{"index":0,"delta":{"content":text},"finish_reason":null}]});
        let finish = serde_json::json!({"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]});
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
        let result = LlmPlayerIntentionVerifier::new(provider, "fixture-model".into())
            .assess(PlayerIntentionCheck {
                player_message: "Иду в архив".into(),
                candidate: None,
                resolved: vec![],
                conversation: vec![],
                public_context: serde_json::json!({}),
            })
            .await;
        assert_eq!(result.is_ok(), valid, "{reply}");
        if valid {
            let decision = result.unwrap();
            assert!(!decision.allowed);
            assert_eq!(decision.further_requested, pending);
        }
        let requests = server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(
            body["response_format"]["json_schema"]["schema"]["properties"]["remaining_commands"]
                ["items"]["type"],
            "string"
        );
        let input: serde_json::Value = serde_json::from_str(
            body["messages"]
                .as_array()
                .unwrap()
                .iter()
                .find(|message| message["role"] == "user")
                .unwrap()["content"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        assert!(input["public_context"].is_null());
        assert!(input["completed_attempts"].is_array());
    }
}

#[tokio::test]
async fn local_provider_preserves_structured_format_and_its_default_model_alias() {
    use adapter_llm::mistralrs_provider::MistralrsLocalProvider;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_body()),
        )
        .mount(&server)
        .await;
    let provider = MistralrsLocalProvider::new(server.address().port(), "configured-model".into());
    let schema = serde_json::json!({"type":"object","properties":{"ok":{"type":"boolean"}},"required":["ok"],"additionalProperties":false});
    let mut stream = provider
        .stream_chat_json(
            ChatRequest {
                messages: vec![ChatMessage::user_text("Reply as JSON")],
                model: "caller-model".into(),
                max_tokens: Some(128),
                temperature: Some(0.0),
                tools: vec![],
                system_prompt: None,
                reasoning: None,
            },
            schema.clone(),
        )
        .await
        .unwrap();
    while let Some(chunk) = stream.next().await {
        chunk.unwrap();
    }
    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["model"], "default");
    assert_eq!(body["response_format"]["json_schema"]["schema"], schema);
}

#[tokio::test]
async fn structured_schema_survives_each_retry_of_a_tool_enabled_wire_request() {
    use std::sync::Arc;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_json(serde_json::json!({"error":{"message":"fixture rate limit"}})),
        )
        .with_priority(1)
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices":[{"index":0,"message":{"role":"assistant","content":"{\"ok\":true}"},"finish_reason":"stop"}]
        })))
        .with_priority(5)
        .mount(&server)
        .await;
    let provider = adapter_llm::retry::RetryableProvider::with_policy(
        Arc::new(OpenAICompatProvider::new(server.uri(), "fixture".into())),
        backon::ExponentialBuilder::default()
            .with_min_delay(std::time::Duration::from_millis(1))
            .with_max_times(1),
    );
    let schema = serde_json::json!({"type":"object","properties":{"ok":{"type":"boolean"}},"required":["ok"],"additionalProperties":false});
    let mut stream = provider
        .stream_chat_json(
            ChatRequest {
                messages: vec![ChatMessage::user_text("Return JSON")],
                model: "fixture-model".into(),
                max_tokens: Some(128),
                temperature: Some(0.0),
                tools: vec![adapter_llm::Tool {
                    name: "query_rules".into(),
                    description: "fixture".into(),
                    parameters: serde_json::json!({"type":"object","properties":{}}),
                }],
                system_prompt: None,
                reasoning: None,
            },
            schema.clone(),
        )
        .await
        .unwrap();
    while let Some(chunk) = stream.next().await {
        chunk.unwrap();
    }
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    for request in requests {
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body["response_format"]["json_schema"]["schema"], schema);
        assert_eq!(body["response_format"]["json_schema"]["strict"], true);
    }
}
