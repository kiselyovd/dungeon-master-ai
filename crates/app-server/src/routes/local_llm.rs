//! Local LLM manifest endpoint backing the Settings -> Provider tab
//! ModelSelector container (Task 14 of M9-DM).
//!
//! Endpoints:
//! - `GET  /local-llm/manifest`         -> system catalog + user manifest + install state
//! - `POST /local-llm/active-model`     -> set the currently-selected local model
//! - `POST /local-llm/download/:model_id`  -> start a download (202 Accepted)
//! - `DELETE /local-llm/model/:model_id`   -> cancel in-progress or delete installed (204)
//! - `GET  /local-llm/download-events`  -> SSE stream of DownloadEventWire messages
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

use std::collections::HashMap;
use std::convert::Infallible;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::Json;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use app_domain::local_llm::manifest::{SystemEntry, UserEntry};

use crate::error::AppError;
use crate::models::download::DownloadEvent;
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

/// Wire shape for a download event sent over the SSE stream.
///
/// `id` is the dotted wire id (e.g. `"qwen3.5-4b"`) that matches the manifest
/// keys used by the frontend. The `kind` field mirrors `DownloadEvent`'s tag.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct DownloadEventWire {
    /// Dotted wire id consistent with the system catalog (e.g. `"qwen3.5-4b"`).
    pub(crate) id: String,
    pub(crate) kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) bytes_done: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) total_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reason: Option<String>,
}

/// Translate a `ModelId` enum variant into the dotted wire id string used by
/// the manifest and the frontend store. Returns `None` for ids not yet exposed
/// in the system catalog (image/video models are not part of the local-llm
/// settings surface).
///
/// The mapping is the exact inverse of `model_id_for_wire` - both functions
/// must stay in sync whenever new Qwen entries are added to the catalog.
pub(crate) fn wire_id_for_model(id: &ModelId) -> Option<&'static str> {
    match id {
        ModelId::Qwen3_5_0_8b => Some("qwen3.5-0.8b"),
        ModelId::Qwen3_5_2b => Some("qwen3.5-2b"),
        ModelId::Qwen3_5_4b => Some("qwen3.5-4b"),
        ModelId::Qwen3_5_9b => Some("qwen3.5-9b"),
        _ => None,
    }
}

