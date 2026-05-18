//! Smoke coverage for the M9-DM Task 14 endpoint pair:
//! - `GET /local-llm/manifest` returns the curated Qwen3.5 system catalog
//! - `POST /local-llm/active-model` accepts known ids and rejects unknown ids

use app_server::test_support::TestServer;

#[tokio::test]
async fn manifest_returns_four_qwen_entries() {
    let server = TestServer::start().await;
    let resp = reqwest::get(server.url("/local-llm/manifest"))
        .await
        .expect("request");
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.expect("json");

    let system = body["system"].as_array().expect("system is array");
    assert_eq!(system.len(), 4);
    let ids: Vec<&str> = system
        .iter()
        .map(|e| e["id"].as_str().expect("id is string"))
        .collect();
    assert_eq!(
        ids,
        vec!["qwen3.5-0.8b", "qwen3.5-2b", "qwen3.5-4b", "qwen3.5-9b"]
    );
    // User manifest is empty until HF search lands.
    assert_eq!(body["user"].as_array().unwrap().len(), 0);
    // Installed ids and download states default to empty for a fresh server.
    assert_eq!(body["installed_ids"].as_array().unwrap().len(), 0);
    assert!(body["download_states"].is_object());
}

#[tokio::test]
async fn active_model_accepts_known_id() {
    let server = TestServer::start().await;
    let client = reqwest::Client::new();
    let resp = client
        .post(server.url("/local-llm/active-model"))
        .json(&serde_json::json!({ "id": "qwen3.5-2b" }))
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 204);
}

#[tokio::test]
async fn active_model_rejects_unknown_id() {
    let server = TestServer::start().await;
    let client = reqwest::Client::new();
    let resp = client
        .post(server.url("/local-llm/active-model"))
        .json(&serde_json::json!({ "id": "made-up-model" }))
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 400);
}
