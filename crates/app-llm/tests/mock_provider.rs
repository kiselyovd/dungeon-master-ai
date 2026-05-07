use app_llm::{ChatChunk, ChatMessage, ChatRequest, FinishReason, LlmProvider, MockProvider};
use futures::StreamExt;

#[tokio::test]
async fn mock_provider_streams_scripted_text_then_done() {
    let provider = MockProvider::new(vec![
        ChatChunk::TextDelta { text: "Hello".into() },
        ChatChunk::TextDelta { text: ", world".into() },
        ChatChunk::Done { reason: FinishReason::Stop },
    ]);

    let req = ChatRequest {
        messages: vec![ChatMessage::User { content: "hi".into() }],
        model: "mock".into(),
        max_tokens: None,
        temperature: None,
        tools: Vec::new(),
        system_prompt: None,
    };

    let mut stream = provider.stream_chat(req).await.expect("stream opens");
    let mut collected = Vec::new();
    while let Some(chunk) = stream.next().await {
        collected.push(chunk.expect("chunk"));
    }

    assert_eq!(collected.len(), 3);
    assert!(matches!(collected[0], ChatChunk::TextDelta { ref text } if text == "Hello"));
    assert!(matches!(collected[2], ChatChunk::Done { reason: FinishReason::Stop }));
}

#[tokio::test]
async fn mock_provider_name_is_mock() {
    let provider = MockProvider::new(vec![]);
    assert_eq!(provider.name(), "mock");
}
