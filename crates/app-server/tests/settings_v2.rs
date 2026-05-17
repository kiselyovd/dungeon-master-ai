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
            "active_provider_id": "local-sdxl-lightning",
            "active_model_id": "sdxl-lightning-4step",
            "providers": {},
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
