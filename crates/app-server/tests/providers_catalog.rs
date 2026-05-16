use app_server::test_support::TestServer;
use serde_json::Value;

#[tokio::test]
async fn get_providers_catalog_returns_chat_image_video_keys() {
    let server = TestServer::start().await;
    let resp = reqwest::get(server.url("/providers/catalog"))
        .await
        .expect("request");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("json");
    assert!(body["chat"].is_array());
    assert!(body["image"].is_array());
    assert!(body["video"].is_array());

    let chat_ids: Vec<&str> = body["chat"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap())
        .collect();
    assert!(chat_ids.contains(&"anthropic"));
    assert!(chat_ids.contains(&"openai-compat"));
    assert!(chat_ids.contains(&"local-mistralrs"));
}

#[tokio::test]
async fn catalog_anthropic_has_three_curated_models_with_haiku_default() {
    let server = TestServer::start().await;
    let body: Value = reqwest::get(server.url("/providers/catalog"))
        .await
        .expect("request")
        .json()
        .await
        .expect("json");
    let anthropic = body["chat"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == "anthropic")
        .expect("anthropic entry");
    let models = anthropic["curated_models"].as_array().unwrap();
    assert_eq!(models.len(), 3);
    let default_count = models.iter().filter(|m| m["default"] == true).count();
    assert_eq!(default_count, 1);
    let haiku = models
        .iter()
        .find(|m| m["model_id"] == "claude-haiku-4-5-20251001")
        .unwrap();
    assert_eq!(haiku["default"], true);
}
