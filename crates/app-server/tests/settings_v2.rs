use app_server::test_support::TestServer;
use serde_json::{Value, json};

fn baseline() -> Value {
    json!({
        "chat": {
            "active_provider_id": "anthropic",
            "active_model_id": "claude-haiku-4-5-20251001",
            "providers": { "anthropic": { "api_key": "sk-test" } },
            "vision_enabled": false,
            "reasoning_enabled": false,
            "reasoning_budget": "medium",
        },
        "image": {
            "enabled": true,
            "active_provider_id": "replicate",
            "active_model_id": "stability-ai/sdxl",
            "providers": { "replicate": { "api_key": "sk-rep" } },
            "preset": "balanced",
            "style_lora": null,
        },
        "video": {
            "enabled": false,
            "active_provider_id": "local-ltx-video",
            "active_model_id": "ltx-video-0.9.6-distilled",
            "providers": {},
            "mode": "prerecorded",
        },
        "behavior": {
            "system_prompt": "DM",
            "temperature": 0.7,
            "ui_language": "en",
            "narration_language": "en",
            "license_restricted_mode": false,
            "agent_max_rounds": 8,
            "scene_transitions": "auto",
        },
    })
}

#[tokio::test]
async fn post_settings_v2_accepts_baseline() {
    let server = TestServer::start().await;
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&baseline())
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn post_settings_v2_rejects_quality_preset_when_license_restricted() {
    let server = TestServer::start().await;
    let mut body = baseline();
    body["behavior"]["license_restricted_mode"] = Value::Bool(true);
    body["image"]["preset"] = Value::String("quality".into());
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 400);
}

#[tokio::test]
async fn post_settings_v2_rejects_unknown_provider() {
    let server = TestServer::start().await;
    let mut body = baseline();
    body["chat"]["active_provider_id"] = Value::String("nope".into());
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 400);
}

#[tokio::test]
async fn post_settings_v2_disables_image_tool_when_image_disabled() {
    let server = TestServer::start().await;
    let mut body = baseline();
    body["image"]["enabled"] = Value::Bool(false);
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 200);
    assert!(
        !server.state.agent_config().tool_availability.image,
        "expected tool_availability.image=false after image.enabled=false",
    );
}

#[tokio::test]
async fn post_settings_v2_enables_image_tool_when_image_enabled() {
    use app_server::agent::orchestrator::AgentConfig;
    use app_server::agent::tools::ToolAvailability;
    let server = TestServer::start().await;
    server.state.set_agent_config(AgentConfig {
        tool_availability: ToolAvailability {
            image: false,
            video: false,
        },
        ..AgentConfig::default()
    });
    let body = baseline();
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 200);
    assert!(
        server.state.agent_config().tool_availability.image,
        "expected tool_availability.image=true after image.enabled=true",
    );
}

#[tokio::test]
async fn post_settings_v2_video_flag_mirrors_video_enabled() {
    use app_server::agent::orchestrator::AgentConfig;
    use app_server::agent::tools::ToolAvailability;
    let server = TestServer::start().await;
    server.state.set_agent_config(AgentConfig {
        tool_availability: ToolAvailability {
            image: false,
            video: false,
        },
        ..AgentConfig::default()
    });
    // Local video provider needs a sidecar URL to construct, else the rebuild
    // step returns 400 and the agent_config update is rolled back implicitly.
    server
        .state
        .set_media_sidecar_url(Some("http://127.0.0.1:8765".into()));
    let mut body = baseline();
    body["video"]["enabled"] = Value::Bool(true);
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 200);
    assert!(
        server.state.agent_config().tool_availability.video,
        "expected tool_availability.video=true after video.enabled=true",
    );
}

#[tokio::test]
async fn post_settings_v2_propagates_behavior_fields_to_agent_config() {
    let server = TestServer::start().await;
    let mut body = baseline();
    body["behavior"]["system_prompt"] = Value::String("You are a strict DM.".into());
    body["behavior"]["temperature"] = json!(0.42);
    body["behavior"]["agent_max_rounds"] = json!(12);
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 200);
    let cfg = server.state.agent_config();
    assert_eq!(cfg.system_prompt, "You are a strict DM.");
    assert!((cfg.temperature - 0.42).abs() < 1e-6);
    assert_eq!(cfg.max_rounds, 12);
}

#[tokio::test]
async fn post_settings_v2_rejects_out_of_range_temperature() {
    let server = TestServer::start().await;
    let mut body = baseline();
    body["behavior"]["temperature"] = json!(2.5);
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 400);
}

#[tokio::test]
async fn post_settings_v2_rebuilds_anthropic_provider() {
    let server = TestServer::start().await;
    assert_eq!(server.state.provider().name(), "mock");
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&baseline())
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 200);
    assert_eq!(server.state.provider().name(), "anthropic");
    assert_eq!(server.state.default_model(), "claude-haiku-4-5-20251001");
}

