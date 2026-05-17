use app_server::test_support::TestServer;
use serde_json::{Value, json};

#[tokio::test]
async fn discover_anthropic_returns_curated_three_models() {
    let server = TestServer::start().await;
    let res = reqwest::Client::new()
        .post(server.url("/providers/discover"))
        .json(&json!({ "provider_id": "anthropic" }))
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.expect("json");
    assert_eq!(body["models"].as_array().unwrap().len(), 3);
    assert_eq!(body["source"], "curated");
}

#[tokio::test]
async fn discover_unsupported_provider_returns_404() {
    let server = TestServer::start().await;
    let res = reqwest::Client::new()
        .post(server.url("/providers/discover"))
        .json(&json!({ "provider_id": "not-supported" }))
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 404);
}

#[tokio::test]
async fn discover_replicate_without_api_key_returns_502() {
    let server = TestServer::start().await;
    let res = reqwest::Client::new()
        .post(server.url("/providers/discover"))
        .json(&json!({ "provider_id": "replicate" }))
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 502);
}
