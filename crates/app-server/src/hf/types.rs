use serde::{Deserialize, Deserializer, Serialize};

/// HF's `gated` field is tri-state: `false` (open) or the strings `"auto"` /
/// `"manual"` (gated). `full=true` search results use the string forms, so a
/// plain `bool` field fails to parse and 500s the whole search. Coerce any
/// non-`false` value to `true`.
fn de_gated<'de, D>(d: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Gated {
        Bool(bool),
        Str(String),
    }
    Ok(match Gated::deserialize(d)? {
        Gated::Bool(b) => b,
        Gated::Str(s) => !s.is_empty() && s != "false",
    })
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
// kebab-case so the frontend's `sort=last-modified` query value deserialises
// (lowercase would expect `lastmodified`). `downloads`/`likes` are unchanged.
#[serde(rename_all = "kebab-case")]
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

// On every field below the rename is SPLIT: deserialize reads HF's own key
// (`id`/`lastModified`/`rfilename`), serialize emits the frontend contract key
// (`repo_id`/`last_modified`/`filename`, matching `src/api/hf.ts`). A symmetric
// `rename` would echo HF's keys back to JS, leaving `model.repo_id` undefined
// and every search card blank (audit blocker 2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfSibling {
    #[serde(rename(serialize = "filename", deserialize = "rfilename"))]
    pub filename: String,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfModel {
    #[serde(rename(serialize = "repo_id", deserialize = "id"))]
    pub repo_id: String,
    #[serde(default)]
    pub likes: u64,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default, deserialize_with = "de_gated")]
    pub gated: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(
        default,
        rename(serialize = "last_modified", deserialize = "lastModified")
    )]
    pub last_modified: Option<String>,
    #[serde(default)]
    pub siblings: Vec<HfSibling>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfModelDetail {
    #[serde(rename(serialize = "repo_id", deserialize = "id"))]
    pub repo_id: String,
    #[serde(default, deserialize_with = "de_gated")]
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