#[tokio::test]
async fn post_settings_v2_persists_anthropic_api_key_to_secrets_repo() {
    let server = TestServer::start().await;
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&baseline())
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 200);
    let stored = server
        .state
        .secrets_repo()
        .get("anthropic_api_key")
        .await
        .expect("repo get");
    assert_eq!(stored, Some("sk-test".to_string()));
}

#[tokio::test]
async fn post_settings_v2_rejects_anthropic_without_api_key() {
    let server = TestServer::start().await;
    let mut body = baseline();
    body["chat"]["providers"]["anthropic"]["api_key"] = Value::String(String::new());
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 400);
}

#[tokio::test]
async fn post_settings_v2_rejects_anthropic_without_provider_config_slice() {
    let server = TestServer::start().await;
    let mut body = baseline();
    body["chat"]["providers"] = json!({});
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 400);
}

#[tokio::test]
async fn post_settings_v2_rebuilds_openai_compat_provider() {
    let server = TestServer::start().await;
    let mut body = baseline();
    body["chat"]["active_provider_id"] = Value::String("openai-compat".into());
    body["chat"]["active_model_id"] = Value::String("gpt-4o".into());
    body["chat"]["providers"] = json!({
        "openai-compat": { "base_url": "https://api.openai.com", "api_key": "sk-oc" }
    });
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 200);
    assert_eq!(server.state.provider().name(), "openai-compat");
    assert_eq!(server.state.default_model(), "gpt-4o");
    let stored = server
        .state
        .secrets_repo()
        .get("openai_compat_api_key")
        .await
        .expect("repo get");
    assert_eq!(stored, Some("sk-oc".to_string()));
}

#[tokio::test]
async fn post_settings_v2_rejects_openai_compat_without_base_url() {
    let server = TestServer::start().await;
    let mut body = baseline();
    body["chat"]["active_provider_id"] = Value::String("openai-compat".into());
    body["chat"]["active_model_id"] = Value::String("gpt-4o".into());
    body["chat"]["providers"] = json!({
        "openai-compat": { "base_url": "", "api_key": "sk-oc" }
    });
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 400);
}

#[tokio::test]
async fn post_settings_v2_rebuilds_local_mistralrs_provider() {
    let server = TestServer::start().await;
    let mut body = baseline();
    body["chat"]["active_provider_id"] = Value::String("local-mistralrs".into());
    body["chat"]["active_model_id"] = Value::String("qwen3.5-4b".into());
    body["chat"]["providers"] = json!({
        "local-mistralrs": { "model_id": "qwen3_5_4b", "port": 9876 }
    });
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 200);
    assert_eq!(server.state.provider().name(), "local-mistralrs");
    assert!(
        server.state.default_model().contains("qwen3.5-4b"),
        "expected default_model to derive from manifest filename, got {}",
        server.state.default_model(),
    );
}

#[tokio::test]
async fn post_settings_v2_rebuilds_local_mistralrs_with_custom_mmproj_model() {
    let server = TestServer::start().await;
    let mut body = baseline();
    body["chat"]["active_provider_id"] = Value::String("local-mistralrs".into());
    body["chat"]["active_model_id"] =
        Value::String("custom:Qwen/Qwen2.5-VL-7B-Instruct-GGUF".into());
    body["chat"]["providers"] = json!({
        "local-mistralrs": {
            "model_id": {
                "custom": {
                    "hf_repo": "Qwen/Qwen2.5-VL-7B-Instruct-GGUF",
                    "gguf_filename": "qwen2.5-vl-7b-instruct-q4_k_m.gguf",
                    "mmproj_filename": "mmproj-qwen2.5-vl-7b-f16.gguf"
                }
            },
            "port": 9876
        }
    });
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 200, "body: {:?}", res.text().await);
    assert_eq!(server.state.provider().name(), "local-mistralrs");
    assert_eq!(
        server.state.default_model(),
        "qwen2.5-vl-7b-instruct-q4_k_m.gguf",
        "default_model must come from manifest_for(Custom).hf_filename",
    );
}

#[tokio::test]
async fn post_settings_v2_rebuilds_local_mistralrs_with_custom_no_mmproj() {
    let server = TestServer::start().await;
    let mut body = baseline();
    body["chat"]["active_provider_id"] = Value::String("local-mistralrs".into());
    body["chat"]["active_model_id"] = Value::String("custom:llama".into());
    body["chat"]["providers"] = json!({
        "local-mistralrs": {
            "model_id": {
                "custom": {
                    "hf_repo": "TheBloke/Llama-2-7B-Chat-GGUF",
                    "gguf_filename": "llama-2-7b-chat.Q4_K_M.gguf"
                }
            },
            "port": 9876
        }
    });
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 200, "body: {:?}", res.text().await);
    assert_eq!(server.state.provider().name(), "local-mistralrs");
    assert_eq!(
        server.state.default_model(),
        "llama-2-7b-chat.Q4_K_M.gguf",
    );
}

