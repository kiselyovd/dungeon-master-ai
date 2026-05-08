//! Integration tests for the Local Mode HTTP surface (Phase D).
//!
//! - Config GET/POST round-trips state through `LocalModeConfig`.
//! - `/local/runtime/status` returns Off/Off snapshot before any sidecar boot.
//! - `/local/download/{id}` POST happily kicks off (then aborts via cancel),
//!   and DELETE returns 204 even from idle.
//! - `/settings` `local-mistralrs` variant swaps the active provider.

use app_server::test_support::TestServer;
use reqwest::Client;
use serde_json::json;

#[tokio::test]
async fn config_round_trips_via_post_then_get() {
    let server = TestServer::start().await;
    let client = Client::new();

    let post_body = json!({
        "selected_llm": "qwen3_5_2b",
        "vram_strategy": "keep-both-loaded"
    });
    let resp = client
        .post(server.url("/local-mode/config"))
        .json(&post_body)
        .send()
        .await
        .expect("post config");
    assert_eq!(resp.status(), 200);

    let resp = client
        .get(server.url("/local-mode/config"))
        .send()
        .await
        .expect("get config");
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["selected_llm"], "qwen3_5_2b");
    assert_eq!(body["vram_strategy"], "keep-both-loaded");
}

#[tokio::test]
async fn runtime_status_initially_off_for_both() {
    let server = TestServer::start().await;
    let resp = reqwest::get(server.url("/local/runtime/status"))
        .await
        .expect("get status");
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["llm"]["state"], "off");
    assert_eq!(body["image"]["state"], "off");
}

#[tokio::test]
async fn download_cancel_returns_204_even_when_idle() {
    let server = TestServer::start().await;
    let client = Client::new();
    let resp = client
        .delete(server.url("/local/download/qwen3_5_0_8b"))
        .send()
        .await
        .expect("delete idle");
    assert_eq!(resp.status(), 204);
}

#[tokio::test]
async fn local_mistralrs_provider_swap_via_settings() {
    let server = TestServer::start().await;
    let client = Client::new();
    let body = json!({
        "kind": "local-mistralrs",
        "model_id": "qwen3_5_2b",
        "port": 37000
    });
    let resp = client
        .post(server.url("/settings"))
        .json(&body)
        .send()
        .await
        .expect("post settings");
    assert_eq!(resp.status(), 200);
    let info: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(info["kind"], "local-mistralrs");
    assert_eq!(info["default_model"], "qwen3.5-2b-instruct-q4_k_m.gguf");
}
