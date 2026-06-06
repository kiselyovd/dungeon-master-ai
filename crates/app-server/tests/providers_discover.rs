use app_server::test_support::TestServer;
use serde_json::json;

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