#[tokio::test]
async fn post_settings_v2_rejects_local_mistralrs_custom_missing_gguf_filename() {
    let server = TestServer::start().await;
    let mut body = baseline();
    body["chat"]["active_provider_id"] = Value::String("local-mistralrs".into());
    body["chat"]["active_model_id"] = Value::String("custom:broken".into());
    body["chat"]["providers"] = json!({
        "local-mistralrs": {
            "model_id": {
                "custom": {
                    "hf_repo": "x/y"
                }
            },
            "port": 9876
        }
    });
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 400);
}

// ---- image provider rebuild ----

#[tokio::test]
async fn post_settings_v2_clears_image_provider_when_image_disabled() {
    let server = TestServer::start().await;
    let mut body = baseline();
    body["image"]["enabled"] = Value::Bool(false);
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 200);
    assert!(
        server.state.image_provider().is_none(),
        "expected image_provider to be None after image.enabled=false",
    );
}

#[tokio::test]
async fn post_settings_v2_rebuilds_replicate_image_provider() {
    let server = TestServer::start().await;
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&baseline())
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 200);
    let p = server
        .state
        .image_provider()
        .expect("image provider should be Some for replicate");
    assert!(
        p.cost_per_image() > 0.0,
        "expected cloud provider (cost>0), got {}",
        p.cost_per_image()
    );
    let stored = server
        .state
        .secrets_repo()
        .get("replicate_api_key")
        .await
        .expect("repo get");
    assert_eq!(stored, Some("sk-rep".to_string()));
}

#[tokio::test]
async fn post_settings_v2_rejects_replicate_without_api_key() {
    let server = TestServer::start().await;
    let mut body = baseline();
    body["image"]["providers"]["replicate"]["api_key"] = Value::String(String::new());
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 400);
}

#[tokio::test]
async fn post_settings_v2_rebuilds_local_image_provider_when_sidecar_running() {
    let server = TestServer::start().await;
    server
        .state
        .set_media_sidecar_url(Some("http://127.0.0.1:8765".into()));
    let mut body = baseline();
    body["image"]["active_provider_id"] = Value::String("local-sdxl-lightning".into());
    body["image"]["active_model_id"] = Value::String("sdxl-lightning-4step".into());
    body["image"]["providers"] = json!({});
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 200);
    let p = server
        .state
        .image_provider()
        .expect("image provider should be Some for local");
    assert_eq!(
        p.cost_per_image(),
        0.0,
        "expected local provider (cost==0), got {}",
        p.cost_per_image()
    );
}

#[tokio::test]
async fn post_settings_v2_rejects_local_image_without_sidecar_url() {
    let server = TestServer::start().await;
    let mut body = baseline();
    body["image"]["active_provider_id"] = Value::String("local-sdxl-lightning".into());
    body["image"]["active_model_id"] = Value::String("sdxl-lightning-4step".into());
    body["image"]["providers"] = json!({});
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 400);
}

#[tokio::test]
async fn post_settings_v2_rejects_replicate_without_provider_config_slice() {
    let server = TestServer::start().await;
    let mut body = baseline();
    body["image"]["providers"] = json!({});
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 400);
}

#[tokio::test]
async fn post_settings_v2_rejects_unknown_image_provider() {
    let server = TestServer::start().await;
    let mut body = baseline();
    body["image"]["active_provider_id"] = Value::String("not-a-real-image-provider".into());
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 400);
}

// ---- video provider rebuild ----

#[tokio::test]
async fn post_settings_v2_clears_video_provider_when_video_disabled() {
    let server = TestServer::start().await;
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&baseline())
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 200);
    assert!(
        server.state.video_provider().is_none(),
        "expected video_provider to be None when video.enabled=false in baseline",
    );
}

#[tokio::test]
async fn post_settings_v2_rebuilds_local_video_provider_when_sidecar_running() {
    let server = TestServer::start().await;
    server
        .state
        .set_media_sidecar_url(Some("http://127.0.0.1:8765".into()));
    let mut body = baseline();
    body["video"]["enabled"] = Value::Bool(true);
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 200);
    assert!(
        server.state.video_provider().is_some(),
        "expected video_provider to be Some after enabled=true with sidecar URL",
    );
}

#[tokio::test]
async fn post_settings_v2_rejects_local_video_without_sidecar_url() {
    let server = TestServer::start().await;
    let mut body = baseline();
    body["video"]["enabled"] = Value::Bool(true);
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 400);
}

#[tokio::test]
async fn post_settings_v2_rejects_unknown_video_provider() {
    let server = TestServer::start().await;
    let mut body = baseline();
    body["video"]["active_provider_id"] = Value::String("not-a-real-video-provider".into());
    let res = reqwest::Client::new()
        .post(server.url("/settings/v2"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 400);
}
