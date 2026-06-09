//! Saves API (M5 P2.13)
//!
//! Five endpoints powering the "Chronicles of Adventure" tome modal:
//! - GET    /sessions/{session_id}/saves         list saves for a session
//! - POST   /sessions/{session_id}/saves         create a manual / checkpoint save
//! - POST   /sessions/{session_id}/saves/quick   one-shot quick save (Ctrl+S)
//! - GET    /saves/{save_id}                     load full save (with game_state)
//! - DELETE /saves/{save_id}                     delete a save
//!
//! v1 is linear - branches deferred to v2. Validation rejects unknown
//! `kind` / `tag` values up front so the UI never has to handle stray
//! values pulled out of the DB.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{
    restore_snapshot, save_delete, save_insert, save_list_by_session, save_load, save_update,
    SaveRow, SaveSummary,
};
use crate::error::AppError;
use crate::state::AppState;

const ALLOWED_KINDS: &[&str] = &["manual", "auto", "checkpoint"];
const ALLOWED_TAGS: &[&str] = &["combat", "exploration", "dialog", "npc"];

#[derive(Debug, Deserialize)]
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

fn parse_session_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|e| AppError::BadRequest(format!("invalid session_id: {e}")))
}

fn parse_save_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|e| AppError::BadRequest(format!("invalid save_id: {e}")))
}

fn validate_kind(kind: &str) -> Result<(), AppError> {
    if ALLOWED_KINDS.contains(&kind) {
        Ok(())
    } else {
        Err(AppError::BadRequest(format!("invalid kind: {kind}")))
    }
}

fn validate_tag(tag: &str) -> Result<(), AppError> {
    if ALLOWED_TAGS.contains(&tag) {
        Ok(())
    } else {
        Err(AppError::BadRequest(format!("invalid tag: {tag}")))
    }
}

fn envelope_for(title: &str, summary: &str, tag: &str, kind: &str) -> serde_json::Value {
    // Linear-save v1 envelope: front-end metadata only. Chat / combat
    // rehydration goes through the existing snapshot path; the user-driven
    // save is purely a UI bookmark for now.
    serde_json::json!({
        "schema_version": 1,
        "state": {
            "title": title,
            "summary": summary,
            "tag": tag,
            "kind": kind,
        }
    })
}

pub async fn list_saves(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<Vec<SaveSummary>>, AppError> {
    let sid = parse_session_id(&session_id)?;
    let saves = save_list_by_session(state.db(), sid).await?;
    Ok(Json(saves))
}

pub async fn create_save(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(body): Json<CreateSaveRequest>,
) -> Result<(StatusCode, Json<CreateSaveResponse>), AppError> {
    let sid = parse_session_id(&session_id)?;
    validate_kind(&body.kind)?;
    validate_tag(&body.tag)?;
    let envelope = envelope_for(&body.title, &body.summary, &body.tag, &body.kind);
    let id = save_insert(
        state.db(),
        sid,
        &body.kind,
        &body.title,
        &body.summary,
        &body.tag,
        &envelope,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(CreateSaveResponse { id })))
}

pub async fn quick_save(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<(StatusCode, Json<CreateSaveResponse>), AppError> {
    let sid = parse_session_id(&session_id)?;
    let title = "Quick save".to_string();
    let summary = "(no scene)".to_string();
    let tag = "exploration";
    let kind = "auto";
    let envelope = envelope_for(&title, &summary, tag, kind);
    let id = save_insert(state.db(), sid, kind, &title, &summary, tag, &envelope).await?;
    Ok((StatusCode::CREATED, Json(CreateSaveResponse { id })))
}

pub async fn get_save(
    State(state): State<AppState>,
    Path(save_id): Path<String>,
) -> Result<Json<SaveRow>, AppError> {
    let id = parse_save_id(&save_id)?;
    match save_load(state.db(), id).await? {
        Some(row) => Ok(Json(row)),
        None => Err(AppError::NotFound),
    }
}

pub async fn delete_save(
    State(state): State<AppState>,
    Path(save_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let id = parse_save_id(&save_id)?;
    let removed = save_delete(state.db(), id).await?;
    if removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

/// Restore a save's combat + scene state into the DB and return the full
/// game_state for the frontend to rehydrate from.
/// POST /saves/{save_id}/restore?session_id={session_id}
///
/// W2.3: This is the load-side counterpart to quick_save. It:
///   1. Ends the current open combat encounter for `session_id`.
///   2. Re-inserts the saved encounter + tokens.
///   3. Returns the full `GameStateV2` so the frontend can rehydrate
///      the combat and scene Zustand slices.
#[derive(Debug, Deserialize)]
pub struct RestoreQuery {
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct RestoreResponse {
    pub game_state: serde_json::Value,
}

pub async fn restore_save(
    State(state): State<AppState>,
    Path(save_id): Path<String>,
    axum::extract::Query(q): axum::extract::Query<RestoreQuery>,
) -> Result<Json<RestoreResponse>, AppError> {
    let id = parse_save_id(&save_id)?;
    let session_id = Uuid::parse_str(&q.session_id)
        .map_err(|e| AppError::BadRequest(format!("invalid session_id: {e}")))?;
    match restore_snapshot(state.db(), session_id, id).await? {
        Some(gs) => {
            let game_state = serde_json::to_value(&gs)
                .map_err(|e| AppError::Internal(format!("serialize game_state: {e}")))?;
            Ok(Json(RestoreResponse { game_state }))
        }
        None => Err(AppError::NotFound),
    }
}

/// Overwrite an existing save's metadata in place (PUT /saves/{save_id}). The
/// "Overwrite" UI action used to POST a create, duplicating the row; this
/// updates by id instead. Reuses the create request shape. [M11 F3]
pub async fn update_save(
    State(state): State<AppState>,
    Path(save_id): Path<String>,
    Json(body): Json<CreateSaveRequest>,
) -> Result<impl IntoResponse, AppError> {
    let id = parse_save_id(&save_id)?;
    validate_kind(&body.kind)?;
    validate_tag(&body.tag)?;
    let envelope = envelope_for(&body.title, &body.summary, &body.tag, &body.kind);
    let updated = save_update(
        state.db(),
        id,
        &body.kind,
        &body.title,
        &body.summary,
        &body.tag,
        &envelope,
    )
    .await?;
    if updated {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}
