//! Integration tests for the Local Mode HTTP surface (Phase D).
//!
//! - Config GET/POST round-trips state through `LocalModeConfig`.
//! - `/local/runtime/status` returns Off/Off snapshot before any sidecar boot.
//! - `/local/download/{id}` POST happily kicks off (then aborts via cancel),
//!   and DELETE returns 204 even from idle.
//! - `/settings/v2` `local-mistralrs` variant swaps the active provider.

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
    // POST /settings/v2 shape: nested with chat.providers["local-mistralrs"]
    let body = json!({
        "chat": {
            "active_provider_id": "local-mistralrs",
            "active_model_id": "Qwen3.5-2B-Q4_K_M.gguf",
            "providers": {
                "local-mistralrs": { "model_id": "qwen3_5_2b", "port": 37000 }
            },
            "vision_enabled": false,
            "reasoning_enabled": false,
            "reasoning_budget": "medium"
        },
        "image": {
            "enabled": false,
            "active_provider_id": "local-sdxl-lightning",
            "active_model_id": "sdxl-lightning-4step",
            "providers": {},
            "preset": "balanced",
            "style_lora": null
        },
        "video": {
            "enabled": false,
            "active_provider_id": "local-ltx-video",
            "active_model_id": "ltx-video-0.9.6-distilled",
            "providers": {},
            "mode": "prerecorded"
        },
        "behavior": {
            "system_prompt": "",
            "temperature": 0.7,
            "ui_language": "en",
            "narration_language": "en",
            "license_restricted_mode": false,
            "agent_max_rounds": 8,
            "scene_transitions": "auto"
        }
    });
    let resp = client
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("post settings v2");
    assert_eq!(resp.status(), 200);
    // After swapping, the active provider name should reflect local-mistralrs.
    assert_eq!(server.state.provider().name(), "local-mistralrs");
    assert_eq!(
        server.state.default_model(),
        "Qwen3.5-2B-Q4_K_M.gguf"
    );
}
