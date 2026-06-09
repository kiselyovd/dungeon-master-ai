//! Direct image-generation route used by the character wizard's Portrait tab.
//!
//! Unlike the agent's `generate_image` tool (which peels the blob off into a
//! dedicated `ImageGenerated` SSE event), this is a plain request/response
//! endpoint: the frontend POSTs a prompt and gets back a ready-to-use data URL
//! it can drop straight into an `<img src>`. Returns 404 when no image provider
//! is wired (Image tab disabled / not configured in Settings), mirroring the
//! `/video/generate` contract.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::image::provider::ImagePrompt;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ImageGenerateRequest {
    pub prompt: String,
    #[serde(default)]
    pub style_preset: Option<String>,
    #[serde(default)]
    pub scene_id: Option<String>,
}

#[derive(Serialize)]
pub struct ImageGenerateResponse {
    /// `data:<mime>;base64,<...>` - directly usable as an `<img src>`.
    pub url: String,
    pub mime_type: String,
}

pub async fn post_image_generate(
    State(state): State<AppState>,
    Json(req): Json<ImageGenerateRequest>,
) -> Result<Json<ImageGenerateResponse>, StatusCode> {
    let provider = state.image_provider().ok_or(StatusCode::NOT_FOUND)?;

    let prompt = req.prompt.trim().to_string();
    if prompt.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let image_prompt = ImagePrompt {
        content_prompt: prompt,
        style_preset: req.style_preset.unwrap_or_else(|| "portrait".to_string()),
        scene_id: req.scene_id,
        npc_ids: Vec::new(),
        backend_preset: None,
        width: None,
        height: None,
    };

    let bytes = provider
        .generate(image_prompt)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let url = format!(
        "data:{};base64,{}",
        bytes.mime_type,
        B64.encode(&bytes.data)
    );
    Ok(Json(ImageGenerateResponse {
        url,
        mime_type: bytes.mime_type,
    }))
}
