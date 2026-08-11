//! POST /agent/turn - the main agent loop SSE endpoint.
//!
//! Accepts a player action + history, runs the orchestrator in a background
//! task, and streams all AgentEvents as SSE messages keyed by their variant
//! name. The frontend (J1) deserialises these to update chat history,
//! tool-call log, journal, and NPC state.

use std::convert::Infallible;
use std::pin::Pin;
use std::sync::Arc;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::Json;
use futures::stream::{Stream, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

use app_llm::{ChatMessage, MessagePart};

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
    /// Image attachments staged for this turn (vision). Empty/omitted for
    /// text-only turns. [M11 F2]
    #[serde(default)]
    pub images: Vec<MessagePart>,
    /// Pre-formatted live VTT board snapshot (scene + initiative + token
    /// HP/AC/position/conditions) so the DM narrates from the real board.
    /// Omitted outside combat.
    #[serde(default)]
    pub board: Option<String>,
}

#[tracing::instrument(skip_all, fields(session_id = %req.session_id, campaign_id = %req.campaign_id))]
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
    let image_provider = state.image_provider();
    let video_provider = state.video_provider();
    let gpu_swap = build_gpu_swap(&state).await;
    let pool = state.db().clone();

    let turn_req = AgentTurnRequest {
        campaign_id: req.campaign_id,
        session_id: req.session_id,
        player_message: req.player_message,
        history: req.history,
        images: req.images,
        board: req.board,
    };

    let (tx, rx) = mpsc::channel::<AgentEvent>(64);

    let pool_for_orch = pool.clone();
    tokio::spawn(async move {
        let orch =
            AgentOrchestrator::new(provider, pool_for_orch, config, retriever, image_provider)
                .with_gpu_swap(gpu_swap)
                .with_video_provider(video_provider);
        if let Err(e) = orch.run(turn_req, tx).await {
            tracing::warn!(error = %e, "agent loop error");
        }
    });

    let event_stream: Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>> =
        Box::pin(ReceiverStream::new(rx).map(|agent_event| Ok(agent_event_to_sse(agent_event))));

    Ok(Sse::new(event_stream).keep_alive(KeepAlive::default()))
}

/// Build the Auto-Swap coordinator when a local LLM runtime is up under the
/// Auto-Swap VRAM strategy. Returns `None` for a cloud LLM, a stopped local
/// runtime, or any non-Auto-Swap strategy, in which case image generation runs
/// without touching the LLM.
async fn build_gpu_swap(
    state: &AppState,
) -> Option<Arc<crate::local_runtime::registry::ImageGpuSwap>> {
    use crate::routes::local_mode::VramStrategy;
    if state.local_mode_config().vram_strategy != VramStrategy::AutoSwap {
        return None;
    }
    let port = match state.runtime_status().await.llm {
        crate::local_runtime::runtime::RuntimeStatus::Ready { port } => port,
        _ => return None,
    };
    let args = crate::routes::local_mode::build_llm_spawn_args(state, port).ok()?;
    Some(Arc::new(crate::local_runtime::registry::ImageGpuSwap::new(
        state.runtime_registry(),
        args,
        port,
    )))
}

/// Convert an application event into its stable SSE event name and JSON body.
/// Keeping this mapping pure makes the cross-process contract executable.
pub fn agent_event_to_wire(ev: AgentEvent) -> (&'static str, serde_json::Value) {
    match ev {
        AgentEvent::ReasoningText { text } => {
            ("reasoning_text", serde_json::json!({ "text": text }))
        }
        AgentEvent::ImageGenerated {
            tool_call_id,
            round,
            mime_type,
            image_b64,
            kind,
        } => (
            "image_generated",
            serde_json::json!({
                "tool_call_id": tool_call_id,
                "round": round,
                "mime_type": mime_type,
                "image_b64": image_b64,
                "kind": kind,
            }),
        ),
        AgentEvent::TextDelta { text } => ("text_delta", serde_json::json!({ "text": text })),
        AgentEvent::ToolCallStart {
            id,
            tool_name,
            round,
        } => (
            "tool_call_start",
            serde_json::json!({
                "id": id,
                "tool_name": tool_name,
                "round": round,
            }),
        ),
        AgentEvent::ToolCallResult {
            id,
            tool_name,
            args,
            result,
            is_error,
            round,
            handled_by,
        } => (
            "tool_call_result",
            serde_json::json!({
                "id": id,
                "tool_name": tool_name,
                "args": args,
                "result": result,
                "is_error": is_error,
                "round": round,
                "handled_by": handled_by,
            }),
        ),
        AgentEvent::VideoGenerated {
            tool_call_id,
            round,
            mime_type,
            video_b64,
            kind,
        } => (
            "video_generated",
            serde_json::json!({
                "tool_call_id": tool_call_id,
                "round": round,
                "mime_type": mime_type,
                "video_b64": video_b64,
                "kind": kind,
            }),
        ),
        AgentEvent::AgentDone { total_rounds } => (
            "agent_done",
            serde_json::json!({ "total_rounds": total_rounds }),
        ),
    }
}

fn agent_event_to_sse(ev: AgentEvent) -> Event {
    let (event_name, payload) = agent_event_to_wire(ev);
    Event::default()
        .event(event_name)
        .json_data(payload)
        .expect("agent event json")
}
