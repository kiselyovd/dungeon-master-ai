use std::collections::HashMap;
use std::convert::Infallible;

use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures::{Stream, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio_stream::wrappers::ReceiverStream;

use crate::services::{
    HttpOutcome, HttpServiceError, HttpServices, LocalControlOperation, LocalEventStream,
};

#[derive(Debug, Deserialize)]
pub struct TokenBody {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct SetActiveModelRequest {
    pub id: String,
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
    #[serde(default)]
    pub force: bool,
}

fn response(outcome: HttpOutcome) -> Result<Response, HttpServiceError> {
    let status = StatusCode::from_u16(outcome.status).map_err(|_| HttpServiceError::Internal {
        code: "status_invalid",
    })?;
    Ok(match outcome.body {
        Some(body) => (status, Json(body)).into_response(),
        None => status.into_response(),
    })
}

async fn execute(
    services: HttpServices,
    operation: LocalControlOperation,
) -> Result<Response, HttpServiceError> {
    response(services.local_control.execute(operation).await?)
}

pub async fn post_token(
    Extension(s): Extension<HttpServices>,
    Json(body): Json<TokenBody>,
) -> Result<Response, HttpServiceError> {
    if body.token.trim().is_empty() {
        return Err(HttpServiceError::BadRequest {
            code: "token_empty",
        });
    }
    execute(s, LocalControlOperation::HfSetToken { token: body.token }).await
}
pub async fn delete_token(
    Extension(s): Extension<HttpServices>,
) -> Result<Response, HttpServiceError> {
    execute(s, LocalControlOperation::HfDeleteToken).await
}
pub async fn get_token_status(
    Extension(s): Extension<HttpServices>,
) -> Result<Response, HttpServiceError> {
    execute(s, LocalControlOperation::HfTokenStatus).await
}
pub async fn search(
    Extension(s): Extension<HttpServices>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Response, HttpServiceError> {
    if query.get("q").is_none_or(|value| value.trim().is_empty()) {
        return Err(HttpServiceError::BadRequest {
            code: "search_query_empty",
        });
    }
    execute(
        s,
        LocalControlOperation::HfSearch {
            query: serde_json::to_value(query).map_err(|_| HttpServiceError::BadRequest {
                code: "search_query_invalid",
            })?,
        },
    )
    .await
}
pub async fn license_check(
    Extension(s): Extension<HttpServices>,
    Path(repo_id): Path<String>,
) -> Result<Response, HttpServiceError> {
    execute(s, LocalControlOperation::HfLicense { repo_id }).await
}
pub async fn add_manifest(
    Extension(s): Extension<HttpServices>,
    Json(body): Json<AddManifestBody>,
) -> Result<Response, HttpServiceError> {
    execute(s, LocalControlOperation::HfManifestAdd { body: json!({"repo_id":body.repo_id,"hf_filename":body.hf_filename,"arch":body.arch,"quant":body.quant,"size_gb":body.size_gb,"license":body.license,"display_name":body.display_name,"force":body.force}) }).await
}
pub async fn delete_manifest(
    Extension(s): Extension<HttpServices>,
    Path(id): Path<String>,
) -> Result<Response, HttpServiceError> {
    execute(s, LocalControlOperation::HfManifestDelete { id }).await
}

pub async fn get_manifest(
    Extension(s): Extension<HttpServices>,
) -> Result<Response, HttpServiceError> {
    execute(s, LocalControlOperation::LocalLlmManifest).await
}
pub async fn set_active_model(
    Extension(s): Extension<HttpServices>,
    Json(body): Json<SetActiveModelRequest>,
) -> Result<Response, HttpServiceError> {
    execute(s, LocalControlOperation::LocalLlmSetActive { id: body.id }).await
}
pub async fn start_download(
    Extension(s): Extension<HttpServices>,
    Path(id): Path<String>,
) -> Result<Response, HttpServiceError> {
    execute(s, LocalControlOperation::LocalLlmStartDownload { id }).await
}
pub async fn cancel_or_delete(
    Extension(s): Extension<HttpServices>,
    Path(id): Path<String>,
) -> Result<Response, HttpServiceError> {
    execute(s, LocalControlOperation::LocalLlmCancelOrDelete { id }).await
}

fn event_stream(
    receiver: tokio::sync::mpsc::Receiver<Value>,
    named: bool,
) -> impl Stream<Item = Result<Event, Infallible>> {
    ReceiverStream::new(receiver).map(move |value| {
        let event = Event::default();
        let event = if named {
            event.event("download")
        } else {
            event
        };
        Ok(event
            .json_data(value)
            .unwrap_or_else(|_| Event::default().data("{}")))
    })
}
pub async fn download_events(
    Extension(s): Extension<HttpServices>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, HttpServiceError> {
    let receiver = s
        .local_control
        .events(LocalEventStream::LocalLlmDownloads)
        .await?;
    Ok(Sse::new(event_stream(receiver, true)).keep_alive(KeepAlive::default()))
}

pub async fn get_config(
    Extension(s): Extension<HttpServices>,
) -> Result<Response, HttpServiceError> {
    execute(s, LocalControlOperation::LocalModeGetConfig).await
}
pub async fn post_config(
    Extension(s): Extension<HttpServices>,
    Json(config): Json<Value>,
) -> Result<Response, HttpServiceError> {
    execute(s, LocalControlOperation::LocalModeSetConfig { config }).await
}
pub async fn post_local_download(
    Extension(s): Extension<HttpServices>,
    Path(id): Path<String>,
) -> Result<Response, HttpServiceError> {
    execute(s, LocalControlOperation::LocalModeStartDownload { id }).await
}
pub async fn delete_local_download(
    Extension(s): Extension<HttpServices>,
    Path(id): Path<String>,
) -> Result<Response, HttpServiceError> {
    execute(s, LocalControlOperation::LocalModeDeleteDownload { id }).await
}
pub async fn download_progress(
    Extension(s): Extension<HttpServices>,
    Path(id): Path<String>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, HttpServiceError> {
    let receiver = s
        .local_control
        .events(LocalEventStream::LocalModeDownload { id })
        .await?;
    Ok(Sse::new(event_stream(receiver, false)).keep_alive(KeepAlive::default()))
}
pub async fn runtime_start(
    Extension(s): Extension<HttpServices>,
) -> Result<Response, HttpServiceError> {
    execute(s, LocalControlOperation::LocalRuntimeStart).await
}
pub async fn runtime_stop(
    Extension(s): Extension<HttpServices>,
) -> Result<Response, HttpServiceError> {
    execute(s, LocalControlOperation::LocalRuntimeStop).await
}
pub async fn runtime_status(
    Extension(s): Extension<HttpServices>,
) -> Result<Response, HttpServiceError> {
    execute(s, LocalControlOperation::LocalRuntimeStatus).await
}
