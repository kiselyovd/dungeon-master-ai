use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeState {
    #[default]
    Stopped,
    Starting,
    Running,
    Degraded,
    Failed,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeStatus {
    pub state: RuntimeState,
    pub model_id: Option<String>,
    pub image_enabled: bool,
    pub video_enabled: bool,
    pub failure_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeStartRequest {
    pub model_id: String,
    pub enable_image: bool,
    pub enable_video: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemEntry {
    pub id: String,
    pub hf_repo: String,
    pub hf_filename: String,
    pub arch: String,
    pub quant: String,
    pub size_gb: f32,
    pub license: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserEntry {
    #[serde(flatten)]
    pub system: SystemEntry,
    pub added_at: String,
    pub source: String,
}
