//! HTTP handlers for `/hf/*` endpoints.
//!
//! Token storage lives in `crate::secrets` under a single well-known key so
//! the same vault row backs all surfaces (HTTP, future CLI). Each handler
//! reads the token at call time rather than caching it on `AppState` so
//! deleting via `DELETE /hf/token` takes effect immediately for the next
//! search or license-check call.
//!
//! NOTE on `repo_id`: HF repo ids contain a `/` separator (e.g.
//! `Qwen/Qwen3-4B`). Axum's default `Path` extractor stops at the first `/`,
//! and the wildcard form `{*repo_id}` is only allowed as the **last** path
//! segment. The route for `license_check` therefore reads
//! `/hf/model/license/{*repo_id}` rather than the more natural
//! `/hf/model/{repo_id}/license` so the wildcard sits at the tail. The
//! frontend passes the id verbatim without URL-encoding the slash.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::hf::client::HfClient;
use crate::hf::manifest as user_manifest;
use crate::hf::types::{HfModel, HfSearchQuery, HfSort, SizeBucket};
use crate::secrets::{clear_hf_token, get_hf_token, set_hf_token};
use crate::state::AppState;
use app_domain::local_llm::manifest::{SystemEntry, UserEntry};

/// `user_manifest.json` sits next to `models_dir` (one level up so the data
/// root holds both the per-model directories and the manifest index). Falls
/// back to `.` when the parent cannot be resolved, which can only happen for
/// a degenerate root path like `/` that production configs never produce.
pub(crate) fn user_manifest_path(state: &AppState) -> std::path::PathBuf {
    let dir = state.models_dir();
    dir.parent()
        .unwrap_or(std::path::Path::new("."))
        .join("user_manifest.json")
}

async fn make_client(state: &AppState) -> HfClient {
    let token = get_hf_token(state.secrets_repo().as_ref()).await;
    HfClient::new(token)
}

