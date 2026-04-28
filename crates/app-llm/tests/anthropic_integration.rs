use app_llm::{AnthropicProvider, ChatChunk, ChatMessage, ChatRequest, LlmProvider};
use futures::StreamExt;

fn skip_unless_env() -> Option<String> {
    std::env::var("ANTHROPIC_API_KEY")
        .ok()
        .filter(|k| !k.trim().is_empty())
}

#[tokio::test]
async fn anthropic_streams_real_response_when_key_present() {
    let Some(key) = skip_unless_env() else {
        eprintln!("skipped: ANTHROPIC_API_KEY not set");
        return;
    };

    let provider = AnthropicProvider::new(key);
    let req = ChatRequest {
        messages: vec![ChatMessage::User {
            content: "Reply with exactly the word: pong".into(),
        }],
        model: "claude-haiku-4-5-20251001".into(),
        max_tokens: Some(10),
        temperature: Some(0.0),
    };

    let mut stream = provider.stream_chat(req).await.expect("stream opens");
    let mut text = String::new();
    let mut saw_done = false;
    while let Some(chunk) = stream.next().await {
        match chunk.expect("chunk") {
            ChatChunk::TextDelta { text: t } => text.push_str(&t),
            ChatChunk::Done { .. } => saw_done = true,
        }
    }

    assert!(saw_done, "stream did not send Done");
    assert!(
        text.to_lowercase().contains("pong"),
        "expected 'pong' in response, got: {text:?}"
    );
}
