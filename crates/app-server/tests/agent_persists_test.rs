//! E.2 smoke tests: /agent/turn persists user, assistant, and (when present)
//! assistant_with_tool_calls + tool_result rows.

use std::sync::Arc;

use app_llm::{ChatChunk, ChatMessage, FinishReason, MockProvider};
use app_server::db;
use app_server::test_support::TestServer;
use reqwest::Client;

const SESSION_ID: &str = "00000000-0000-0000-0000-000000000002";
const CAMPAIGN_ID: &str = "00000000-0000-0000-0000-000000000001";

#[tokio::test]
async fn agent_turn_persists_user_and_assistant() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    db::init_db(&pool).await.unwrap();
    let mock = Arc::new(MockProvider::new(vec![
        ChatChunk::TextDelta {
            text: "You hear footsteps".into(),
        },
        ChatChunk::Done {
            reason: FinishReason::Stop,
        },
    ]));
    let server = TestServer::start_with(mock, pool.clone()).await;

    let resp = Client::new()
        .post(server.url("/agent/turn"))
        .header("content-type", "application/json")
        .body(format!(
            r#"{{"player_message":"I listen","history":[],"campaign_id":"{CAMPAIGN_ID}","session_id":"{SESSION_ID}"}}"#
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let _ = resp.text().await.unwrap();

    // The persistence side-effect spawns tokio tasks; let them settle.
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    let history = db::list_messages_by_session(&pool, SESSION_ID).await.unwrap();
    assert!(history.len() >= 2, "expected user+assistant rows, got {history:?}");
    assert!(matches!(&history[0], ChatMessage::User { .. }));
    let has_assistant = history
        .iter()
        .any(|m| matches!(m, ChatMessage::Assistant { content } if content.contains("footsteps")));
    assert!(has_assistant, "no assistant row with narration; got {history:?}");
}

#[tokio::test]
async fn agent_turn_persists_assistant_with_tool_calls_and_tool_result() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    db::init_db(&pool).await.unwrap();

    // Round 1: tool call. Round 2: stop (no second LLM round simulated; the
    // orchestrator should emit AgentDone after the single tool execution).
    let mock = Arc::new(MockProvider::new(vec![
        ChatChunk::TextDelta {
            text: "Rolling: ".into(),
        },
        ChatChunk::ToolCallStart {
            id: "c1".into(),
            name: "roll_dice".into(),
        },
        ChatChunk::ToolCallArgsDelta {
            id: "c1".into(),
            args_fragment: r#"{"dice":"1d20","reason":"perception"}"#.into(),
        },
        ChatChunk::ToolCallDone { id: "c1".into() },
        ChatChunk::Done {
            reason: FinishReason::ToolUse,
        },
    ]));
    let server = TestServer::start_with(mock, pool.clone()).await;

    let resp = Client::new()
        .post(server.url("/agent/turn"))
        .header("content-type", "application/json")
        .body(format!(
            r#"{{"player_message":"I scan the room","history":[],"campaign_id":"{CAMPAIGN_ID}","session_id":"{SESSION_ID}"}}"#
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let _ = resp.text().await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let history = db::list_messages_by_session(&pool, SESSION_ID).await.unwrap();
    assert!(matches!(&history[0], ChatMessage::User { .. }));
    let has_awtc = history
        .iter()
        .any(|m| matches!(m, ChatMessage::AssistantWithToolCalls { .. }));
    let has_tool_result = history
        .iter()
        .any(|m| matches!(m, ChatMessage::ToolResult(_)));
    assert!(
        has_awtc,
        "no AssistantWithToolCalls row; got {} rows",
        history.len()
    );
    assert!(has_tool_result, "no ToolResult row; got {} rows", history.len());
}
