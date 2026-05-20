//! Live integration test: Anthropic Haiku 4.5 streams a real text response.
//!
//! This test is `#[ignore]` by default - it requires a real `ANTHROPIC_API_KEY`
//! with credits. Run explicitly with:
//!
//! ```bash
//! ANTHROPIC_API_KEY=sk-... cargo test -p app-llm --test anthropic_integration -- --ignored
//! ```
//!
//! Verifies that a streamed chat request yields `TextDelta` chunks ending in a
//! `Done`, with the model reply containing the expected token.

use app_llm::{AnthropicProvider, ChatChunk, ChatMessage, ChatRequest, LlmProvider};
use futures::StreamExt;

fn skip_unless_env() -> Option<String> {
    std::env::var("ANTHROPIC_API_KEY")
        .ok()
        .filter(|k| !k.trim().is_empty())
}

#[tokio::test]
#[ignore = "requires live Anthropic key with credits"]
async fn anthropic_streams_real_response_when_key_present() {
    let Some(key) = skip_unless_env() else {
        eprintln!("skipped: ANTHROPIC_API_KEY not set");
        return;
    };

    let provider = AnthropicProvider::new(key);
    let req = ChatRequest {
        messages: vec![ChatMessage::user_text("Reply with exactly the word: pong")],
        model: "claude-haiku-4-5-20251001".into(),
        max_tokens: Some(10),
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
            ChatChunk::Done { .. } => saw_done = true,
            ChatChunk::ThinkingDelta { .. } => {
                // Thinking content may appear with reasoning-capable models; ignore here.
            }
            ChatChunk::ToolCallStart { .. }
            | ChatChunk::ToolCallArgsDelta { .. }
            | ChatChunk::ToolCallDone { .. } => {
                panic!("unexpected tool-call chunk in legacy text-only test");
            }
        }
    }

    assert!(saw_done, "stream did not send Done");
    assert!(
        text.to_lowercase().contains("pong"),
        "expected 'pong' in response, got: {text:?}"
    );
}
