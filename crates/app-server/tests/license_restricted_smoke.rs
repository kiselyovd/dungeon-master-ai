//! Sub-task #2: prove that `behavior.license_restricted_mode` filters
//! non-OSS (Apache 2.0 / MIT / BSD) providers out of the rebuilt
//! ProviderRegistry, hard-errors when the chat slot is blocked (chat is
//! mandatory), silently drops image/video, and surfaces a
//! `license_restricted_no_compat` warning when restriction leaves no
//! usable media providers.

use app_server::routes::settings::post_settings_v2;
use app_server::routes::settings::SettingsConfigV2;
use app_server::testing::new_test_state;
use axum::extract::State;
use axum::Json;
use serde_json::json;

fn cfg_with(license_restricted: bool, chat: serde_json::Value, image: serde_json::Value) -> SettingsConfigV2 {
    cfg_with_video(license_restricted, chat, image, false)
}

fn cfg_with_video(
    license_restricted: bool,
    chat: serde_json::Value,
    image: serde_json::Value,
    video_enabled: bool,
) -> SettingsConfigV2 {
    serde_json::from_value(json!({
        "chat": chat,
        "image": image,
        "video": {
            "enabled": video_enabled,
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
            "license_restricted_mode": license_restricted,
            "agent_max_rounds": 8,
            "scene_transitions": "auto",
        },
    }))
    .expect("config parse")
}

fn local_mistralrs_chat() -> serde_json::Value {
    json!({
        "active_provider_id": "local-mistralrs",
        "active_model_id": "qwen3.5-4b",
        "providers": { "local-mistralrs": { "model_id": "qwen3_5_4b", "port": 8765 } },
        "vision_enabled": false,
        "reasoning_enabled": false,
        "reasoning_budget": "medium",
    })
}

fn anthropic_chat() -> serde_json::Value {
    json!({
        "active_provider_id": "anthropic",
        "active_model_id": "claude-haiku-4-5-20251001",
        "providers": { "anthropic": { "api_key": "sk-ant-test" } },
        "vision_enabled": false,
        "reasoning_enabled": false,
        "reasoning_budget": "medium",
    })
}

fn replicate_image() -> serde_json::Value {
    json!({
        "enabled": true,
        "active_provider_id": "replicate",
        "active_model_id": "stability-ai/sdxl",
        "providers": { "replicate": { "api_key": "r8_test" } },
        "preset": "balanced",
        "style_lora": null,
    })
}

fn local_sdxl_lightning_image() -> serde_json::Value {
    json!({
        "enabled": true,
        "active_provider_id": "local-sdxl-lightning",
        "active_model_id": "sdxl-lightning-4step",
        "providers": {},
        "preset": "balanced",
        "style_lora": null,
    })
}

#[tokio::test]
async fn license_restricted_blocks_non_oss_chat_provider() {
    let state = new_test_state().await;
    let cfg = cfg_with(true, anthropic_chat(), local_sdxl_lightning_image());

    let res = post_settings_v2(State(state.clone()), Json(cfg)).await;
    let err = res.expect_err("anthropic chat must be blocked under license_restricted_mode");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("license_restricted_mode") && msg.contains("anthropic"),
        "expected BadRequest mentioning anthropic + license_restricted_mode, got: {msg}"
    );
}

#[tokio::test]
async fn license_unrestricted_keeps_all_providers() {
    let state = new_test_state().await;
    let cfg = cfg_with(false, anthropic_chat(), replicate_image());

    let res = post_settings_v2(State(state.clone()), Json(cfg))
        .await
        .expect("post_settings_v2 should succeed without restriction");

    let body = res.0;
    assert_eq!(body["status"], "ok");
    assert_eq!(body["license_restricted_no_compat"], false);

    let reg = state.registry();
    assert_eq!(reg.chat.name(), "anthropic");
    assert!(reg.image.is_some(), "replicate image provider should survive");
}

#[tokio::test]
async fn license_restricted_filters_non_oss_image_silently() {
    let state = new_test_state().await;
    // local-mistralrs chat (OSS Apache 2.0 (Qwen)) + replicate image (non-OSS "varies per model")
    // + restricted = chat survives, image filtered to None, warning fires.
    let cfg = cfg_with(true, local_mistralrs_chat(), replicate_image());

    let res = post_settings_v2(State(state.clone()), Json(cfg))
        .await
        .expect("post_settings_v2 should succeed when only image gets filtered");

    let body = res.0;
    assert_eq!(body["status"], "ok");
    assert_eq!(
        body["license_restricted_no_compat"], true,
        "warning must fire when restriction silently drops an enabled image provider"
    );

    let reg = state.registry();
    assert_eq!(reg.chat.name(), "local-mistralrs");
    assert!(reg.image.is_none(), "non-OSS image provider must be filtered out");
}

#[tokio::test]
async fn license_restricted_with_video_enabled_filters_ltx() {
    let state = new_test_state().await;
    // local-mistralrs chat (OSS) + local-sdxl-lightning image (OSS but build_image_provider
    // fails without sidecar) - so we disable image here and just test video filtering.
    // Video=enabled + LTX-Video (non-OSS OpenRAIL-M) + restricted -> video=None, warning=true.
    let mut image = local_sdxl_lightning_image();
    image["enabled"] = json!(false);
    let cfg = cfg_with_video(true, local_mistralrs_chat(), image, true);

    let res = post_settings_v2(State(state.clone()), Json(cfg))
        .await
        .expect("post_settings_v2 should succeed");

    let body = res.0;
    assert_eq!(body["status"], "ok");
    assert_eq!(
        body["license_restricted_no_compat"], true,
        "warning fires because video was enabled but LTX got filtered out"
    );

    let reg = state.registry();
    assert_eq!(reg.chat.name(), "local-mistralrs");
    assert!(reg.video.is_none(), "LTX-Video must be filtered under restriction");
}
