//! Live integration test: Anthropic Haiku 4.5 emits a tool-call chunk.
//!
//! This test is `#[ignore]` by default - it requires a real `ANTHROPIC_API_KEY`
//! with credits. Run explicitly with:
//!
//! ```bash
//! ANTHROPIC_API_KEY=sk-... cargo test -p app-llm --test anthropic_tool_call_tests -- --ignored
//! ```
//!
//! Verifies that when a tool schema is registered and the prompt forces tool
//! use, the provider emits a `ChatChunk::ToolCallStart` (and at least one
//! `ToolCallArgsDelta` plus a `ToolCallDone`) before the final `Done`.

use app_llm::{
    AnthropicProvider, ChatChunk, ChatMessage, ChatRequest, FinishReason, LlmProvider, Tool,
};
use futures::StreamExt;
use serde_json::json;

#[tokio::test]
#[ignore = "requires live Anthropic key with credits"]
async fn anthropic_emits_tool_call_chunks_for_roll_dice() {
    let key = std::env::var("ANTHROPIC_API_KEY")
        .expect("ANTHROPIC_API_KEY must be set for live tool-call test");
    assert!(!key.trim().is_empty(), "ANTHROPIC_API_KEY is empty");

    let provider = AnthropicProvider::new(key);
    let roll_dice = Tool {
        name: "roll_dice".into(),
        description: "Roll dice in standard tabletop notation, e.g. 1d20 or 2d6+3.".into(),
        parameters: json!({
            "type": "object",
            "properties": {
                "dice": {
                    "type": "string",
                    "description": "Dice expression like 1d20 or 2d6+3"
                }
            },
            "required": ["dice"]
        }),
    };

    let req = ChatRequest {
        messages: vec![
            ChatMessage::System {
                content: "You are a dungeon master. When the user asks for an attack roll, you MUST call the roll_dice tool. Do not narrate; just call the tool.".into(),
            },
            ChatMessage::User {
                content: "Roll a d20 for my attack.".into(),
            },
        ],
        model: "claude-haiku-4-5-20251001".into(),
        max_tokens: Some(256),
        temperature: Some(0.0),
        tools: vec![roll_dice],
        system_prompt: None,
    };

    let mut stream = provider.stream_chat(req).await.expect("stream opens");
    let mut saw_start = false;
    let mut saw_args_delta = false;
    let mut saw_done_for_call = false;
    let mut started_id: Option<String> = None;
    let mut accumulated_args = String::new();
    let mut final_reason: Option<FinishReason> = None;

    while let Some(chunk) = stream.next().await {
        match chunk.expect("chunk") {
            ChatChunk::ToolCallStart { id, name } => {
                assert_eq!(name, "roll_dice", "expected roll_dice tool-call start");
                started_id = Some(id);
                saw_start = true;
            }
            ChatChunk::ToolCallArgsDelta { id, args_fragment } => {
                assert_eq!(
                    Some(&id),
                    started_id.as_ref(),
                    "args delta id must match the started tool-call id"
                );
                accumulated_args.push_str(&args_fragment);
                saw_args_delta = true;
            }
            ChatChunk::ToolCallDone { id } => {
                assert_eq!(
                    Some(&id),
                    started_id.as_ref(),
                    "done id must match the started tool-call id"
                );
                saw_done_for_call = true;
            }
            ChatChunk::Done { reason } => {
                final_reason = Some(reason);
            }
            ChatChunk::TextDelta { .. } => {
                // Allowed - some models emit a small preamble before the tool call.
            }
        }
    }

    assert!(saw_start, "expected at least one ToolCallStart");
    assert!(saw_args_delta, "expected at least one ToolCallArgsDelta");
    assert!(saw_done_for_call, "expected ToolCallDone for the call");
    assert!(
        accumulated_args.contains("dice"),
        "expected the accumulated args JSON to mention the 'dice' field, got: {accumulated_args:?}"
    );
    assert_eq!(
        final_reason,
        Some(FinishReason::ToolUse),
        "expected final FinishReason::ToolUse"
    );
}
