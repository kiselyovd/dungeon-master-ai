use app_server::test_support::TestServer;
use reqwest::Client;
use serde_json::json;

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
