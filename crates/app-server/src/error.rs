use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("llm provider error: {0}")]
    Llm(#[from] app_llm::LlmError),
    #[error("not found")]
    NotFound,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("internal: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::Llm(app_llm::LlmError::AuthFailure) => {
                (StatusCode::UNAUTHORIZED, "auth_failed", self.to_string())
            }
            AppError::Llm(app_llm::LlmError::RateLimit) => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limit",
                self.to_string(),
            ),
            AppError::Llm(_) => (StatusCode::BAD_GATEWAY, "provider_error", self.to_string()),
            AppError::NotFound => (StatusCode::NOT_FOUND, "not_found", self.to_string()),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request", self.to_string()),
            AppError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                self.to_string(),
            ),
        };

        let body = Json(json!({
            "error": {
                "code": code,
                "message": message,
            }
        }));

        (status, body).into_response()
    }
}
