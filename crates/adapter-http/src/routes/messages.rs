use axum::extract::{Extension, Path};
use axum::Json;
use serde_json::Value;

use crate::services::{HttpServiceError, HttpServices};

pub async fn list_messages(
    Extension(services): Extension<HttpServices>,
    Path(session_id): Path<String>,
) -> Result<Json<Value>, HttpServiceError> {
    services.campaign.messages(session_id).await.map(Json)
}
