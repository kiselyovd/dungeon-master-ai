use app_application::ports::media::{ImagePrompt, ImageSource};
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::Json;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::services::{HttpServiceError, HttpServices};

#[derive(Debug, Deserialize)]
pub struct ImageGenerateRequest {
    pub prompt: String,
    #[serde(default)]
    pub style_preset: Option<String>,
    #[serde(default)]
    pub scene_id: Option<String>,
    #[serde(default)]
    pub asset_scope: Option<super::media_assets::LocalMediaScope>,
}

#[derive(Debug, Serialize)]
pub struct ImageGenerateResponse {
    pub url: String,
    pub mime_type: String,
    pub source: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_ref: Option<app_application::models::media_asset::MediaReference>,
}

pub async fn post_image_generate(
    Extension(services): Extension<HttpServices>,
    Json(request): Json<ImageGenerateRequest>,
) -> Result<Json<ImageGenerateResponse>, StatusCode> {
    let content_prompt = request.prompt.trim().to_string();
    if content_prompt.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let style_preset = request
        .style_preset
        .unwrap_or_else(|| "portrait".to_string());
    let category = match style_preset.as_str() {
        "portrait" | "character_portrait" => {
            app_application::models::media_asset::MediaCategory::Portrait
        }
        "map" | "battle_map" => app_application::models::media_asset::MediaCategory::Map,
        _ => app_application::models::media_asset::MediaCategory::Illustration,
    };
    let owner = request
        .asset_scope
        .as_ref()
        .map(super::media_assets::LocalMediaScope::owner)
        .transpose()
        .map_err(status_for_service_error)?;
    if category == app_application::models::media_asset::MediaCategory::Portrait
        && matches!(
            owner,
            Some(app_application::models::media_asset::MediaOwner::Campaign { actor_id: None, .. })
        )
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let generated = services
        .media
        .generate_image(ImagePrompt {
            content_prompt,
            style_preset,
            scene_id: request.scene_id,
            npc_ids: Vec::new(),
            backend_preset: None,
            width: None,
            height: None,
        })
        .await
        .map_err(status_for_service_error)?;
    let media_ref = if matches!(generated.source, ImageSource::Generated) {
        if let Some(owner) = owner {
            Some(
                services
                    .media
                    .register_asset(
                        owner,
                        category,
                        app_application::models::media_asset::MediaOrigin::Generated,
                        &generated.mime_type,
                        generated.data.clone(),
                    )
                    .await
                    .map_err(status_for_service_error)?,
            )
        } else {
            None
        }
    } else {
        None
    };
    let url = if media_ref.is_some() {
        String::new()
    } else {
        format!(
            "data:{};base64,{}",
            generated.mime_type,
            B64.encode(&generated.data)
        )
    };
    let (source, asset_id) = match generated.source {
        ImageSource::Generated => ("generated", None),
        ImageSource::Bundled { asset_id } => ("bundled", Some(asset_id)),
    };
    Ok(Json(ImageGenerateResponse {
        url,
        mime_type: generated.mime_type,
        source,
        asset_id,
        media_ref,
    }))
}

fn status_for_service_error(error: HttpServiceError) -> StatusCode {
    match error {
        HttpServiceError::NotFound => StatusCode::NOT_FOUND,
        HttpServiceError::BadRequest { .. } => StatusCode::BAD_REQUEST,
        HttpServiceError::Conflict { .. } => StatusCode::CONFLICT,
        HttpServiceError::UnprocessableEntity { .. } => StatusCode::UNPROCESSABLE_ENTITY,
        HttpServiceError::PayloadTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
        HttpServiceError::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
        HttpServiceError::RateLimit { .. } => StatusCode::TOO_MANY_REQUESTS,
        HttpServiceError::BadGateway { .. } => StatusCode::BAD_GATEWAY,
        HttpServiceError::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
