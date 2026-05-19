//! Integration tests for POST /character/assist personality_flag path.

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
async fn flag_stream_with_slot_and_pool_emits_tokens_and_done() {
    let pool = test_pool().await;
    let mock = Arc::new(MockProvider::new(vec![
        ChatChunk::TextDelta {
            text: "I idolize ".into(),
        },
        ChatChunk::TextDelta {
            text: "my faith's quiet martyr.".into(),
        },
        ChatChunk::Done {
            reason: FinishReason::Stop,
        },
    ]));
    let server = TestServer::start_with(mock, pool).await;
    let client = reqwest::Client::new();
    let resp = client
        .post(server.url("/character/assist"))
        .json(&json!({
            "kind": "field",
            "context": empty_draft_json(),
            "params": {
                "field": "personality_flag",
                "slot_id": "bg-trait",
                "source": "background",
                "source_label": "Acolyte",
                "pool": [
                    "I idolize a particular hero of my faith.",
                    "I see omens in every event."
                ]
            },
            "locale": "en"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let text = String::from_utf8(resp.bytes().await.unwrap().to_vec()).unwrap();
    assert!(text.contains("event: token"));
    assert!(text.contains("idolize"));
    assert!(text.contains("event: done"));
}

#[tokio::test]
async fn flag_stream_with_empty_pool_still_succeeds() {
    let pool = test_pool().await;
    let mock = Arc::new(MockProvider::new(vec![
        ChatChunk::TextDelta {
            text: "A fresh entry.".into(),
        },
        ChatChunk::Done {
            reason: FinishReason::Stop,
        },
    ]));
    let server = TestServer::start_with(mock, pool).await;
    let client = reqwest::Client::new();
    let resp = client
        .post(server.url("/character/assist"))
        .json(&json!({
            "kind": "field",
            "context": empty_draft_json(),
            "params": {
                "field": "personality_flag",
                "slot_id": "race-quirk",
                "source": "race",
                "pool": []
            },
            "locale": "en"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let text = String::from_utf8(resp.bytes().await.unwrap().to_vec()).unwrap();
    assert!(text.contains("event: token"));
    assert!(text.contains("event: done"));
}

#[tokio::test]
async fn flag_request_missing_slot_id_returns_400() {
    let pool = test_pool().await;
    let mock = Arc::new(MockProvider::new(vec![ChatChunk::Done {
        reason: FinishReason::Stop,
    }]));
    let server = TestServer::start_with(mock, pool).await;
    let client = reqwest::Client::new();
    let resp = client
        .post(server.url("/character/assist"))
        .json(&json!({
            "kind": "field",
            "context": empty_draft_json(),
            "params": { "field": "personality_flag" },
            "locale": "en"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
    let body = resp.text().await.unwrap();
    assert!(body.contains("missing_slot_id"));
}
