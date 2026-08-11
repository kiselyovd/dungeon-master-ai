use std::convert::Infallible;

use app_application::ports::media::VideoPrompt;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::Json;
use futures::{Stream, StreamExt};
use tokio_stream::wrappers::ReceiverStream;

use crate::services::{HttpServiceError, HttpServices};

pub async fn post_video_generate(
    Extension(services): Extension<HttpServices>,
    Json(prompt): Json<VideoPrompt>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let stream = services
        .media
        .generate_video(prompt)
        .await
        .map_err(status_for_service_error)?;
    let sse = ReceiverStream::new(stream.events).map(|event| {
        let json = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
        Ok::<_, Infallible>(Event::default().data(json))
    });
    Ok(Sse::new(sse))
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
