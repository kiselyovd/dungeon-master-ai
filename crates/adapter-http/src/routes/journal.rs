use axum::extract::{Extension, Query};
use axum::Json;
use serde_json::Value;

use super::CampaignQuery;
use crate::services::{HttpServiceError, HttpServices};

pub async fn get_journal(
    Extension(services): Extension<HttpServices>,
    Query(query): Query<CampaignQuery>,
) -> Result<Json<Value>, HttpServiceError> {
    services.campaign.journal(query.campaign_id).await.map(Json)
}
