//! POST /agent/turn - the main agent loop SSE endpoint.
//!
//! Accepts a player action + history, runs the orchestrator in a background
//! task, and streams all AgentEvents as SSE messages keyed by their variant
//! name. The frontend (J1) deserialises these to update chat history,
//! tool-call log, journal, and NPC state.

use std::convert::Infallible;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::Json;
use futures::stream::{Stream, StreamExt};
use serde::Deserialize;
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

use app_llm::{ChatMessage, MessagePart, ToolCall, ToolResult};

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
    let gpu_swap = build_gpu_swap(&state).await;
    let pool = state.db().clone();

    // Persist user message before the orchestrator runs. Best-effort.
    let session_id_str = req.session_id.to_string();
    // Persist the user turn with any image parts so a reloaded session shows
    // the same multimodal message the LLM saw (F2).
    let user_msg = if req.images.is_empty() {
        ChatMessage::user_text(req.player_message.clone())
    } else {
        let mut parts = vec![MessagePart::Text {
            text: req.player_message.clone(),
        }];
        parts.extend(req.images.iter().cloned());
        ChatMessage::User { parts }
    };
    if let Err(e) = crate::db::insert_message(&pool, &session_id_str, &user_msg).await {
        tracing::warn!(err = %e, "failed to persist user message");
    }

    let turn_req = AgentTurnRequest {
        campaign_id: req.campaign_id,
        session_id: req.session_id,
        player_message: req.player_message,
        history: req.history,
        images: req.images,
    };

    let (tx, rx) = mpsc::channel::<AgentEvent>(64);

    let pool_for_orch = pool.clone();
    tokio::spawn(async move {
        let orch =
            AgentOrchestrator::new(provider, pool_for_orch, config, retriever, image_provider)
                .with_gpu_swap(gpu_swap);
        if let Err(e) = orch.run(turn_req, tx).await {
            tracing::warn!(error = %e, "agent loop error");
        }
    });

    let persist_state = Arc::new(Mutex::new(PersistState::default()));
    let pool_for_persist = pool.clone();
    let session_for_persist = session_id_str.clone();

    let event_stream: Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>> =
        Box::pin(ReceiverStream::new(rx).map(move |agent_event| {
            persist_event(
                &agent_event,
                &persist_state,
                &pool_for_persist,
                &session_for_persist,
            );
            Ok(agent_event_to_sse(agent_event))
        }));

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

/// Per-round buffer used to assemble assistant + tool_call rows out of the
/// stream of `AgentEvent`s emitted by the orchestrator.
#[derive(Default)]
struct PersistState {
    current_round: usize,
    text_buf: String,
    tool_calls: Vec<ToolCall>,
    tool_results: Vec<ToolResult>,
}

fn persist_event(
    ev: &AgentEvent,
    state: &Arc<Mutex<PersistState>>,
    pool: &SqlitePool,
    session_id: &str,
) {
    match ev {
        AgentEvent::ReasoningText { .. } => {
            // Thinking/reasoning text is transient UI-only; not persisted to history.
        }
        AgentEvent::ImageGenerated { .. } => {
            // Transient: the generated image is streamed to the UI, not
            // persisted to chat history.
        }
        AgentEvent::TextDelta { text } => {
            if let Ok(mut s) = state.lock() {
                s.text_buf.push_str(text);
            }
        }
        AgentEvent::ToolCallStart { round, .. } => {
            flush_round_if_changed(*round, state, pool, session_id);
        }
        AgentEvent::ToolCallResult {
            id,
            tool_name,
            args,
            result,
            is_error,
            round,
            handled_by: _,
        } => {
            flush_round_if_changed(*round, state, pool, session_id);
            if let Ok(mut s) = state.lock() {
                s.tool_calls.push(ToolCall {
                    id: id.clone(),
                    name: tool_name.clone(),
                    args: args.clone(),
                });
                s.tool_results.push(ToolResult {
                    tool_call_id: id.clone(),
                    content: serde_json::to_string(result).unwrap_or_default(),
                    is_error: *is_error,
                });
            }
        }
        AgentEvent::AgentDone { .. } => {
            flush_pending(state, pool, session_id);
        }
    }
}

fn flush_round_if_changed(
    new_round: usize,
    state: &Arc<Mutex<PersistState>>,
    pool: &SqlitePool,
    session_id: &str,
) {
    let needs_flush = match state.lock() {
        Ok(s) => s.current_round != new_round && (s.current_round != 0 || !s.text_buf.is_empty()),
        Err(_) => false,
    };
    if needs_flush {
        flush_pending(state, pool, session_id);
    }
    if let Ok(mut s) = state.lock() {
        s.current_round = new_round;
    }
}

fn flush_pending(state: &Arc<Mutex<PersistState>>, pool: &SqlitePool, session_id: &str) {
    let drained = match state.lock() {
        Ok(mut s) => {
            let text = std::mem::take(&mut s.text_buf);
            let tool_calls = std::mem::take(&mut s.tool_calls);
            let tool_results = std::mem::take(&mut s.tool_results);
            Some((text, tool_calls, tool_results))
        }
        Err(_) => None,
    };
    let Some((text, tool_calls, tool_results)) = drained else {
        return;
    };
    if text.is_empty() && tool_calls.is_empty() {
        return;
    }
    let pool = pool.clone();
    let session_id = session_id.to_string();
    tokio::spawn(async move {
        if !tool_calls.is_empty() {
            let msg = ChatMessage::AssistantWithToolCalls {
                content: if text.is_empty() { None } else { Some(text) },
                tool_calls,
            };
            if let Err(e) = crate::db::insert_message(&pool, &session_id, &msg).await {
                tracing::warn!(err = %e, "failed to persist assistant_with_tool_calls");
            }
            for tr in tool_results {
                let row = ChatMessage::ToolResult(tr);
                if let Err(e) = crate::db::insert_message(&pool, &session_id, &row).await {
                    tracing::warn!(err = %e, "failed to persist tool_result");
                }
            }
        } else if !text.is_empty() {
            let msg = ChatMessage::Assistant { content: text };
            if let Err(e) = crate::db::insert_message(&pool, &session_id, &msg).await {
                tracing::warn!(err = %e, "failed to persist assistant");
            }
        }
    });
}

fn agent_event_to_sse(ev: AgentEvent) -> Event {
    match ev {
        AgentEvent::ReasoningText { text } => Event::default()
            .event("reasoning_text")
            .json_data(serde_json::json!({ "text": text }))
            .expect("reasoning_text json"),
        AgentEvent::ImageGenerated {
            tool_call_id,
            round,
            mime_type,
            image_b64,
        } => Event::default()
            .event("image_generated")
            .json_data(serde_json::json!({
                "tool_call_id": tool_call_id,
                "round": round,
                "mime_type": mime_type,
                "image_b64": image_b64,
            }))
            .expect("image_generated json"),
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
            handled_by,
        } => Event::default()
            .event("tool_call_result")
            .json_data(serde_json::json!({
                "id": id,
                "tool_name": tool_name,
                "args": args,
                "result": result,
                "is_error": is_error,
                "round": round,
                "handled_by": handled_by,
            }))
            .expect("tool_call_result json"),
        AgentEvent::AgentDone { total_rounds } => Event::default()
            .event("agent_done")
            .json_data(serde_json::json!({ "total_rounds": total_rounds }))
            .expect("agent_done json"),
    }
}
