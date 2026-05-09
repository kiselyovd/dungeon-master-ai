//! GET /sessions/{session_id}/messages
//!
//! Returns the persisted chat history for a given session in chronological
//! order. Used by the frontend on session open to rehydrate Zustand from
//! the canonical SQLite source instead of localStorage.

use app_llm::ChatMessage;
use axum::Json;
use axum::extract::{Path, State};
use serde::Serialize;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Serialize)]
pub struct MessagesResponse {
    pub messages: Vec<ChatMessage>,
}

pub async fn list_messages(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<MessagesResponse>, AppError> {
    let messages = crate::db::list_messages_by_session(state.db(), &session_id).await?;
    Ok(Json(MessagesResponse { messages }))
}
