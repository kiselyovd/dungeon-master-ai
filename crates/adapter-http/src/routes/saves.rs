use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::services::{HttpServiceError, HttpServices, SaveMetadataCommand};

const ALLOWED_KINDS: &[&str] = &["manual", "auto", "checkpoint"];
const ALLOWED_TAGS: &[&str] = &["combat", "exploration", "dialog", "npc"];

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSaveRequest {
    pub kind: String,
    pub title: String,
    pub summary: String,
    pub tag: String,
}

#[derive(Debug, Serialize)]
pub struct CreateSaveResponse {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct RestoreQuery {
    pub session_id: String,
}

fn parse_id(raw: &str, code: &'static str) -> Result<Uuid, HttpServiceError> {
    Uuid::parse_str(raw).map_err(|_| HttpServiceError::BadRequest { code })
}

fn metadata(body: CreateSaveRequest) -> Result<SaveMetadataCommand, HttpServiceError> {
    if !ALLOWED_KINDS.contains(&body.kind.as_str()) {
        return Err(HttpServiceError::BadRequest {
            code: "save_kind_invalid",
        });
    }
    if !ALLOWED_TAGS.contains(&body.tag.as_str()) {
        return Err(HttpServiceError::BadRequest {
            code: "save_tag_invalid",
        });
    }
    Ok(SaveMetadataCommand {
        kind: body.kind,
        title: body.title,
        summary: body.summary,
        tag: body.tag,
    })
}

pub async fn list_saves(
    Extension(services): Extension<HttpServices>,
    Path(session_id): Path<String>,
) -> Result<Json<Value>, HttpServiceError> {
    services
        .saves
        .list(parse_id(&session_id, "session_id_invalid")?)
        .await
        .map(Json)
}

pub async fn create_save(
    Extension(services): Extension<HttpServices>,
    Path(session_id): Path<String>,
    Json(body): Json<CreateSaveRequest>,
) -> Result<(StatusCode, Json<CreateSaveResponse>), HttpServiceError> {
    let id = services
        .saves
        .create(
            parse_id(&session_id, "session_id_invalid")?,
            metadata(body)?,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(CreateSaveResponse { id })))
}

pub async fn quick_save(
    Extension(services): Extension<HttpServices>,
    Path(session_id): Path<String>,
) -> Result<(StatusCode, Json<CreateSaveResponse>), HttpServiceError> {
    let id = services
        .saves
        .quick(parse_id(&session_id, "session_id_invalid")?)
        .await?;
    Ok((StatusCode::CREATED, Json(CreateSaveResponse { id })))
}

pub async fn get_save(
    Extension(services): Extension<HttpServices>,
    Path(save_id): Path<String>,
) -> Result<Json<Value>, HttpServiceError> {
    services
        .saves
        .get(parse_id(&save_id, "save_id_invalid")?)
        .await
        .map(Json)
}

pub async fn delete_save(
    Extension(services): Extension<HttpServices>,
    Path(save_id): Path<String>,
) -> Result<impl IntoResponse, HttpServiceError> {
    if services
        .saves
        .delete(parse_id(&save_id, "save_id_invalid")?)
        .await?
    {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(HttpServiceError::NotFound)
    }
}

pub async fn restore_save(
    Extension(services): Extension<HttpServices>,
    Path(save_id): Path<String>,
    Query(query): Query<RestoreQuery>,
) -> Result<Json<Value>, HttpServiceError> {
    let projection = services
        .saves
        .restore(
            parse_id(&query.session_id, "session_id_invalid")?,
            parse_id(&save_id, "save_id_invalid")?,
        )
        .await?;
    Ok(Json(projection))
}

pub async fn update_save(
    Extension(services): Extension<HttpServices>,
    Path(save_id): Path<String>,
    Json(body): Json<CreateSaveRequest>,
) -> Result<impl IntoResponse, HttpServiceError> {
    if services
        .saves
        .update(parse_id(&save_id, "save_id_invalid")?, metadata(body)?)
        .await?
    {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(HttpServiceError::NotFound)
    }
}
