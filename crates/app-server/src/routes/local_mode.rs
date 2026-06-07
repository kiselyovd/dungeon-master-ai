//! Local-mode HTTP surface: configuration, model downloads, runtime control.
//!
//! Endpoints:
//! - `GET  /local-mode/config`            -> current selected model + VRAM strategy
//! - `POST /local-mode/config`            -> update selected model + VRAM strategy
//! - `POST /local/download/{id}`          -> kick off resumable HF download
//! - `DELETE /local/download/{id}`        -> cancel + delete partial file/dir
//! - `GET  /local/download/{id}/progress` -> SSE stream of progress/completed/failed
//! - `POST /local/runtime/start`          -> spawn LLM (+ optional image) sidecar
//! - `POST /local/runtime/stop`           -> stop both runtimes
//! - `GET  /local/runtime/status`         -> snapshot of both LocalRuntime states

use std::convert::Infallible;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::Json;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::error::AppError;
use crate::local_runtime::{port::discover_free_port, RegistrySnapshot};
use crate::models::manifest::{manifest_for, ModelId, ModelKind};
use crate::models::DownloadEvent;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModeConfig {
    pub selected_llm: ModelId,
    pub vram_strategy: VramStrategy,
}

impl Default for LocalModeConfig {
    fn default() -> Self {
        Self {
            // Gemma 4 E2B (AutoIsq) is the default: the deprecated mistralrs-server
            // build we ship SEGFAULTS loading a GGUF model unless stdout is a real
            // TTY (verified live), and the backend always pipes it - so GGUF models
            // (Qwen3-8B) cannot be the default until mistralrs is upgraded. The
            // AutoIsq `run` path does not hit that crash.
            selected_llm: ModelId::Gemma4E2bIt,
            vram_strategy: VramStrategy::AutoSwap,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum VramStrategy {
    AutoSwap,
    KeepBothLoaded,
    DisableImageGen,
}

pub async fn get_config(State(state): State<AppState>) -> Json<LocalModeConfig> {
    Json(state.local_mode_config())
}

pub async fn post_config(
    State(state): State<AppState>,
    Json(cfg): Json<LocalModeConfig>,
) -> Result<Json<LocalModeConfig>, AppError> {
    state.set_local_mode_config(cfg.clone());
    Ok(Json(cfg))
}

pub async fn post_download(
    State(state): State<AppState>,
    Path(model_id): Path<ModelId>,
) -> Result<StatusCode, AppError> {
    state
        .download_manager()
        .start(model_id)
        .await
        .map_err(AppError::BadRequest)?;
    Ok(StatusCode::ACCEPTED)
}

pub async fn delete_download(
    State(state): State<AppState>,
    Path(model_id): Path<ModelId>,
) -> StatusCode {
    state.download_manager().cancel(model_id).await;
    StatusCode::NO_CONTENT
}

pub async fn download_progress(
    State(state): State<AppState>,
    Path(model_id): Path<ModelId>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.download_manager().events.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(
        move |res: Result<
            DownloadEvent,
            tokio_stream::wrappers::errors::BroadcastStreamRecvError,
        >| {
            let ev = res.ok()?;
            let event_id = match &ev {
                DownloadEvent::Progress { id, .. }
                | DownloadEvent::Completed { id, .. }
                | DownloadEvent::Failed { id, .. } => id,
            };
            if event_id != &model_id {
                return None;
            }
            let payload = serde_json::to_string(&ev).ok()?;
            Some(Ok(Event::default().data(payload)))
        },
    );
    Sse::new(stream).keep_alive(KeepAlive::default())
}

pub async fn runtime_status(State(state): State<AppState>) -> Json<RegistrySnapshot> {
    Json(state.runtime_status().await)
}

pub async fn runtime_start(State(state): State<AppState>) -> Result<StatusCode, AppError> {
    let cfg = state.local_mode_config();
    let model = manifest_for(&cfg.selected_llm)
        .ok_or_else(|| AppError::BadRequest("unknown selected_llm".into()))?;
    let port = discover_free_port().map_err(|e| AppError::Internal(e.to_string()))?;
    // Spawn args depend on the model kind. AutoIsq (Gemma 4 safetensors) loads
    // via mistralrs' auto-loader + ISQ and is fetched from HF on first start, so
    // there is no local file to require. GGUF models select the file via the
    // `gguf` subcommand (dir + filename) and must be downloaded first.
    let llm_args = match &model.kind {
        ModelKind::AutoIsq { isq } => {
            crate::local_runtime::mistralrs_run_args(port, model.hf_repo, isq)
        }
        _ => {
            let models_dir = state.models_dir();
            let llm_path = models_dir.join(model.hf_filename);
            if !llm_path.exists() {
                return Err(AppError::BadRequest(format!(
                    "model not downloaded: {}",
                    model.display_name
                )));
            }
            let models_dir_str = models_dir.to_string_lossy().into_owned();
            crate::local_runtime::mistralrs_gguf_args(port, &models_dir_str, model.hf_filename)
        }
    };
    let llm_arg_refs: Vec<&str> = llm_args.iter().map(String::as_str).collect();
    state
        .runtime_registry()
        .llm
        .start_with_retry("mistralrs-server", &llm_arg_refs, port, 3)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    if !matches!(cfg.vram_strategy, VramStrategy::DisableImageGen) {
        let img_port = discover_free_port().map_err(|e| AppError::Internal(e.to_string()))?;
        let img_port_str = img_port.to_string();
        // The Python sidecar declares --weights-dir as required=True
        // (sidecar/app.py:113); without it the process argparse-exits before
        // it can serve /healthz. models_dir holds the per-backend weight
        // subdirectories the sidecar's PipelineDispatcher resolves.
        let weights_dir_str = state.models_dir().to_string_lossy().into_owned();
        let img_args: &[&str] = &["--port", &img_port_str, "--weights-dir", &weights_dir_str];
        state
            .runtime_registry()
            .image
            .start_with_retry("dmai-image-sidecar", img_args, img_port, 3)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        state.set_media_sidecar_url(Some(format!("http://127.0.0.1:{img_port}")));
    }
    Ok(StatusCode::OK)
}

pub async fn runtime_stop(State(state): State<AppState>) -> StatusCode {
    let _ = state.runtime_registry().llm.stop().await;
    let _ = state.runtime_registry().image.stop().await;
    state.set_media_sidecar_url(None);
    StatusCode::OK
}
