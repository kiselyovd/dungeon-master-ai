//! Integration tests for the multi-provider settings HTTP surface:
//! GET /providers + POST /settings/v2 (provider hot-swap).

use app_server::test_support::TestServer;
use reqwest::Client;
use serde_json::json;

/// Returns a baseline v2 body with Anthropic as the active provider.
fn v2_anthropic(api_key: &str, model: &str) -> serde_json::Value {
    json!({
        "chat": {
            "active_provider_id": "anthropic",
            "active_model_id": model,
            "providers": { "anthropic": { "api_key": api_key } },
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
    })
}

/// Returns a baseline v2 body with openai-compat as the active provider.
fn v2_openai_compat(base_url: &str, api_key: &str, model: &str) -> serde_json::Value {
    json!({
        "chat": {
            "active_provider_id": "openai-compat",
            "active_model_id": model,
            "providers": {
                "openai-compat": { "base_url": base_url, "api_key": api_key }
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
    })
}

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

    let resp = Client::new()
        .post(server.url("/settings/v2"))
        .json(&v2_openai_compat("http://localhost:1234", "sk-test", "qwen3-1.7b"))
        .send()
        .await
        .expect("post /settings/v2");
    assert_eq!(resp.status(), 200);

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

    let resp = Client::new()
        .post(server.url("/settings/v2"))
        .json(&v2_anthropic("sk-ant-test", "claude-haiku-4-5-20251001"))
        .send()
        .await
        .expect("post /settings/v2");
    assert_eq!(resp.status(), 200);

    // Re-query /providers - the active provider should now reflect the swap.
    let providers: serde_json::Value = reqwest::get(server.url("/providers"))
        .await
        .expect("get")
        .json()
        .await
        .expect("json");
    assert_eq!(providers["active"]["kind"], "anthropic");
    assert!(
        providers["active"]["default_model"]
            .as_str()
            .unwrap_or("")
            .starts_with("claude-")
    );
}

#[tokio::test]
async fn post_settings_v2_rejects_empty_api_key() {
    let server = TestServer::start().await;

    // An empty api_key on an anthropic slice should be rejected.
    let body = json!({
        "chat": {
            "active_provider_id": "anthropic",
            "active_model_id": "claude-haiku-4-5-20251001",
            "providers": { "anthropic": { "api_key": "   " } },
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

    let resp = Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("post /settings/v2");
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn post_settings_v2_rejects_unknown_chat_provider() {
    let server = TestServer::start().await;

    let body = json!({
        "chat": {
            "active_provider_id": "telepathic-vibes",
            "active_model_id": "whatever",
            "providers": {},
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

    let resp = Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("post /settings/v2");
    // validate_settings_v2 rejects unknown provider with 400.
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn post_settings_v2_rejects_openai_compat_without_base_url() {
    let server = TestServer::start().await;

    // base_url empty
    let body = json!({
        "chat": {
            "active_provider_id": "openai-compat",
            "active_model_id": "qwen3",
            "providers": {
                "openai-compat": { "base_url": "", "api_key": "sk-x" }
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
    let resp = Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("post /settings/v2");
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn post_settings_v2_updates_system_prompt_and_temperature() {
    let server = TestServer::start().await;

    let body = json!({
        "chat": {
            "active_provider_id": "anthropic",
            "active_model_id": "claude-haiku-4-5-20251001",
            "providers": { "anthropic": { "api_key": "sk-ant-test" } },
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
            "system_prompt": "Be concise.",
            "temperature": 0.5,
            "ui_language": "en",
            "narration_language": "en",
            "license_restricted_mode": false,
            "agent_max_rounds": 8,
            "scene_transitions": "auto"
        }
    });

    let resp = Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("post /settings/v2");
    assert_eq!(resp.status(), 200);

    let updated: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(updated, json!({ "status": "ok" }));

    // Verify the agent config was updated.
    let agent_cfg = server.state.agent_config();
    assert_eq!(agent_cfg.system_prompt, "Be concise.");
    assert!((agent_cfg.temperature - 0.5_f32).abs() < 0.001);
}

#[tokio::test]
async fn post_settings_v2_rejects_temperature_out_of_range() {
    let server = TestServer::start().await;

    let body = json!({
        "chat": {
            "active_provider_id": "anthropic",
            "active_model_id": "claude-haiku-4-5-20251001",
            "providers": { "anthropic": { "api_key": "sk-ant-test" } },
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
            "temperature": 5.0,
            "ui_language": "en",
            "narration_language": "en",
            "license_restricted_mode": false,
            "agent_max_rounds": 8,
            "scene_transitions": "auto"
        }
    });

    let resp = Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("post /settings/v2");
    assert_eq!(resp.status(), 400);

    let text = resp.text().await.expect("text");
    assert!(
        text.contains("temperature must be between"),
        "expected error message about temperature range, got: {text}"
    );
}