#[derive(Debug, Deserialize)]
pub struct TokenBody {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct TokenStatus {
    pub connected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
}

/// Surface a short, non-reversible prefix of the stored token so the UI can
/// confirm "yes, the right one is loaded" without leaking the full value back
/// to JS. Tokens shorter than 8 chars are echoed verbatim because there is
/// nothing meaningful to mask.
fn prefix(token: &str) -> String {
    if token.len() <= 8 {
        return token.into();
    }
    let head = &token[..4];
    let tail = &token[token.len() - 4..];
    format!("{head}...{tail}")
}

pub async fn post_token(
    State(state): State<AppState>,
    Json(body): Json<TokenBody>,
) -> Result<Json<TokenStatus>, AppError> {
    if body.token.trim().is_empty() {
        return Err(AppError::BadRequest("empty token".into()));
    }
    set_hf_token(state.secrets_repo().as_ref(), &body.token)
        .await
        .map_err(|e| AppError::Internal(format!("store hf token: {e}")))?;
    Ok(Json(TokenStatus {
        connected: true,
        prefix: Some(prefix(&body.token)),
    }))
}

pub async fn delete_token(State(state): State<AppState>) -> Result<StatusCode, AppError> {
    clear_hf_token(state.secrets_repo().as_ref())
        .await
        .map_err(|e| AppError::Internal(format!("clear hf token: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_token_status(State(state): State<AppState>) -> Json<TokenStatus> {
    let tok = get_hf_token(state.secrets_repo().as_ref()).await;
    match tok {
        Some(t) => Json(TokenStatus {
            connected: true,
            prefix: Some(prefix(&t)),
        }),
        None => Json(TokenStatus {
            connected: false,
            prefix: None,
        }),
    }
}

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
    #[serde(default)]
    pub arch: Option<String>,
    #[serde(default)]
    pub quant: Option<String>,
    #[serde(default)]
    pub size: Option<SizeBucket>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub sort: Option<HfSort>,
}

/// Map an HF client error onto the HTTP error surface. A bad/expired token is
/// the user's problem (400), not a server fault (500); a missing model is 404.
fn hf_err_to_app(e: crate::hf::types::HfError) -> AppError {
    use crate::hf::types::HfError;
    match e {
        HfError::InvalidToken => {
            AppError::BadRequest("HuggingFace authorization failed - check your token".into())
        }
        HfError::NotFound => AppError::NotFound,
        other => AppError::Internal(format!("hf: {other}")),
    }
}

pub async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Vec<HfModel>>, AppError> {
    let client = make_client(&state).await;
    let q = HfSearchQuery {
        q: params.q,
        arch: params.arch,
        quant: params.quant,
        size: params.size,
        license: params.license,
        sort: params.sort.unwrap_or(HfSort::Downloads),
    };
    client.search(q).await.map(Json).map_err(hf_err_to_app)
}

pub async fn license_check(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let client = make_client(&state).await;
    let status = client
        .check_license(&repo_id)
        .await
        .map_err(hf_err_to_app)?;
    Ok(Json(status))
}

#[derive(Debug, Deserialize)]
pub struct AddManifestBody {
    pub repo_id: String,
    pub hf_filename: String,
    pub arch: String,
    pub quant: String,
    pub size_gb: f32,
    pub license: String,
    pub display_name: String,
    /// When `true`, skip the arch/quant compat whitelist. Used by power users
    /// who know the sidecar will accept a non-curated file; the UI surfaces a
    /// confirmation modal before flipping this flag.
    #[serde(default)]
    pub force: bool,
}

/// Lower-case ASCII alphanumeric, with every other character collapsed to a
/// single underscore. Produces stable, filesystem-safe ids; collisions are
/// caught by the manifest's duplicate-id check.
fn slugify(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

pub async fn add_manifest(
    State(state): State<AppState>,
    Json(body): Json<AddManifestBody>,
) -> Result<StatusCode, AppError> {
    if !body.force {
        if !crate::hf::compat::is_compat_arch(&body.arch) {
            return Err(AppError::BadRequest(format!(
                "unsupported arch: {}",
                body.arch
            )));
        }
        if !crate::hf::compat::is_compat_quant(&body.hf_filename) {
            return Err(AppError::BadRequest(format!(
                "unsupported quant/file: {}",
                body.hf_filename
            )));
        }
    }
    let id = format!("{}_{}", slugify(&body.repo_id), slugify(&body.quant));
    let entry = UserEntry {
        system: SystemEntry {
            id: id.clone(),
            hf_repo: body.repo_id,
            hf_filename: body.hf_filename,
            arch: body.arch,
            quant: body.quant,
            size_gb: body.size_gb,
            license: body.license,
            display_name: body.display_name,
        },
        added_at: chrono::Utc::now().to_rfc3339(),
        source: "hf-search".into(),
    };
    let path = user_manifest_path(&state);
    user_manifest::add_entry(&path, entry)
        .map_err(|e| AppError::BadRequest(format!("manifest add: {e}")))?;
    Ok(StatusCode::CREATED)
}

pub async fn delete_manifest(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let path = user_manifest_path(&state);
    user_manifest::remove_entry(&path, &id)
        .map_err(|e| AppError::Internal(format!("manifest remove: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_masks_long_token() {
        let p = prefix("hf_abcdefghij1234567890");
        assert!(p.starts_with("hf_a"));
        assert!(p.ends_with("7890"));
        assert!(p.contains("..."));
    }

    #[test]
    fn prefix_passes_through_short_token() {
        assert_eq!(prefix("short"), "short");
        assert_eq!(prefix("12345678"), "12345678");
    }

    #[test]
    fn slugify_lowers_and_collapses_separators() {
        assert_eq!(slugify("Qwen/Qwen3-4B"), "qwen_qwen3_4b");
        assert_eq!(slugify("gguf-q4_k_m"), "gguf_q4_k_m");
    }

    #[test]
    fn invalid_token_maps_to_bad_request() {
        use crate::hf::types::HfError;
        let app = hf_err_to_app(HfError::InvalidToken);
        assert!(matches!(app, AppError::BadRequest(_)), "got {app:?}");
    }

    #[test]
    fn not_found_maps_to_not_found() {
        use crate::hf::types::HfError;
        assert!(matches!(
            hf_err_to_app(HfError::NotFound),
            AppError::NotFound
        ));
    }
}
