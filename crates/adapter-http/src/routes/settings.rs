pub use app_application::models::settings::SettingsConfigV2;
use axum::extract::Extension;
use axum::Json;
use serde_json::Value;

use crate::services::{HttpServiceError, HttpServices};

pub async fn get_providers(
    Extension(services): Extension<HttpServices>,
) -> Result<Json<Value>, HttpServiceError> {
    services.settings.provider_info().await.map(Json)
}

pub async fn post_settings_v2(
    Extension(services): Extension<HttpServices>,
    Json(config): Json<SettingsConfigV2>,
) -> Result<Json<Value>, HttpServiceError> {
    services.settings.update(config).await.map(Json)
}
