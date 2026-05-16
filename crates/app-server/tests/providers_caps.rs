use app_server::test_support::TestServer;
use serde_json::Value;

#[tokio::test]
async fn caps_for_anthropic_opus_returns_all_true() {
    let server = TestServer::start().await;
    let resp = reqwest::get(server.url("/providers/anthropic/caps?model=claude-opus-4-7"))
        .await
        .expect("request");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["vision_input"], true);
    assert_eq!(body["reasoning"], true);
    assert_eq!(body["tool_calls"], true);
    assert_eq!(body["streaming"], true);
}

#[tokio::test]
async fn caps_for_unknown_provider_returns_404() {
    let server = TestServer::start().await;
    let resp = reqwest::get(server.url("/providers/unknown-provider/caps?model=x"))
        .await
        .expect("request");
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn caps_for_openai_compat_unknown_model_falls_back_to_inference() {
    let server = TestServer::start().await;
    let resp = reqwest::get(server.url("/providers/openai-compat/caps?model=custom-llm-001"))
        .await
        .expect("request");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["tool_calls"], true);
    assert_eq!(body["streaming"], true);
    assert_eq!(body["vision_input"], false);
    assert_eq!(body["reasoning"], false);
}

#[tokio::test]
async fn caps_for_openai_compat_o3_mini_infers_reasoning() {
    let server = TestServer::start().await;
    let resp = reqwest::get(server.url("/providers/openai-compat/caps?model=o3-mini"))
        .await
        .expect("request");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["reasoning"], true);
}
