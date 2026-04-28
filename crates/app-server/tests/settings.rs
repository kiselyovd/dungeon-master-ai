//! Integration tests for the multi-provider settings HTTP surface:
//! GET /providers + POST /settings (provider hot-swap).

use app_server::test_support::TestServer;
use reqwest::Client;
use serde_json::json;

#[tokio::test]
async fn get_providers_lists_kinds_and_active_mock() {
    let server = TestServer::start().await;

    let resp = reqwest::get(server.url("/providers"))
        .await
        .expect("get /providers");
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.expect("json");
    let available = body["available"]
        .as_array()
        .expect("available array")
        .iter()
        .map(|v| v.as_str().expect("string").to_string())
        .collect::<Vec<_>>();
    assert!(available.contains(&"anthropic".to_string()));
    assert!(available.contains(&"openai-compat".to_string()));

    // The default test server is wired with MockProvider whose name is "mock".
    assert_eq!(body["active"]["kind"], "mock");
    assert_eq!(body["active"]["default_model"], "mock");
}

#[tokio::test]
async fn post_settings_swaps_to_openai_compat() {
    let server = TestServer::start().await;

    let body = json!({
        "kind": "openai-compat",
        "base_url": "http://localhost:1234",
        "api_key": "sk-test",
        "model": "qwen3-1.7b",
    });

    let resp = Client::new()
        .post(server.url("/settings"))
        .json(&body)
        .send()
        .await
        .expect("post /settings");
    assert_eq!(resp.status(), 200);

    let updated: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(updated["kind"], "openai-compat");
    assert_eq!(updated["default_model"], "qwen3-1.7b");

    // Re-query /providers - the active provider should now reflect the swap.
    let providers: serde_json::Value = reqwest::get(server.url("/providers"))
        .await
        .expect("get")
        .json()
        .await
        .expect("json");
    assert_eq!(providers["active"]["kind"], "openai-compat");
    assert_eq!(providers["active"]["default_model"], "qwen3-1.7b");
}

#[tokio::test]
async fn post_settings_swaps_to_anthropic_with_default_model() {
    let server = TestServer::start().await;

    let body = json!({
        "kind": "anthropic",
        "api_key": "sk-ant-test",
    });

    let resp = Client::new()
        .post(server.url("/settings"))
        .json(&body)
        .send()
        .await
        .expect("post /settings");
    assert_eq!(resp.status(), 200);

    let updated: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(updated["kind"], "anthropic");
    // The default Claude model should have been substituted.
    assert!(
        updated["default_model"]
            .as_str()
            .unwrap_or("")
            .starts_with("claude-")
    );
}

#[tokio::test]
async fn post_settings_rejects_empty_api_key() {
    let server = TestServer::start().await;

    let body = json!({
        "kind": "anthropic",
        "api_key": "   ",
    });

    let resp = Client::new()
        .post(server.url("/settings"))
        .json(&body)
        .send()
        .await
        .expect("post /settings");
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn post_settings_rejects_unknown_kind() {
    let server = TestServer::start().await;

    let body = json!({
        "kind": "telepathic-vibes",
        "api_key": "sk-x",
    });

    let resp = Client::new()
        .post(server.url("/settings"))
        .json(&body)
        .send()
        .await
        .expect("post /settings");
    // axum returns 422 for failed JSON body deserialization (invalid enum tag).
    assert_eq!(resp.status(), 422);
}

#[tokio::test]
async fn post_settings_rejects_openai_compat_without_required_fields() {
    let server = TestServer::start().await;

    // base_url empty
    let body = json!({
        "kind": "openai-compat",
        "base_url": "",
        "api_key": "sk-x",
        "model": "qwen3",
    });
    let resp = Client::new()
        .post(server.url("/settings"))
        .json(&body)
        .send()
        .await
        .expect("post /settings");
    assert_eq!(resp.status(), 400);
}
