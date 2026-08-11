use app_server::test_support::TestServer;

#[tokio::test]
async fn health_returns_ok_with_version() {
    let server = TestServer::start().await;

    let resp = reqwest::get(server.url("/health")).await.expect("request");
    assert_eq!(resp.status(), 200);
    let request_id = resp
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .expect("generated request id");
    assert!(!request_id.is_empty());
    let body: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["service"], "app-server");
    assert!(body["version"].is_string());
}

#[tokio::test]
async fn body_limit_rejects_oversized_agent_payload_before_dispatch() {
    let server = TestServer::start().await;
    let oversized = "x".repeat(33 * 1024 * 1024);
    let response = reqwest::Client::new()
        .post(server.url("/agent/turn"))
        .header("content-type", "application/json")
        .body(oversized)
        .send()
        .await
        .expect("request");
    assert_eq!(response.status(), reqwest::StatusCode::PAYLOAD_TOO_LARGE);
}