/// Convert a `DownloadEvent` into the `DownloadEventWire` shape for SSE
/// delivery. Events whose `ModelId` has no catalog wire id are dropped (returns
/// `None`); the SSE handler skips `None`s.
fn to_wire(ev: DownloadEvent) -> Option<DownloadEventWire> {
    match ev {
        DownloadEvent::Progress {
            id,
            bytes_done,
            total_bytes,
        } => {
            let wire_id = wire_id_for_model(&id)?.to_owned();
            Some(DownloadEventWire {
                id: wire_id,
                kind: "progress",
                bytes_done: Some(bytes_done),
                total_bytes,
                reason: None,
            })
        }
        DownloadEvent::Completed { id, .. } => {
            let wire_id = wire_id_for_model(&id)?.to_owned();
            Some(DownloadEventWire {
                id: wire_id,
                kind: "completed",
                bytes_done: None,
                total_bytes: None,
                reason: None,
            })
        }
        DownloadEvent::Failed { id, reason } => {
            let wire_id = wire_id_for_model(&id)?.to_owned();
            Some(DownloadEventWire {
                id: wire_id,
                kind: "failed",
                bytes_done: None,
                total_bytes: None,
                reason: Some(reason),
            })
        }
    }
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

/// POST /local-llm/download/:model_id
///
/// Kicks off a background download for the given wire id. Returns 202 Accepted
/// immediately; progress flows over the SSE endpoint. Returns 400 for unknown
/// ids, 500 if the `DownloadManager` reports an internal start error.
pub async fn start_download(
    State(state): State<AppState>,
    Path(model_id): Path<String>,
) -> Result<StatusCode, AppError> {
    let id = model_id_for_wire(&model_id)
        .ok_or_else(|| AppError::BadRequest(format!("unknown local model id: {model_id}")))?;
    let manager = state.download_manager();
    manager
        .start(id)
        .await
        .map_err(|e| AppError::Internal(format!("download start failed: {e}")))?;
    Ok(StatusCode::ACCEPTED)
}

/// DELETE /local-llm/model/:model_id
///
/// Cancels an in-progress download or removes an installed model. Idempotent:
/// calling on an idle model just ensures any partial files are cleaned up.
/// Returns 204 No Content. Returns 400 for unknown ids.
pub async fn cancel_or_delete(
    State(state): State<AppState>,
    Path(model_id): Path<String>,
) -> Result<StatusCode, AppError> {
    let id = model_id_for_wire(&model_id)
        .ok_or_else(|| AppError::BadRequest(format!("unknown local model id: {model_id}")))?;
    let manager = state.download_manager();
    manager.cancel(id).await;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /local-llm/download-events
///
/// SSE stream. Subscribes to the `DownloadManager`'s broadcast channel and
/// forwards each event to the client as a JSON-encoded `DownloadEventWire`.
/// Events for model ids not in the system catalog are silently dropped.
/// The stream ends when the client disconnects or the broadcast channel closes.
pub async fn download_events(State(state): State<AppState>) -> impl IntoResponse {
    let rx = state.download_manager().events.subscribe();
    let broadcast_stream = BroadcastStream::new(rx);

    let sse_stream: Box<dyn Stream<Item = Result<Event, Infallible>> + Send + Unpin> =
        Box::new(Box::pin(broadcast_stream.filter_map(|res| {
            // Lagged(n): a slow client silently skips missed progress ticks;
            // the stream stays alive.
            let ev = res.ok()?;
            let wire = to_wire(ev)?;
            let event = Event::default().event("download").json_data(wire).ok()?;
            Some(Ok::<Event, Infallible>(event))
        })));

    Sse::new(sse_stream).keep_alive(KeepAlive::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_id_for_model_round_trips_all_qwen_variants() {
        // Verify that wire_id_for_model is the exact inverse of model_id_for_wire
        // for every id in the system catalog. A mismatch here would silently
        // break progress-bar keying on the frontend.
        let cases = [
            (ModelId::Qwen3_5_0_8b, "qwen3.5-0.8b"),
            (ModelId::Qwen3_5_2b, "qwen3.5-2b"),
            (ModelId::Qwen3_5_4b, "qwen3.5-4b"),
            (ModelId::Qwen3_5_9b, "qwen3.5-9b"),
        ];
        for (id, expected_wire) in &cases {
            assert_eq!(
                wire_id_for_model(id),
                Some(*expected_wire),
                "wire_id_for_model({id:?}) should be {expected_wire}"
            );
            // inverse: model_id_for_wire must recover the original id
            assert_eq!(
                model_id_for_wire(expected_wire),
                Some(id.clone()),
                "model_id_for_wire({expected_wire}) should recover {id:?}"
            );
        }
    }

    #[test]
    fn wire_id_for_model_returns_none_for_image_video_ids() {
        // Image and video model ids are not part of the local-llm catalog surface.
        assert_eq!(wire_id_for_model(&ModelId::SdxlLightning4StepLora), None);
        assert_eq!(wire_id_for_model(&ModelId::NunchakuFluxDevInt4), None);
        assert_eq!(wire_id_for_model(&ModelId::LtxVideo09_6Distilled), None);
    }

    #[test]
    fn to_wire_translates_progress_event() {
        let ev = crate::models::download::DownloadEvent::Progress {
            id: ModelId::Qwen3_5_4b,
            bytes_done: 1024,
            total_bytes: Some(4096),
        };
        let wire = to_wire(ev).expect("Qwen3_5_4b must produce a wire event");
        assert_eq!(wire.id, "qwen3.5-4b");
        assert_eq!(wire.kind, "progress");
        assert_eq!(wire.bytes_done, Some(1024));
        assert_eq!(wire.total_bytes, Some(4096));
    }

    #[test]
    fn to_wire_drops_events_for_non_catalog_ids() {
        let ev = crate::models::download::DownloadEvent::Completed {
            id: ModelId::NunchakuFluxDevInt4,
            bytes_total: 100,
        };
        assert!(
            to_wire(ev).is_none(),
            "non-catalog ids must be dropped by to_wire"
        );
    }

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
