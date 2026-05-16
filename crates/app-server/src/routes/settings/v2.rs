//! M7-DM SettingsConfigV2 shape: chat / image / video / behavior. Replaces
//! the M5/M6 single-provider settings shape via hard-cutover migration on
//! first launch (see frontend `migrateLegacySettings`).

use app_llm::Capabilities;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsConfigV2 {
    pub chat: ChatConfig,
    pub image: ImageConfig,
    pub video: VideoConfig,
    pub behavior: BehaviorConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConfig {
    pub active_provider_id: String,
    pub active_model_id: String,
    /// Per-provider config opaque to the routing layer; each provider's
    /// constructor reads its slice. JSON shape is `{ <provider_id>: { ... } }`.
    pub providers: serde_json::Value,
    #[serde(default)]
    pub vision_enabled: bool,
    #[serde(default)]
    pub reasoning_enabled: bool,
    #[serde(default = "default_reasoning_budget")]
    pub reasoning_budget: ReasoningBudget,
    #[serde(default)]
    pub capabilities_override: Option<Capabilities>,
}

fn default_reasoning_budget() -> ReasoningBudget {
    ReasoningBudget::Medium
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningBudget {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageConfig {
    #[serde(default = "_true")]
    pub enabled: bool,
    pub active_provider_id: String,
    pub active_model_id: String,
    pub providers: serde_json::Value,
    pub preset: ImagePreset,
    pub style_lora: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ImagePreset {
    Fast,
    Balanced,
    Quality,
    QualityOss,
    Cloud,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConfig {
    #[serde(default)]
    pub enabled: bool,
    pub active_provider_id: String,
    pub active_model_id: String,
    pub providers: serde_json::Value,
    pub mode: VideoMode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VideoMode {
    Prerecorded,
    Live,
    Race,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorConfig {
    pub system_prompt: String,
    pub temperature: f32,
    pub ui_language: String,
    pub narration_language: String,
    #[serde(default)]
    pub license_restricted_mode: bool,
    #[serde(default = "default_max_rounds")]
    pub agent_max_rounds: u32,
    #[serde(default = "default_scene_transitions")]
    pub scene_transitions: SceneTransitions,
}

fn _true() -> bool {
    true
}
fn default_max_rounds() -> u32 {
    8
}
fn default_scene_transitions() -> SceneTransitions {
    SceneTransitions::Auto
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SceneTransitions {
    Auto,
    Manual,
    Off,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_round_trip() {
        let cfg = SettingsConfigV2 {
            chat: ChatConfig {
                active_provider_id: "anthropic".into(),
                active_model_id: "claude-haiku-4-5-20251001".into(),
                providers: serde_json::json!({}),
                vision_enabled: false,
                reasoning_enabled: false,
                reasoning_budget: ReasoningBudget::Medium,
                capabilities_override: None,
            },
            image: ImageConfig {
                enabled: true,
                active_provider_id: "local-sdxl-lightning".into(),
                active_model_id: "sdxl-lightning-4step".into(),
                providers: serde_json::json!({}),
                preset: ImagePreset::Balanced,
                style_lora: None,
            },
            video: VideoConfig {
                enabled: false,
                active_provider_id: "local-ltx-video".into(),
                active_model_id: "ltx-video-0.9.6-distilled".into(),
                providers: serde_json::json!({}),
                mode: VideoMode::Prerecorded,
            },
            behavior: BehaviorConfig {
                system_prompt: "You are a DM.".into(),
                temperature: 0.7,
                ui_language: "en".into(),
                narration_language: "en".into(),
                license_restricted_mode: false,
                agent_max_rounds: 8,
                scene_transitions: SceneTransitions::Auto,
            },
        };
        let s = serde_json::to_string(&cfg).expect("serialise");
        let back: SettingsConfigV2 = serde_json::from_str(&s).expect("deserialise");
        assert_eq!(back.chat.active_provider_id, "anthropic");
        assert_eq!(back.image.preset, ImagePreset::Balanced);
        assert_eq!(back.video.mode, VideoMode::Prerecorded);
        assert_eq!(back.behavior.scene_transitions, SceneTransitions::Auto);
    }

    #[test]
    fn image_preset_kebab_case_serialisation() {
        let json = serde_json::to_string(&ImagePreset::QualityOss).unwrap();
        assert_eq!(json, "\"quality-oss\"");
    }

    #[test]
    fn reasoning_budget_lowercase_serialisation() {
        let json = serde_json::to_string(&ReasoningBudget::Medium).unwrap();
        assert_eq!(json, "\"medium\"");
    }
}
