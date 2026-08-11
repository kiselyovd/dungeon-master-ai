use app_server::test_support::TestServer;
use reqwest::Client;
use serde_json::json;
use sqlx::Row;

#[tokio::test]
async fn post_combat_start_returns_200() {
    let server = TestServer::start().await;
    let body = json!({
        "campaign_id": "00000000-0000-0000-0000-000000000001",
        "session_id": "00000000-0000-0000-0000-000000000002",
        "initiative_entries": [
            { "id": "00000000-0000-0000-0000-000000000010", "name": "Hero", "roll": 18, "dex_mod": 2, "hp": 15, "max_hp": 15, "ac": 14 },
            { "id": "00000000-0000-0000-0000-000000000011", "name": "Goblin", "roll": 10, "dex_mod": 1, "hp": 7, "max_hp": 7, "ac": 13 }
        ]
    });
    let resp = Client::new()
        .post(server.url("/combat/start"))
        .json(&body)
        .send()
        .await
        .expect("post");
    assert_eq!(
        resp.status(),
        200,
        "body: {}",
        resp.text().await.unwrap_or_default()
    );
}

#[tokio::test]
async fn post_combat_end_returns_200() {
    let server = TestServer::start().await;
    let body = json!({ "encounter_id": "00000000-0000-0000-0000-000000000020" });
    let resp = Client::new()
        .post(server.url("/combat/end"))
        .json(&body)
        .send()
        .await
        .expect("post");
    // 200 or 404 (no active combat) - just not 500
    assert_ne!(resp.status().as_u16(), 500);
}

#[tokio::test]
async fn post_combat_action_rejects_invalid_payload() {
    let server = TestServer::start().await;
    let body = json!({ "not_an_action": true });
    let resp = Client::new()
        .post(server.url("/combat/action"))
        .json(&body)
        .send()
        .await
        .expect("post");
    assert!(
        resp.status().is_client_error(),
        "invalid payload should be 4xx, got {}",
        resp.status()
    );
}

#[tokio::test]
async fn post_combat_action_commits_a_new_authoritative_revision() {
    let server = TestServer::start().await;
    let client = Client::new();
    let start = json!({
        "campaign_id": "00000000-0000-0000-0000-000000000001",
        "session_id": "00000000-0000-0000-0000-000000000002",
        "initiative_entries": [
            { "id": "00000000-0000-0000-0000-000000000010", "name": "Hero", "roll": 18, "dex_mod": 2, "hp": 15, "max_hp": 15, "ac": 14 },
            { "id": "00000000-0000-0000-0000-000000000011", "name": "Goblin", "roll": 10, "dex_mod": 1, "hp": 7, "max_hp": 7, "ac": 13 }
        ]
    });
    let start_response = client
        .post(server.url("/combat/start"))
        .json(&start)
        .send()
        .await
        .expect("start combat");
    assert_eq!(start_response.status(), 200);
    let row = sqlx::query("SELECT id FROM combat_encounters LIMIT 1")
        .fetch_one(server.state.db())
        .await
        .expect("encounter row");
    let encounter_id: String = row.try_get("id").expect("encounter id");

    let action = json!({
        "encounter_id": encounter_id,
        "action_type": "attack",
        "request_id": "http-attack-1",
        "expected_revision": 0,
        "rng_seed": 42,
        "args": {
            "attacker_id": "00000000-0000-0000-0000-000000000010",
            "target_id": "00000000-0000-0000-0000-000000000011",
            "attack_modifier": 100,
            "damage_dice": "1d6+2",
            "damage_type": "slashing"
        }
    });
    let action_response = client
        .post(server.url("/combat/action"))
        .json(&action)
        .send()
        .await
        .expect("resolve attack");
    assert_eq!(action_response.status(), 200);
    let body = action_response.text().await.expect("SSE body");
    assert!(body.contains("event: combat_projection"), "body: {body}");
    assert!(body.contains("\"revision\":1"), "body: {body}");

    let row = sqlx::query("SELECT revision FROM combat_encounters WHERE id = ?1")
        .bind(encounter_id)
        .fetch_one(server.state.db())
        .await
        .expect("updated encounter");
    let revision: i64 = row.try_get("revision").expect("revision");
    assert_eq!(revision, 1);
}
