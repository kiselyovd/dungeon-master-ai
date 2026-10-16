use axum::extract::{Extension, Path};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::services::{HttpServiceError, HttpServices, RestoreSaveCommand, SaveMetadataCommand};

const ALLOWED_KINDS: &[&str] = &["manual", "auto", "checkpoint"];
const ALLOWED_TAGS: &[&str] = &["combat", "exploration", "dialog", "npc"];

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSaveRequest {
    pub campaign_id: Uuid,
    pub expected_revision: u64,
    pub causation_id: String,
    pub character: Value,
    pub kind: String,
    pub title: String,
    pub summary: String,
    pub tag: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSaveRequest {
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
pub struct RestoreSaveRequest {
    pub campaign_id: Uuid,
    pub session_id: Uuid,
    pub expected_revision: u64,
    pub causation_id: String,
}

fn parse_id(raw: &str, code: &'static str) -> Result<Uuid, HttpServiceError> {
    Uuid::parse_str(raw).map_err(|_| HttpServiceError::BadRequest { code })
}

fn validate_metadata(kind: &str, tag: &str) -> Result<(), HttpServiceError> {
    if !ALLOWED_KINDS.contains(&kind) {
        return Err(HttpServiceError::BadRequest {
            code: "save_kind_invalid",
        });
    }
    if !ALLOWED_TAGS.contains(&tag) {
        return Err(HttpServiceError::BadRequest {
            code: "save_tag_invalid",
        });
    }
    Ok(())
}

fn metadata(body: CreateSaveRequest) -> Result<SaveMetadataCommand, HttpServiceError> {
    validate_metadata(&body.kind, &body.tag)?;
    Ok(SaveMetadataCommand {
        campaign_id: body.campaign_id,
        expected_revision: body.expected_revision,
        causation_id: body.causation_id,
        character: body.character,
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
    Json(mut body): Json<CreateSaveRequest>,
) -> Result<(StatusCode, Json<CreateSaveResponse>), HttpServiceError> {
    body.kind = "auto".into();
    let id = services
        .saves
        .create(
            parse_id(&session_id, "session_id_invalid")?,
            metadata(body)?,
        )
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
    Json(body): Json<RestoreSaveRequest>,
) -> Result<Json<Value>, HttpServiceError> {
    let projection = services
        .saves
        .restore(RestoreSaveCommand {
            save_id: parse_id(&save_id, "save_id_invalid")?,
            campaign_id: body.campaign_id,
            session_id: body.session_id,
            expected_revision: body.expected_revision,
            causation_id: body.causation_id,
        })
        .await?;
    Ok(Json(projection))
}

pub async fn update_save(
    Extension(services): Extension<HttpServices>,
    Path(save_id): Path<String>,
    Json(body): Json<UpdateSaveRequest>,
) -> Result<impl IntoResponse, HttpServiceError> {
    validate_metadata(&body.kind, &body.tag)?;
    if services
        .saves
        .update(
            parse_id(&save_id, "save_id_invalid")?,
            SaveMetadataCommand {
                campaign_id: Uuid::nil(),
                expected_revision: 0,
                causation_id: String::new(),
                character: Value::Null,
                kind: body.kind,
                title: body.title,
                summary: body.summary,
                tag: body.tag,
            },
        )
        .await?
    {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(HttpServiceError::NotFound)
    }
}
