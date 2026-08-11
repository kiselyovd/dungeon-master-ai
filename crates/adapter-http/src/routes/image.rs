use app_application::ports::media::ImagePrompt;
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
}

#[derive(Debug, Serialize)]
pub struct ImageGenerateResponse {
    pub url: String,
    pub mime_type: String,
}

pub async fn post_image_generate(
    Extension(services): Extension<HttpServices>,
    Json(request): Json<ImageGenerateRequest>,
) -> Result<Json<ImageGenerateResponse>, StatusCode> {
    let content_prompt = request.prompt.trim().to_string();
    if content_prompt.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let generated = services
        .media
        .generate_image(ImagePrompt {
            content_prompt,
            style_preset: request
                .style_preset
                .unwrap_or_else(|| "portrait".to_string()),
            scene_id: request.scene_id,
            npc_ids: Vec::new(),
            backend_preset: None,
            width: None,
            height: None,
        })
        .await
        .map_err(status_for_service_error)?;
    let url = format!(
        "data:{};base64,{}",
        generated.mime_type,
        B64.encode(&generated.data)
    );
    Ok(Json(ImageGenerateResponse {
        url,
        mime_type: generated.mime_type,
    }))
}

fn status_for_service_error(error: HttpServiceError) -> StatusCode {
    match error {
        HttpServiceError::NotFound => StatusCode::NOT_FOUND,
        HttpServiceError::BadRequest { .. } => StatusCode::BAD_REQUEST,
        HttpServiceError::PayloadTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
        HttpServiceError::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
        HttpServiceError::RateLimit { .. } => StatusCode::TOO_MANY_REQUESTS,
        HttpServiceError::BadGateway { .. } => StatusCode::BAD_GATEWAY,
        HttpServiceError::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
