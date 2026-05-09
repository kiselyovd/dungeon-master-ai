//! M5 P2.13: integration tests for the Saves API.
//!
//! Covers the round trip the UI exercises (create -> list -> load ->
//! delete) plus the quick-save shortcut and the validation error paths
//! (unknown kind / unknown tag).

use std::sync::Arc;

use app_llm::MockProvider;
use app_server::db;
use app_server::test_support::TestServer;
use reqwest::Client;
use serde_json::{Value, json};

#[tokio::test]
async fn save_round_trip_create_list_load_delete() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    db::init_db(&pool).await.unwrap();
    let server = TestServer::start_with(Arc::new(MockProvider::new(vec![])), pool).await;

    let session_id = uuid::Uuid::new_v4().to_string();
    let client = Client::new();

    // Create a manual save.
    let create = client
        .post(server.url(&format!("/sessions/{session_id}/saves")))
        .json(&json!({
            "kind": "manual",
            "title": "Before the boss",
            "summary": "Party rests outside the lair.",
            "tag": "exploration",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create.status(), 201, "create save status");
    let created: Value = create.json().await.unwrap();
    let save_id = created["id"].as_str().unwrap().to_string();

    // List saves: must contain the new one.
    let list = client
        .get(server.url(&format!("/sessions/{session_id}/saves")))
        .send()
        .await
        .unwrap();
    assert_eq!(list.status(), 200);
    let listed: Vec<Value> = list.json().await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["title"], "Before the boss");
    assert_eq!(listed[0]["kind"], "manual");
    assert_eq!(listed[0]["tag"], "exploration");
    assert!(
        listed[0].get("game_state").is_none(),
        "list shape must not embed game_state"
    );

    // Load the save by id - the full envelope must be present.
    let load = client
        .get(server.url(&format!("/saves/{save_id}")))
        .send()
        .await
        .unwrap();
    assert_eq!(load.status(), 200);
    let loaded: Value = load.json().await.unwrap();
    assert_eq!(loaded["title"], "Before the boss");
    assert_eq!(loaded["game_state"]["schema_version"], 1);
    assert_eq!(loaded["game_state"]["state"]["title"], "Before the boss");

    // Delete the save - 204 No Content.
    let del = client
        .delete(server.url(&format!("/saves/{save_id}")))
        .send()
        .await
        .unwrap();
    assert_eq!(del.status(), 204);

    // List is empty again.
    let list2: Vec<Value> = client
        .get(server.url(&format!("/sessions/{session_id}/saves")))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(list2.len(), 0);

    // Loading the now-deleted save returns 404.
    let load_again = client
        .get(server.url(&format!("/saves/{save_id}")))
        .send()
        .await
        .unwrap();
    assert_eq!(load_again.status(), 404);

    // Deleting the now-deleted save returns 404.
    let del_again = client
        .delete(server.url(&format!("/saves/{save_id}")))
        .send()
        .await
        .unwrap();
    assert_eq!(del_again.status(), 404);
}

#[tokio::test]
async fn quick_save_creates_auto_save_with_default_metadata() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    db::init_db(&pool).await.unwrap();
    let server = TestServer::start_with(Arc::new(MockProvider::new(vec![])), pool).await;

    let session_id = uuid::Uuid::new_v4().to_string();
    let client = Client::new();

    let resp = client
        .post(server.url(&format!("/sessions/{session_id}/saves/quick")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let listed: Vec<Value> = client
        .get(server.url(&format!("/sessions/{session_id}/saves")))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["kind"], "auto");
    assert_eq!(listed[0]["title"], "Quick save");
    assert_eq!(listed[0]["tag"], "exploration");
}

#[tokio::test]
async fn create_save_rejects_unknown_kind_or_tag() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    db::init_db(&pool).await.unwrap();
    let server = TestServer::start_with(Arc::new(MockProvider::new(vec![])), pool).await;

    let session_id = uuid::Uuid::new_v4().to_string();
    let client = Client::new();

    let bad_kind = client
        .post(server.url(&format!("/sessions/{session_id}/saves")))
        .json(&json!({
            "kind": "ghost",
            "title": "x",
            "summary": "y",
            "tag": "exploration",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(bad_kind.status(), 400);

    let bad_tag = client
        .post(server.url(&format!("/sessions/{session_id}/saves")))
        .json(&json!({
            "kind": "manual",
            "title": "x",
            "summary": "y",
            "tag": "feast",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(bad_tag.status(), 400);
}

#[tokio::test]
async fn list_saves_orders_newest_first() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    db::init_db(&pool).await.unwrap();
    let session_id = uuid::Uuid::new_v4();

    // Insert two saves with explicit ordering via unique created_at by using direct DB calls.
    let envelope = json!({"schema_version": 1, "state": {}});
    let _id1 = db::save_insert(&pool, session_id, "manual", "first", "summary 1", "combat", &envelope)
        .await
        .unwrap();
    // Sleep briefly to guarantee a strictly-larger created_at on save 2.
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    let id2 = db::save_insert(&pool, session_id, "auto", "second", "summary 2", "dialog", &envelope)
        .await
        .unwrap();

    let server = TestServer::start_with(Arc::new(MockProvider::new(vec![])), pool).await;
    let listed: Vec<Value> = Client::new()
        .get(server.url(&format!("/sessions/{session_id}/saves")))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0]["id"], id2.to_string(), "newest save first");
    assert_eq!(listed[0]["title"], "second");
    assert_eq!(listed[1]["title"], "first");
}
