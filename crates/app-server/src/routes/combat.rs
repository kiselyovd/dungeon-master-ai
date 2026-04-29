use std::convert::Infallible;

use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::state::AppState;

// ---- SSE event types ----

/// All SSE events emitted during combat. Each variant maps to one SSE `event:` name.
/// M3 will stream these through the LLM narration path; M2 emits them from HTTP handlers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CombatSseEvent {
    CombatStarted {
        encounter_id: Uuid,
        initiative: Vec<InitiativeEntryDto>,
    },
    TurnStarted {
        encounter_id: Uuid,
        round: u32,
        active_id: Uuid,
        active_name: String,
    },
    DamageApplied {
        target_id: Uuid,
        amount: i32,
        new_hp: i32,
        damage_type: String,
        was_critical: bool,
    },
    ConditionAdded {
        target_id: Uuid,
        condition: String,
    },
    ConditionRemoved {
        target_id: Uuid,
        condition: String,
    },
    CombatEnded {
        encounter_id: Uuid,
        reason: String,
    },
}

impl CombatSseEvent {
    pub fn event_name(&self) -> &'static str {
        match self {
            CombatSseEvent::CombatStarted { .. } => "combat_started",
            CombatSseEvent::TurnStarted { .. } => "turn_started",
            CombatSseEvent::DamageApplied { .. } => "damage_applied",
            CombatSseEvent::ConditionAdded { .. } => "condition_added",
            CombatSseEvent::ConditionRemoved { .. } => "condition_removed",
            CombatSseEvent::CombatEnded { .. } => "combat_ended",
        }
    }

    pub fn to_sse_event(&self) -> Event {
        Event::default()
            .event(self.event_name())
            .json_data(self)
            .expect("json_data")
    }
}

// ---- Request / response DTOs ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitiativeEntryDto {
    pub id: Uuid,
    pub name: String,
    pub roll: i32,
    pub dex_mod: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub ac: i32,
}

#[derive(Debug, Deserialize)]
pub struct StartCombatRequest {
    pub campaign_id: Uuid,
    pub session_id: Uuid,
    pub initiative_entries: Vec<InitiativeEntryDto>,
}

#[derive(Debug, Serialize)]
pub struct StartCombatResponse {
    pub encounter_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CombatActionRequest {
    pub encounter_id: Uuid,
    pub action_type: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct EndCombatRequest {
    pub encounter_id: Uuid,
}

// ---- Route handlers ----

pub async fn post_combat_start(
    State(state): State<AppState>,
    Json(req): Json<StartCombatRequest>,
) -> Result<impl IntoResponse, AppError> {
    let encounter_id = Uuid::new_v4();

    // Persist the encounter skeleton.
    let initiative_json = serde_json::to_string(&req.initiative_entries)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"INSERT INTO combat_encounters (id, session_id, round, started_at, initiative)
           VALUES (?1, ?2, 1, ?3, ?4)"#,
    )
    .bind(encounter_id.to_string())
    .bind(req.session_id.to_string())
    .bind(now)
    .bind(initiative_json)
    .execute(state.db())
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    // Insert tokens.
    for entry in &req.initiative_entries {
        sqlx::query(
            r#"INSERT INTO combat_tokens
               (id, encounter_id, name, current_hp, max_hp, ac, pos_x, pos_y, conditions)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 0, '[]')"#,
        )
        .bind(entry.id.to_string())
        .bind(encounter_id.to_string())
        .bind(&entry.name)
        .bind(entry.hp)
        .bind(entry.max_hp)
        .bind(entry.ac)
        .execute(state.db())
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    }

    // Emit SSE event.
    let started_event = CombatSseEvent::CombatStarted {
        encounter_id,
        initiative: req.initiative_entries.clone(),
    };
    let sse_stream =
        stream::once(async move { Ok::<Event, Infallible>(started_event.to_sse_event()) });

    Ok(Sse::new(sse_stream).keep_alive(KeepAlive::default()))
}

pub async fn post_combat_action(
    State(_state): State<AppState>,
    Json(req): Json<CombatActionRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Phase 1: Validate via tool-call validator.
    use app_domain::combat::validator::validate_tool_call;
    let _validated = validate_tool_call(&req.action_type, req.args.clone())
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    // Phase 2 (full resolution) is wired in M3 when the LLM agent loop drives
    // the resolver. M2 returns an empty success SSE stream after validation passes.
    let sse_stream = stream::empty::<Result<Event, Infallible>>();
    Ok(Sse::new(sse_stream).keep_alive(KeepAlive::default()))
}

pub async fn post_combat_end(
    State(state): State<AppState>,
    Json(req): Json<EndCombatRequest>,
) -> Result<impl IntoResponse, AppError> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(r#"UPDATE combat_encounters SET ended_at = ?1 WHERE id = ?2"#)
        .bind(now)
        .bind(req.encounter_id.to_string())
        .execute(state.db())
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let ended_event = CombatSseEvent::CombatEnded {
        encounter_id: req.encounter_id,
        reason: "manual_end".into(),
    };
    let sse_stream =
        stream::once(async move { Ok::<Event, Infallible>(ended_event.to_sse_event()) });
    Ok(Sse::new(sse_stream).keep_alive(KeepAlive::default()))
}
