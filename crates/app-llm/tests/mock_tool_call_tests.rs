use app_llm::{ChatChunk, ChatMessage, ChatRequest, FinishReason, LlmProvider, MockProvider};

#[tokio::test]
async fn mock_emits_tool_call_then_text() {
    use futures::StreamExt;
    let chunks = vec![
        ChatChunk::ToolCallStart {
            id: "c1".into(),
            name: "roll_dice".into(),
        },
        ChatChunk::ToolCallArgsDelta {
            id: "c1".into(),
            args_fragment: r#"{"dice":"1d20"}"#.into(),
        },
        ChatChunk::ToolCallDone { id: "c1".into() },
        ChatChunk::Done {
            reason: FinishReason::ToolUse,
        },
    ];
    let mock = MockProvider::new(chunks);
    let req = ChatRequest {
        messages: vec![ChatMessage::User {
            content: "attack".into(),
        }],
        model: "mock".into(),
        max_tokens: None,
        temperature: None,
        tools: vec![],
        system_prompt: None,
    };
    let mut stream = mock.stream_chat(req).await.unwrap();
    let mut names = vec![];
    while let Some(chunk) = stream.next().await {
        match chunk.unwrap() {
            ChatChunk::ToolCallStart { name, .. } => names.push(name),
            ChatChunk::Done { .. } => break,
            _ => {}
        }
    }
    assert_eq!(names, vec!["roll_dice"]);
}

#[tokio::test]
async fn mock_set_chunks_replaces_queue_for_round_two() {
    use futures::StreamExt;
    let mock = MockProvider::new(vec![ChatChunk::Done {
        reason: FinishReason::Stop,
    }]);

    // Round 1: drain whatever was scripted (just Done).
    let req1 = ChatRequest {
        messages: vec![],
        model: "mock".into(),
        max_tokens: None,
        temperature: None,
        tools: vec![],
        system_prompt: None,
    };
    let mut s1 = mock.stream_chat(req1).await.unwrap();
    while s1.next().await.is_some() {}

    // Round 2: load fresh chunks via set_chunks.
    mock.set_chunks(vec![
        ChatChunk::TextDelta {
            text: "round two".into(),
        },
        ChatChunk::Done {
            reason: FinishReason::Stop,
        },
    ]);
    let req2 = ChatRequest {
        messages: vec![],
        model: "mock".into(),
        max_tokens: None,
        temperature: None,
        tools: vec![],
        system_prompt: None,
    };
    let mut s2 = mock.stream_chat(req2).await.unwrap();
    let mut text = String::new();
    while let Some(c) = s2.next().await {
        if let Ok(ChatChunk::TextDelta { text: t }) = c {
            text.push_str(&t);
        }
    }
    assert_eq!(text, "round two");
}
