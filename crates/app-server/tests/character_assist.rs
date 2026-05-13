//! Integration tests for POST /character/assist.

use app_llm::{ChatChunk, FinishReason, MockProvider};
use app_server::test_support::TestServer;
use serde_json::json;
use sqlx::SqlitePool;
use std::sync::Arc;

async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    app_server::db::init_db(&pool).await.unwrap();
    pool
}

fn empty_draft_json() -> serde_json::Value {
    json!({
        "classId": null, "subclassId": null,
        "raceId": null, "subraceId": null,
        "backgroundId": null, "abilityMethod": null,
        "abilities": { "str": 10, "dex": 10, "con": 10, "int": 10, "wis": 10, "cha": 10 },
        "abilityRollHistory": [], "pointBuyRemaining": 27,
        "skillProfs": [], "spells": { "cantrips": [], "level1": [] },
        "equipmentMode": null, "equipmentSlots": [], "equipmentInventory": [],
        "goldRemaining": 0, "personalityFlags": [],
        "ideals": "", "bonds": "", "flaws": "", "backstory": "",
        "name": "", "alignment": null,
        "portraitUrl": null, "portraitPrompt": null,
        "activeTab": "class"
    })
}

#[tokio::test]
async fn field_stream_emits_tokens_and_done() {
    let pool = test_pool().await;
    let mock = Arc::new(MockProvider::new(vec![
        ChatChunk::TextDelta { text: "Astarion ".into() },
        ChatChunk::TextDelta { text: "Ancunin".into() },
        ChatChunk::Done { reason: FinishReason::Stop },
    ]));
    let server = TestServer::start_with(mock, pool).await;
    let client = reqwest::Client::new();
    let resp = client
        .post(server.url("/character/assist"))
        .json(&json!({
            "kind": "field",
            "context": empty_draft_json(),
            "params": { "field": "name" },
            "locale": "en"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.bytes().await.unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.contains("event: token"));
    assert!(text.contains("Astarion"));
    assert!(text.contains("event: done"));
}

#[tokio::test]
async fn full_stream_emits_draft_patch_and_done() {
    let pool = test_pool().await;
    // Streaming tool-call: Start -> ArgsDelta(s) -> Done -> Done(reason)
    let json_args = r#"{"classId":"fighter","raceId":"human","backgroundId":"acolyte","name":"Roric"}"#;
    let mock = Arc::new(MockProvider::new(vec![
        ChatChunk::ToolCallStart {
            id: "call_1".into(),
            name: "apply_character_patch".into(),
        },
        ChatChunk::ToolCallArgsDelta {
            id: "call_1".into(),
            args_fragment: json_args.into(),
        },
        ChatChunk::ToolCallDone { id: "call_1".into() },
        ChatChunk::Done { reason: FinishReason::ToolUse },
    ]));
    let server = TestServer::start_with(mock, pool).await;
    let client = reqwest::Client::new();
    let resp = client
        .post(server.url("/character/assist"))
        .json(&json!({
            "kind": "full",
            "context": empty_draft_json(),
            "params": {},
            "locale": "en"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.bytes().await.unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.contains("event: draft_patch"));
    assert!(text.contains("\"classId\":\"fighter\""));
    assert!(text.contains("event: done"));
}

#[tokio::test]
async fn test_chat_stream_emits_tokens() {
    let pool = test_pool().await;
    let mock = Arc::new(MockProvider::new(vec![
        ChatChunk::TextDelta {
            text: "A grizzled barkeep eyes you. ".into(),
        },
        ChatChunk::TextDelta {
            text: "What brings you?".into(),
        },
        ChatChunk::Done { reason: FinishReason::Stop },
    ]));
    let server = TestServer::start_with(mock, pool).await;
    let client = reqwest::Client::new();
    let resp = client
        .post(server.url("/character/assist"))
        .json(&json!({
            "kind": "test_chat",
            "context": empty_draft_json(),
            "params": { "user_message": "Hello.", "history": [] },
            "locale": "en"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.bytes().await.unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.contains("event: token"));
    assert!(text.contains("grizzled barkeep"));
    assert!(text.contains("event: done"));
}

#[tokio::test]
async fn full_stream_invalid_tool_call_returns_error_event() {
    let pool = test_pool().await;
    let mock = Arc::new(MockProvider::new(vec![
        ChatChunk::ToolCallStart {
            id: "call_x".into(),
            name: "wrong_tool".into(),
        },
        ChatChunk::ToolCallDone { id: "call_x".into() },
        ChatChunk::Done { reason: FinishReason::Stop },
    ]));
    let server = TestServer::start_with(mock, pool).await;
    let client = reqwest::Client::new();
    let resp = client
        .post(server.url("/character/assist"))
        .json(&json!({
            "kind": "full",
            "context": empty_draft_json(),
            "params": {},
            "locale": "en"
        }))
        .send()
        .await
        .unwrap();
    let body = resp.bytes().await.unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.contains("event: error"));
    assert!(text.contains("invalid_patch"));
}
