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
use serde_json::{json, Value};

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
async fn update_save_overwrites_in_place_without_creating_a_duplicate() {
    // M11 F3: the "Overwrite" action PUTs an existing save instead of POSTing a
    // new one (which used to duplicate the row).
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    db::init_db(&pool).await.unwrap();
    let server = TestServer::start_with(Arc::new(MockProvider::new(vec![])), pool).await;

    let session_id = uuid::Uuid::new_v4().to_string();
    let client = Client::new();

    let created: Value = client
        .post(server.url(&format!("/sessions/{session_id}/saves")))
        .json(&json!({
            "kind": "manual",
            "title": "Original",
            "summary": "first summary",
            "tag": "exploration",
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let save_id = created["id"].as_str().unwrap().to_string();

    // Overwrite via PUT - 204 No Content.
    let put = client
        .put(server.url(&format!("/saves/{save_id}")))
        .json(&json!({
            "kind": "manual",
            "title": "Updated",
            "summary": "new summary",
            "tag": "combat",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(put.status(), 204);

    // Still exactly one save (no duplicate row), with the updated metadata.
    let listed: Vec<Value> = client
        .get(server.url(&format!("/sessions/{session_id}/saves")))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(listed.len(), 1, "overwrite must not create a duplicate row");
    assert_eq!(listed[0]["id"], save_id);
    assert_eq!(listed[0]["title"], "Updated");
    assert_eq!(listed[0]["tag"], "combat");

    // Updating a non-existent save returns 404.
    let missing = client
        .put(server.url(&format!("/saves/{}", uuid::Uuid::new_v4())))
        .json(&json!({ "kind": "manual", "title": "x", "summary": "y", "tag": "combat" }))
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), 404);
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
    let _id1 = db::save_insert(
        &pool,
        session_id,
        "manual",
        "first",
        "summary 1",
        "combat",
        &envelope,
    )
    .await
    .unwrap();
    // Sleep briefly to guarantee a strictly-larger created_at on save 2.
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    let id2 = db::save_insert(
        &pool,
        session_id,
        "auto",
        "second",
        "summary 2",
        "dialog",
        &envelope,
    )
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

// ---- W2.3: real game_state capture + restore ----

/// Helper: seed a session row so build_save_game_state can resolve the campaign_id.
async fn seed_session(pool: &sqlx::SqlitePool, session_id: uuid::Uuid, campaign_id: uuid::Uuid) {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO campaigns (id, name, language, created_at, last_played) \
         VALUES (?1, 'Test Campaign', 'en', ?2, ?2)",
    )
    .bind(campaign_id.to_string())
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO sessions (id, campaign_id, number, started_at) VALUES (?1, ?2, 1, ?3)",
    )
    .bind(session_id.to_string())
    .bind(campaign_id.to_string())
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn quick_save_executor_captures_real_combat_and_scene_state() {
    use app_llm::ToolCall;
    use app_server::agent::tool_executor::execute_tool;

    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    db::init_db(&pool).await.unwrap();

    let session_id = uuid::Uuid::new_v4();
    let campaign_id = uuid::Uuid::new_v4();
    seed_session(&pool, session_id, campaign_id).await;

    // Start a combat encounter.
    let enc_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let initiative = serde_json::json!(["hero-1", "goblin-1"]).to_string();
    sqlx::query(
        "INSERT INTO combat_encounters (id, session_id, round, active_turn, started_at, initiative) \
         VALUES (?1, ?2, 3, 'hero-1', ?3, ?4)",
    )
    .bind(&enc_id)
    .bind(session_id.to_string())
    .bind(&now)
    .bind(initiative)
    .execute(&pool)
    .await
    .unwrap();

    // Add a token with resistances.
    sqlx::query(
        "INSERT INTO combat_tokens \
         (id, encounter_id, name, current_hp, max_hp, ac, pos_x, pos_y, conditions, is_dead, resistances, immunities, vulnerabilities) \
         VALUES (?1,?2,'Hero',12,15,14,2,3,'[\"prone\"]',0,'[\"fire\"]','[\"poison\"]','[\"bludgeoning\"]')",
    )
    .bind("hero-1")
    .bind(&enc_id)
    .execute(&pool)
    .await
    .unwrap();

    // Insert a scene.
    db::scene_insert(
        &pool,
        campaign_id,
        "Dragon's Lair",
        Some("Round 3"),
        "combat",
        None,
    )
    .await
    .unwrap();

    // Execute quick_save via the tool executor.
    let tc = ToolCall {
        id: "tc-qs".into(),
        name: "quick_save".into(),
        args: serde_json::json!({ "label": "boss fight" }),
    };
    let (val, is_err) =
        execute_tool(&tc, &pool, None, None, None, "", campaign_id, session_id).await;
    assert!(!is_err, "executor failed: {val}");

    let save_id = val["save_id"].as_str().expect("save_id");

    // Load the save and inspect game_state.
    use sqlx::Row;
    let row = sqlx::query("SELECT game_state FROM snapshots WHERE id = ?1")
        .bind(save_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let gs_str: String = row.get("game_state");
    let gs: serde_json::Value = serde_json::from_str(&gs_str).unwrap();

    assert_eq!(gs["schema_version"], 2, "must be schema_version 2");
    assert_eq!(gs["label"], "boss fight");

    let combat = &gs["combat"];
    assert!(combat.is_object(), "combat must be present");
    assert_eq!(combat["active"], true);
    assert_eq!(combat["round"], 3);
    assert_eq!(combat["current_turn_id"], "hero-1");
    let tokens = combat["tokens"].as_array().unwrap();
    assert_eq!(tokens.len(), 1);
    let tok = &tokens[0];
    assert_eq!(tok["id"], "hero-1");
    assert_eq!(tok["hp"], 12);
    assert_eq!(tok["max_hp"], 15);
    assert_eq!(tok["ac"], 14);
    assert!(tok["conditions"]
        .as_array()
        .unwrap()
        .contains(&json!("prone")));
    assert!(tok["resistances"]
        .as_array()
        .unwrap()
        .contains(&json!("fire")));
    assert!(tok["immunities"]
        .as_array()
        .unwrap()
        .contains(&json!("poison")));
    assert!(tok["vulnerabilities"]
        .as_array()
        .unwrap()
        .contains(&json!("bludgeoning")));

    let scene = &gs["scene"];
    assert!(scene.is_object(), "scene must be present");
    assert_eq!(scene["title"], "Dragon's Lair");
    assert_eq!(scene["subtitle"], "Round 3");
    assert_eq!(scene["mode"], "combat");
}

#[tokio::test]
async fn restore_snapshot_rehydrates_combat_encounter_and_tokens() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    db::init_db(&pool).await.unwrap();

    let session_id = uuid::Uuid::new_v4();
    let campaign_id = uuid::Uuid::new_v4();
    seed_session(&pool, session_id, campaign_id).await;

    // Build a game_state v2 directly and insert a save row.
    let enc_id = "enc-restore-test".to_string();
    let gs = serde_json::json!({
        "schema_version": 2,
        "label": "checkpoint",
        "combat": {
            "active": true,
            "encounter_id": enc_id,
            "round": 2,
            "current_turn_id": "tok-a",
            "initiative": ["tok-a", "tok-b"],
            "tokens": [
                {
                    "id": "tok-a", "name": "Paladin", "hp": 20, "max_hp": 30,
                    "ac": 18, "x": 1, "y": 2, "conditions": [],
                    "resistances": ["radiant"], "immunities": [], "vulnerabilities": []
                },
                {
                    "id": "tok-b", "name": "Orc", "hp": 8, "max_hp": 15,
                    "ac": 12, "x": 4, "y": 5, "conditions": ["frightened"],
                    "resistances": [], "immunities": [], "vulnerabilities": ["radiant"]
                }
            ]
        },
        "scene": {
            "title": "Hall of Echoes",
            "subtitle": null,
            "mode": "combat"
        }
    });

    let save_id = db::save_insert(
        &pool,
        session_id,
        "auto",
        "checkpoint",
        "combat save",
        "combat",
        &gs,
    )
    .await
    .unwrap();

    // Put a different open encounter in the DB so we can verify it gets ended.
    let now = chrono::Utc::now().to_rfc3339();
    let old_enc_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO combat_encounters (id, session_id, round, started_at, initiative) \
         VALUES (?1, ?2, 1, ?3, '[]')",
    )
    .bind(&old_enc_id)
    .bind(session_id.to_string())
    .bind(&now)
    .execute(&pool)
    .await
    .unwrap();

    // Restore the snapshot.
    let result = db::restore_snapshot(&pool, session_id, save_id)
        .await
        .unwrap();
    assert!(result.is_some(), "restore must return Some for v2 save");

    // Verify the old encounter is now ended.
    use sqlx::Row;
    let old_enc: Option<String> =
        sqlx::query("SELECT ended_at FROM combat_encounters WHERE id = ?1")
            .bind(&old_enc_id)
            .fetch_one(&pool)
            .await
            .unwrap()
            .try_get("ended_at")
            .unwrap();
    assert!(old_enc.is_some(), "old encounter must have been ended");

    // Verify the restored encounter exists and is open.
    let row = sqlx::query(
        "SELECT round, active_turn FROM combat_encounters WHERE id = ?1 AND ended_at IS NULL",
    )
    .bind(&enc_id)
    .fetch_one(&pool)
    .await
    .expect("restored encounter must exist with ended_at IS NULL");
    let round: i64 = row.get("round");
    assert_eq!(round, 2);
    let active_turn: String = row.get("active_turn");
    assert_eq!(active_turn, "tok-a");

    // Verify tokens were restored.
    let tokens = sqlx::query(
        "SELECT id, name, current_hp, ac FROM combat_tokens WHERE encounter_id = ?1 ORDER BY id",
    )
    .bind(&enc_id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(tokens.len(), 2);
    let tok_a = tokens
        .iter()
        .find(|r| r.get::<String, _>("id") == "tok-a")
        .unwrap();
    assert_eq!(tok_a.get::<i32, _>("current_hp"), 20);
    assert_eq!(tok_a.get::<i32, _>("ac"), 18);

    let tok_b = tokens
        .iter()
        .find(|r| r.get::<String, _>("id") == "tok-b")
        .unwrap();
    assert_eq!(tok_b.get::<i32, _>("current_hp"), 8);
}

#[tokio::test]
async fn restore_save_route_returns_game_state() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    db::init_db(&pool).await.unwrap();

    let session_id = uuid::Uuid::new_v4();
    let campaign_id = uuid::Uuid::new_v4();
    let pool2 = pool.clone();
    seed_session(&pool, session_id, campaign_id).await;

    // Insert a v2 save.
    let gs = serde_json::json!({
        "schema_version": 2,
        "label": "route test",
        "combat": null,
        "scene": { "title": "Forest", "subtitle": null, "mode": "exploration" }
    });
    let save_id = db::save_insert(
        &pool,
        session_id,
        "auto",
        "route test",
        "",
        "exploration",
        &gs,
    )
    .await
    .unwrap();

    let server = TestServer::start_with(Arc::new(MockProvider::new(vec![])), pool2).await;
    let client = Client::new();

    let resp = client
        .post(server.url(&format!("/saves/{save_id}/restore?session_id={session_id}")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "restore route must return 200");

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body.get("game_state").is_some(), "must have game_state");
    assert_eq!(body["game_state"]["schema_version"], 2);
    assert_eq!(body["game_state"]["scene"]["title"], "Forest");
}
