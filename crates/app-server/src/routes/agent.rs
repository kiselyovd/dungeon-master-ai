//! POST /agent/turn - the main agent loop SSE endpoint.
//!
//! Accepts a player action + history, runs the orchestrator in a background
//! task, and streams all AgentEvents as SSE messages keyed by their variant
//! name. The frontend (J1) deserialises these to update chat history,
//! tool-call log, journal, and NPC state.

use std::convert::Infallible;
use std::pin::Pin;

use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::{Stream, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

use app_llm::ChatMessage;

use crate::agent::orchestrator::{AgentEvent, AgentOrchestrator, AgentTurnRequest};
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct AgentTurnHttpRequest {
    pub campaign_id: Uuid,
    pub session_id: Uuid,
    pub player_message: String,
    pub history: Vec<ChatMessage>,
    /// Override model for this request (optional; falls back to AppState's AgentConfig).
    pub model: Option<String>,
}

pub async fn post_agent_turn(
    State(state): State<AppState>,
    Json(req): Json<AgentTurnHttpRequest>,
) -> Result<impl IntoResponse, AppError> {
    if req.player_message.trim().is_empty() {
        return Err(AppError::BadRequest(
            "player_message must not be empty".into(),
        ));
    }

    let provider = state.provider();
    let mut config = state.agent_config();
    if let Some(model) = req.model {
        config.model = model;
    }
    let retriever = state.srd_retriever();
    let pool = state.db().clone();

    let turn_req = AgentTurnRequest {
        campaign_id: req.campaign_id,
        session_id: req.session_id,
        player_message: req.player_message,
        history: req.history,
    };

    let (tx, rx) = mpsc::channel::<AgentEvent>(64);

    tokio::spawn(async move {
        let orch = AgentOrchestrator::new(provider, pool, config, retriever);
        if let Err(e) = orch.run(turn_req, tx).await {
            tracing::warn!(error = %e, "agent loop error");
        }
    });

    let event_stream: Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>> =
        Box::pin(ReceiverStream::new(rx).map(|agent_event| Ok(agent_event_to_sse(agent_event))));

    Ok(Sse::new(event_stream).keep_alive(KeepAlive::default()))
}

fn agent_event_to_sse(ev: AgentEvent) -> Event {
    match ev {
        AgentEvent::TextDelta { text } => Event::default()
            .event("text_delta")
            .json_data(serde_json::json!({ "text": text }))
            .expect("text_delta json"),
        AgentEvent::ToolCallStart {
            id,
            tool_name,
            round,
        } => Event::default()
            .event("tool_call_start")
            .json_data(serde_json::json!({
                "id": id,
                "tool_name": tool_name,
                "round": round,
            }))
            .expect("tool_call_start json"),
        AgentEvent::ToolCallResult {
            id,
            tool_name,
            args,
            result,
            is_error,
            round,
        } => Event::default()
            .event("tool_call_result")
            .json_data(serde_json::json!({
                "id": id,
                "tool_name": tool_name,
                "args": args,
                "result": result,
                "is_error": is_error,
                "round": round,
            }))
            .expect("tool_call_result json"),
        AgentEvent::AgentDone { total_rounds } => Event::default()
            .event("agent_done")
            .json_data(serde_json::json!({ "total_rounds": total_rounds }))
            .expect("agent_done json"),
    }
}
