use app_application::models::agent::{AgentEvent, AgentTurnRequest};
use app_application::models::chat::{ChatMessage, MessagePart};
use axum::extract::{Extension, Query};
use axum::http::{header, HeaderValue};
use axum::response::sse::{KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::Json;
use futures::StreamExt;
use serde::Deserialize;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

use crate::services::{AgentTurnHttpCommand, HttpServiceError, HttpServices};
use crate::sse::agent_event_to_sse;
#[derive(Deserialize)]
pub struct AgentPendingQuery {
    campaign_id: Uuid,
    session_id: Uuid,
}
pub async fn get_pending_clarification(
    Extension(services): Extension<HttpServices>,
    Query(input): Query<AgentPendingQuery>,
) -> Result<
    Json<Option<app_application::agent::human_commands::PendingHumanCommandClarification>>,
    HttpServiceError,
> {
    if input.campaign_id.is_nil() || input.session_id.is_nil() {
        return Err(HttpServiceError::BadRequest {
            code: "agent_request_invalid",
        });
    }
    Ok(Json(
        services
            .agent
            .pending_clarification(input.campaign_id, input.session_id)
            .await?,
    ))
}

#[derive(Debug, Deserialize)]
pub struct AgentTurnHttpRequest {
    #[serde(default)]
    pub clarification:
        Option<app_application::agent::human_commands::HumanCommandClarificationReply>,
    #[serde(default)]
    pub resume: bool,
    #[serde(default)]
    pub request_id: Option<Uuid>,
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
) -> Result<impl IntoResponse, HttpServiceError> {
    if request.player_message.trim().is_empty() {
        return Err(HttpServiceError::BadRequest {
            code: "player_message_empty",
        });
    }
    if request.request_id.is_some_and(|id| id.is_nil()) {
        return Err(HttpServiceError::BadRequest {
            code: "agent_request_invalid",
        });
    }
    if request
        .clarification
        .as_ref()
        .is_some_and(|reply| !reply.valid() || !request.resume || request.request_id.is_none())
    {
        return Err(HttpServiceError::BadRequest {
            code: "human_command_clarification_invalid",
        });
    }
    let events = services
        .agent
        .turn(AgentTurnHttpCommand {
            clarification: request.clarification,
            resume: request.resume,
            request_id: request.request_id,
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
        Ok::<_, std::convert::Infallible>(agent_event_to_sse(event))
    });
    let mut response = Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response();
    response
        .headers_mut()
        .insert(header::CONNECTION, HeaderValue::from_static("close"));
    Ok(response)
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
        AgentEvent::AgentNeedsInput { .. } => "agent_needs_input",
    }
}
