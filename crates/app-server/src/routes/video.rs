//! M7-DM video generation route. SSE stream of VideoEvent.
//!
//! Reads the configured VideoProvider from AppState and streams its events
//! to the client. 404 when no provider is wired (Video tab disabled in Settings).

use std::convert::Infallible;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::Json;
use futures::Stream;
use futures::StreamExt;
use tokio_stream::wrappers::ReceiverStream;

use crate::state::AppState;
use crate::video::{VideoPrompt, VideoProvider};

pub async fn post_video_generate(
    State(state): State<AppState>,
    Json(prompt): Json<VideoPrompt>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let provider: std::sync::Arc<dyn VideoProvider> =
        state.video_provider().ok_or(StatusCode::NOT_FOUND)?;
    let stream = provider
        .generate(prompt)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let sse = ReceiverStream::new(stream.events).map(|evt| {
        let json = serde_json::to_string(&evt).unwrap_or_else(|_| "{}".to_string());
        Ok::<_, Infallible>(Event::default().data(json))
    });
    Ok(Sse::new(sse))
}
