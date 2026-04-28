use app_server::test_support::TestServer;

#[tokio::test]
async fn health_returns_ok_with_version() {
    let server = TestServer::start().await;

    let resp = reqwest::get(server.url("/health")).await.expect("request");
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["service"], "app-server");
    assert!(body["version"].is_string());
}
