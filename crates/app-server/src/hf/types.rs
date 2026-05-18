use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HfSort {
    Downloads,
    Likes,
    LastModified,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SizeBucket {
    Small,  // < 4 GB
    Medium, // 4-8 GB
    Large,  // > 8 GB
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HfSearchQuery {
    pub q: String,
    pub arch: Option<String>,
    pub quant: Option<String>,
    pub size: Option<SizeBucket>,
    pub license: Option<String>,
    pub sort: HfSort,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfSibling {
    #[serde(rename = "rfilename")]
    pub filename: String,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfModel {
    #[serde(rename = "id")]
    pub repo_id: String,
    #[serde(default)]
    pub likes: u64,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub gated: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, rename = "lastModified")]
    pub last_modified: Option<String>,
    #[serde(default)]
    pub siblings: Vec<HfSibling>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfModelDetail {
    #[serde(rename = "id")]
    pub repo_id: String,
    #[serde(default)]
    pub gated: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub siblings: Vec<HfSibling>,
    #[serde(default)]
    pub card_data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfLicenseStatus {
    pub gated: bool,
    pub accepted: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum HfError {
    #[error("invalid HF token")]
    InvalidToken,
    #[error("model not found")]
    NotFound,
    #[error("rate limit; retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },
    #[error("network error: {0}")]
    Network(String),
    #[error("unexpected status {0}")]
    Unexpected(u16),
}
