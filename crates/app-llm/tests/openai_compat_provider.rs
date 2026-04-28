//! Integration test for OpenAICompatProvider against a wiremock server that
//! mimics the OpenAI Chat Completions streaming API.
//!
//! This is the protocol used by LM Studio, Ollama (`/v1/`), llama.cpp server,
//! vLLM, mistral.rs server, OpenRouter, Groq, DeepSeek, Together, etc.

use app_llm::{ChatChunk, ChatMessage, ChatRequest, FinishReason, LlmProvider, OpenAICompatProvider};
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
        messages: vec![ChatMessage::User {
            content: "hi".into(),
        }],
        model: "qwen3-1.7b".into(),
        max_tokens: Some(32),
        temperature: Some(0.0),
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
        }
    }

    assert_eq!(text, "Hello, world", "unexpected accumulated text: {text:?}");
    assert!(saw_done, "stream did not emit Done");
}

#[tokio::test]
async fn openai_compat_provider_name_is_openai_compat() {
    let provider = OpenAICompatProvider::new("http://localhost:1234".into(), "sk-test".into());
    assert_eq!(provider.name(), "openai-compat");
}
