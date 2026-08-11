use std::convert::Infallible;

use app_application::models::agent::{AgentEvent, AgentTurnRequest};
use app_application::models::chat::{ChatMessage, MessagePart};
use axum::extract::Extension;
use axum::response::sse::{KeepAlive, Sse};
use axum::Json;
use futures::{Stream, StreamExt};
use serde::Deserialize;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

use crate::services::{AgentTurnHttpCommand, HttpServiceError, HttpServices};
use crate::sse::agent_event_to_sse;

#[derive(Debug, Deserialize)]
pub struct AgentTurnHttpRequest {
    pub campaign_id: Uuid,
    pub session_id: Uuid,
    pub player_message: String,
    pub history: Vec<ChatMessage>,
    pub model: Option<String>,
    #[serde(default)]
    pub images: Vec<MessagePart>,
    #[serde(default)]
    pub board: Option<String>,
}

pub async fn post_agent_turn(
    Extension(services): Extension<HttpServices>,
    Json(request): Json<AgentTurnHttpRequest>,
) -> Result<Sse<impl Stream<Item = Result<axum::response::sse::Event, Infallible>>>, HttpServiceError>
{
    if request.player_message.trim().is_empty() {
        return Err(HttpServiceError::BadRequest {
            code: "player_message_empty",
        });
    }
    let events = services
        .agent
        .turn(AgentTurnHttpCommand {
            model: request.model,
            request: AgentTurnRequest {
                campaign_id: request.campaign_id,
                session_id: request.session_id,
                player_message: request.player_message,
                history: request.history,
                images: request.images,
                board: request.board,
            },
        })
        .await?;
    let mut event_count = 0_u64;
    let stream = ReceiverStream::new(events).map(move |event| {
        event_count += 1;
        tracing::debug!(
            event_kind = agent_event_kind(&event),
            event_count,
            "agent SSE event"
        );
        Ok(agent_event_to_sse(event))
    });
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

fn agent_event_kind(event: &AgentEvent) -> &'static str {
    match event {
        AgentEvent::TextDelta { .. } => "text_delta",
        AgentEvent::ToolCallStart { .. } => "tool_call_start",
        AgentEvent::ToolCallResult { .. } => "tool_call_result",
        AgentEvent::ReasoningText { .. } => "reasoning_text",
        AgentEvent::ImageGenerated { .. } => "image_generated",
        AgentEvent::VideoGenerated { .. } => "video_generated",
        AgentEvent::AgentDone { .. } => "agent_done",
    }
}
