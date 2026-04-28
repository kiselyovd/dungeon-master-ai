use axum::extract::State;
use axum::response::IntoResponse;

use crate::error::AppError;
use crate::state::AppState;

pub async fn chat(State(_state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    Err::<&'static str, _>(AppError::NotFound)
}
