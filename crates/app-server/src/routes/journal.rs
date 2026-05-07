use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;
use uuid::Uuid;

use crate::db::{JournalEntry, journal_list};
use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CampaignQuery {
    pub campaign_id: Uuid,
}

pub async fn get_journal(
    State(state): State<AppState>,
    Query(q): Query<CampaignQuery>,
) -> Result<Json<Vec<JournalEntry>>, AppError> {
    let entries = journal_list(state.db(), q.campaign_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(entries))
}
