use std::sync::Arc;

use app_llm::MockProvider;
use app_server::db;
use app_server::test_support::TestServer;
use reqwest::Client;
use serde_json::{json, Value};
use uuid::Uuid;

async fn seed_session(pool: &sqlx::SqlitePool) -> (Uuid, Uuid) {
    let campaign_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let now = "1700000000000";
    sqlx::query(
        "INSERT INTO campaigns \
         (id, schema_version, ruleset_id, title, premise, created_at, updated_at, name, language, last_played) \
         VALUES (?1, 1, 'srd-5.2.1', 'Test', '', ?2, ?2, 'Test', 'ru', ?2)",
    )
    .bind(campaign_id.to_string())
    .bind(now)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO sessions \
         (id, campaign_id, revision, created_at, updated_at, number, started_at) \
         VALUES (?1, ?2, 4, ?3, ?3, 1, ?3)",
    )
    .bind(session_id.to_string())
    .bind(campaign_id.to_string())
    .bind(now)
    .execute(pool)
    .await
    .unwrap();
    (campaign_id, session_id)
}

fn save_body(campaign_id: Uuid, causation_id: &str, kind: &str) -> Value {
    json!({
        "campaign_id": campaign_id,
        "expected_revision": 4,
        "causation_id": causation_id,
        "character": { "name": "Mira", "hp": 12 },
        "kind": kind,
        "title": "Before the gate",
        "summary": "The road is quiet",
        "tag": "exploration"
    })
}

#[tokio::test]
async fn save_round_trip_returns_and_restores_one_complete_revision_projection() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    db::init_db(&pool).await.unwrap();
    let (campaign_id, session_id) = seed_session(&pool).await;
    let server = TestServer::start_with(Arc::new(MockProvider::new(vec![])), pool).await;
    let client = Client::new();

    let create = client
        .post(server.url(&format!("/sessions/{session_id}/saves")))
        .json(&save_body(campaign_id, "http-save-1", "manual"))
        .send()
        .await
        .unwrap();
    assert_eq!(create.status(), 201);
    let save_id = create.json::<Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let listed = client
        .get(server.url(&format!("/sessions/{session_id}/saves")))
        .send()
        .await
        .unwrap()
        .json::<Vec<Value>>()
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["campaign_id"], campaign_id.to_string());
    assert_eq!(listed[0]["revision"], 4);

    let loaded = client
        .get(server.url(&format!("/saves/{save_id}")))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();
    assert_eq!(loaded["snapshot"]["ruleset_id"], "srd-5.2.1");
    assert_eq!(
        loaded["snapshot"]["projection"]["character"]["name"],
        "Mira"
    );

    let restored = client
        .post(server.url(&format!("/saves/{save_id}/restore")))
        .json(&json!({
            "campaign_id": campaign_id,
            "session_id": session_id,
            "expected_revision": 4,
            "causation_id": "http-restore-1"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(restored.status(), 200);
    let restored = restored.json::<Value>().await.unwrap();
    assert_eq!(restored["schema_version"], 1);
    assert_eq!(restored["ruleset_id"], "srd-5.2.1");
    for required in [
        "messages",
        "character",
        "combat",
        "scene",
        "journal",
        "npcs",
    ] {
        assert!(
            restored["projection"].get(required).is_some(),
            "missing {required}"
        );
    }

    let deleted = client
        .delete(server.url(&format!("/saves/{save_id}")))
        .send()
        .await
        .unwrap();
    assert_eq!(deleted.status(), 204);
}

#[tokio::test]
async fn duplicate_save_causation_is_idempotent_and_stale_revision_conflicts() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    db::init_db(&pool).await.unwrap();
    let (campaign_id, session_id) = seed_session(&pool).await;
    let server = TestServer::start_with(Arc::new(MockProvider::new(vec![])), pool).await;
    let client = Client::new();
    let url = server.url(&format!("/sessions/{session_id}/saves"));

    for _ in 0..2 {
        assert_eq!(
            client
                .post(&url)
                .json(&save_body(campaign_id, "same-causation", "manual"))
                .send()
                .await
                .unwrap()
                .status(),
            201
        );
    }
    let listed = client
        .get(&url)
        .send()
        .await
        .unwrap()
        .json::<Vec<Value>>()
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);

    let mut stale = save_body(campaign_id, "stale-causation", "manual");
    stale["expected_revision"] = json!(3);
    let response = client.post(&url).json(&stale).send().await.unwrap();
    assert_eq!(response.status(), 409);
}

#[tokio::test]
async fn quick_save_accepts_the_same_complete_context_and_forces_auto_kind() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    db::init_db(&pool).await.unwrap();
    let (campaign_id, session_id) = seed_session(&pool).await;
    let server = TestServer::start_with(Arc::new(MockProvider::new(vec![])), pool).await;
    let client = Client::new();
    let response = client
        .post(server.url(&format!("/sessions/{session_id}/saves/quick")))
        .json(&save_body(campaign_id, "quick-causation", "manual"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 201);
    let listed = client
        .get(server.url(&format!("/sessions/{session_id}/saves")))
        .send()
        .await
        .unwrap()
        .json::<Vec<Value>>()
        .await
        .unwrap();
    assert_eq!(listed[0]["kind"], "auto");
}
