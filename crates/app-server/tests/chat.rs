use std::sync::Arc;

use app_llm::{ChatChunk, FinishReason, MockProvider};
use app_server::test_support::TestServer;
use futures::StreamExt;
use reqwest::Client;
use serde_json::json;

#[tokio::test]
async fn chat_streams_sse_text_deltas_then_done() {
    let provider = Arc::new(MockProvider::new(vec![
        ChatChunk::TextDelta {
            text: "Hello".into(),
        },
        ChatChunk::TextDelta {
            text: ", world".into(),
        },
        ChatChunk::Done {
            reason: FinishReason::Stop,
        },
    ]));
    let server = TestServer::start_with(provider).await;

    let body = json!({
        "messages": [{"role": "user", "content": "hi"}],
        "model": "mock"
    });

    let resp = Client::new()
        .post(server.url("/chat"))
        .json(&body)
        .send()
        .await
        .expect("post");
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get("content-type")
            .expect("ct")
            .to_str()
            .unwrap(),
        "text/event-stream"
    );

    let mut text = String::new();
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        text.push_str(&String::from_utf8_lossy(&chunk.expect("bytes")));
    }

    assert!(text.contains("event: text_delta"), "raw stream:\n{text}");
    assert!(text.contains("\"text\":\"Hello\""), "raw stream:\n{text}");
    assert!(
        text.contains("\"text\":\", world\""),
        "raw stream:\n{text}"
    );
    assert!(text.contains("event: done"), "raw stream:\n{text}");
}

#[tokio::test]
async fn chat_returns_422_when_messages_missing() {
    let server = TestServer::start().await;

    let body = json!({"model": "mock"});
    let resp = Client::new()
        .post(server.url("/chat"))
        .json(&body)
        .send()
        .await
        .expect("post");
    assert_eq!(resp.status(), 422);
}
