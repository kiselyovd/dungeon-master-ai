//! Local LLM manifest endpoint backing the Settings -> Provider tab
//! ModelSelector container (Task 14 of M9-DM).
//!
//! Endpoints:
//! - `GET  /local-llm/manifest`      -> system catalog + user manifest + install state
//! - `POST /local-llm/active-model`  -> set the currently-selected local model
//!
//! The system catalog is derived from `crates/app-server/src/models/manifest.rs`
//! by walking the Qwen3.5 family entries. The user manifest is empty for now
//! (HF search lands in Tasks 15-19). Install state is reconstructed by asking
//! the `DownloadManager` for the per-id status of every known entry, since the
//! manager does not expose a bulk snapshot helper yet.
//!
//! The set-active-model endpoint reuses `LocalModeConfig::selected_llm`. We
//! accept any string id from the frontend, map it onto a `ModelId` via the
//! Qwen3.5 family table, and reject unknown ids with `BadRequest`. Custom HF
//! repos land in M9-DM Task 19.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use app_domain::local_llm::manifest::{SystemEntry, UserEntry};

use crate::error::AppError;
use crate::models::manager::DownloadStatus;
use crate::models::manifest::{manifest_for, ModelId};
use crate::state::AppState;

/// Per-model download state in the wire shape consumed by the React store.
/// `state` is one of: `"idle" | "queued" | "downloading" | "verifying" | "error"`.
#[derive(Debug, Clone, Serialize)]
pub struct DownloadStateWire {
    pub state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<f32>,
    #[serde(rename = "errorMessage", skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManifestResponse {
    pub system: Vec<SystemEntry>,
    pub user: Vec<UserEntry>,
    pub installed_ids: Vec<String>,
    pub download_states: HashMap<String, DownloadStateWire>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetActiveModelRequest {
    pub id: String,
}

/// Curated local-LLM system catalog. Hardcoded against the Qwen3.5 family in
/// `MANIFEST` (see `crates/app-server/src/models/manifest.rs`). Reads sizes,
/// hf_repo, and hf_filename through `manifest_for(...)` so any future bump
/// to the static manifest flows through automatically; the only fields kept
/// inline are arch, quant, license, and display_name (not present on
/// `ModelManifest`).
fn system_catalog() -> Vec<SystemEntry> {
    const ENTRIES: &[(ModelId, &str, &str, &str, &str)] = &[
        (
            ModelId::Qwen3_5_0_8b,
            "qwen3.5-0.8b",
            "qwen3",
            "gguf-q4_k_m",
            "apache-2.0",
        ),
        (
            ModelId::Qwen3_5_2b,
            "qwen3.5-2b",
            "qwen3",
            "gguf-q4_k_m",
            "apache-2.0",
        ),
        (
            ModelId::Qwen3_5_4b,
            "qwen3.5-4b",
            "qwen3",
            "gguf-q4_k_m",
            "apache-2.0",
        ),
        (
            ModelId::Qwen3_5_9b,
            "qwen3.5-9b",
            "qwen3",
            "gguf-q4_k_m",
            "apache-2.0",
        ),
    ];

    ENTRIES
        .iter()
        .filter_map(|(id, wire_id, arch, quant, license)| {
            let m = manifest_for(id)?;
            // 1 GiB = 1_073_741_824 bytes. Round to one decimal for display.
            let gb = (m.size_bytes_estimate as f64 / (1024.0 * 1024.0 * 1024.0)) as f32;
            let gb_round = (gb * 10.0).round() / 10.0;
            Some(SystemEntry {
                id: (*wire_id).into(),
                hf_repo: m.hf_repo.into(),
                hf_filename: m.hf_filename.into(),
                arch: (*arch).into(),
                quant: (*quant).into(),
                size_gb: gb_round,
                license: (*license).into(),
                display_name: m.display_name.into(),
            })
        })
        .collect()
}

/// Map a wire id (e.g. `"qwen3.5-4b"`) back onto a `ModelId`. Returns `None`
/// for unknown ids; the route turns that into a 400.
fn model_id_for_wire(wire_id: &str) -> Option<ModelId> {
    match wire_id {
        "qwen3.5-0.8b" => Some(ModelId::Qwen3_5_0_8b),
        "qwen3.5-2b" => Some(ModelId::Qwen3_5_2b),
        "qwen3.5-4b" => Some(ModelId::Qwen3_5_4b),
        "qwen3.5-9b" => Some(ModelId::Qwen3_5_9b),
        _ => None,
    }
}

pub async fn get_manifest(State(state): State<AppState>) -> Json<ManifestResponse> {
    let system = system_catalog();
    let user: Vec<UserEntry> = Vec::new();
    // TODO(M9-DM): wire user manifest from secrets/HF search (Task 19).

    let manager = state.download_manager();
    let mut installed_ids: Vec<String> = Vec::new();
    let mut download_states: HashMap<String, DownloadStateWire> = HashMap::new();
    for entry in &system {
        let Some(model_id) = model_id_for_wire(&entry.id) else {
            continue;
        };
        let status = manager.status(model_id).await;
        match status {
            DownloadStatus::Completed { .. } => {
                installed_ids.push(entry.id.clone());
            }
            DownloadStatus::Downloading {
                bytes_done,
                total_bytes,
            } => {
                let progress = total_bytes
                    .filter(|t| *t > 0)
                    .map(|t| (bytes_done as f32) / (t as f32));
                download_states.insert(
                    entry.id.clone(),
                    DownloadStateWire {
                        state: "downloading",
                        progress,
                        error_message: None,
                    },
                );
            }
            DownloadStatus::Failed { reason } => {
                download_states.insert(
                    entry.id.clone(),
                    DownloadStateWire {
                        state: "error",
                        progress: None,
                        error_message: Some(reason),
                    },
                );
            }
            DownloadStatus::Idle => {
                // No entry => treated as idle on the wire (default).
            }
        }
    }

    Json(ManifestResponse {
        system,
        user,
        installed_ids,
        download_states,
    })
}

pub async fn set_active_model(
    State(state): State<AppState>,
    Json(req): Json<SetActiveModelRequest>,
) -> Result<StatusCode, AppError> {
    let model_id = model_id_for_wire(&req.id)
        .ok_or_else(|| AppError::BadRequest(format!("unknown local model id: {}", req.id)))?;
    let mut cfg = state.local_mode_config();
    cfg.selected_llm = model_id;
    state.set_local_mode_config(cfg);
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_catalog_returns_four_qwen_entries() {
        let s = system_catalog();
        assert_eq!(s.len(), 4);
        let ids: Vec<&str> = s.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["qwen3.5-0.8b", "qwen3.5-2b", "qwen3.5-4b", "qwen3.5-9b"]
        );
        // Each entry's hf_repo / hf_filename should match the manifest's
        // canonical values, not made-up strings.
        let four_b = s.iter().find(|e| e.id == "qwen3.5-4b").unwrap();
        assert_eq!(four_b.hf_repo, "Qwen/Qwen3.5-4B-Instruct-GGUF");
        assert_eq!(four_b.hf_filename, "qwen3.5-4b-instruct-q4_k_m.gguf");
    }

    #[test]
    fn model_id_round_trip() {
        assert_eq!(model_id_for_wire("qwen3.5-4b"), Some(ModelId::Qwen3_5_4b));
        assert_eq!(model_id_for_wire("qwen3.5-9b"), Some(ModelId::Qwen3_5_9b));
        assert_eq!(model_id_for_wire("nope"), None);
    }
}
