use axum::extract::{Query, State};
use axum::Json;

use super::CampaignQuery;
use crate::db::{npc_get_all, NpcMemoryRow};
use crate::error::AppError;
use crate::state::AppState;

pub async fn get_npcs(
    State(state): State<AppState>,
    Query(q): Query<CampaignQuery>,
) -> Result<Json<Vec<NpcMemoryRow>>, AppError> {
    let npcs = npc_get_all(state.db(), q.campaign_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(npcs))
}
