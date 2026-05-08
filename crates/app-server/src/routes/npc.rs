use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;
use uuid::Uuid;

use crate::db::{NpcMemoryRow, npc_get_all};
use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CampaignQuery {
    pub campaign_id: Uuid,
}

pub async fn get_npcs(
    State(state): State<AppState>,
    Query(q): Query<CampaignQuery>,
) -> Result<Json<Vec<NpcMemoryRow>>, AppError> {
    let npcs = npc_get_all(state.db(), q.campaign_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(npcs))
}
