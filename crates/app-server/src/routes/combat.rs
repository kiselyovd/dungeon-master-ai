use std::convert::Infallible;
use std::pin::Pin;
use std::sync::Arc;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::Json;
use futures::{stream, Stream};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use app_application::combat::commands::{CombatActionCommand, ResolveCombatCommand};
use app_application::combat::resolve::{ResolveCombatAction, ResolveCombatError};
use app_application::models::combat::{
    CombatProjection, CombatSnapshot, COMBAT_PROJECTION_VERSION,
};
use app_application::ports::repositories::CombatRepository;
use app_domain::combat::combatant::Combatant;
use app_domain::combat::initiative::{InitiativeEntry, InitiativeOrder};
use app_domain::combat::types::{CombatantId, DamageType};
use app_domain::dice::{DiceExpr, Die};

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
    CombatProjection {
        projection: CombatProjection,
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
            CombatSseEvent::CombatProjection { .. } => "combat_projection",
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
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub expected_revision: Option<u64>,
    #[serde(default)]
    pub rng_seed: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct AttackActionArgs {
    attacker_id: Uuid,
    target_id: Uuid,
    #[serde(default)]
    attack_modifier: i32,
    damage_dice: String,
    damage_type: DamageType,
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

    let projection = projection_from_start(encounter_id, &req.initiative_entries);
    let repository = adapter_sqlite::SqliteStore::new(state.db().clone());
    repository
        .create(req.session_id, projection)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;

    // Emit SSE event.
    let started_event = CombatSseEvent::CombatStarted {
        encounter_id,
        initiative: req.initiative_entries.clone(),
    };
    let sse_stream =
        stream::once(async move { Ok::<Event, Infallible>(started_event.to_sse_event()) });

    Ok(Sse::new(sse_stream).keep_alive(KeepAlive::default()))
}

#[tracing::instrument(skip_all, fields(
    encounter_id = %req.encounter_id,
    action_type = %req.action_type,
))]
pub async fn post_combat_action(
    State(state): State<AppState>,
    Json(req): Json<CombatActionRequest>,
) -> Result<impl IntoResponse, AppError> {
    if req.action_type != "attack" {
        use app_application::agent::tool_decoder::decode_tool_call;
        decode_tool_call(&req.action_type, &req.args)
            .map_err(|error| AppError::BadRequest(error.to_string()))?;
        let sse_stream: Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>> =
            Box::pin(stream::empty());
        return Ok(Sse::new(sse_stream).keep_alive(KeepAlive::default()));
    }

    let args: AttackActionArgs = serde_json::from_value(req.args)
        .map_err(|error| AppError::BadRequest(error.to_string()))?;
    let damage_expr = parse_damage_dice(&args.damage_dice)
        .ok_or_else(|| AppError::BadRequest("invalid damage_dice".into()))?;
    let repository = Arc::new(adapter_sqlite::SqliteStore::new(state.db().clone()));
    let use_case = ResolveCombatAction::new(repository);
    let result = use_case
        .execute(ResolveCombatCommand {
            request_id: req.request_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            encounter_id: req.encounter_id,
            expected_revision: req.expected_revision.unwrap_or(0),
            rng_seed: req.rng_seed.unwrap_or(0),
            action: CombatActionCommand::Attack {
                attacker: CombatantId(args.attacker_id),
                target: CombatantId(args.target_id),
                attack_modifier: args.attack_modifier,
                damage_expr,
                damage_type: args.damage_type,
            },
        })
        .await
        .map_err(combat_error)?;
    let event = CombatSseEvent::CombatProjection {
        projection: result.projection,
    };
    let sse_stream: Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>> =
        Box::pin(stream::once(async move {
            Ok::<Event, Infallible>(event.to_sse_event())
        }));
    Ok(Sse::new(sse_stream).keep_alive(KeepAlive::default()))
}

pub async fn post_combat_end(
    State(state): State<AppState>,
    Json(req): Json<EndCombatRequest>,
) -> Result<impl IntoResponse, AppError> {
    let repository = adapter_sqlite::SqliteStore::new(state.db().clone());
    repository
        .end(req.encounter_id)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;

    let ended_event = CombatSseEvent::CombatEnded {
        encounter_id: req.encounter_id,
        reason: "manual_end".into(),
    };
    let sse_stream =
        stream::once(async move { Ok::<Event, Infallible>(ended_event.to_sse_event()) });
    Ok(Sse::new(sse_stream).keep_alive(KeepAlive::default()))
}

fn projection_from_start(encounter_id: Uuid, entries: &[InitiativeEntryDto]) -> CombatProjection {
    let initiative = entries
        .iter()
        .map(|entry| InitiativeEntry {
            id: CombatantId(entry.id),
            roll: entry.roll,
            dex_tiebreak: entry.dex_mod,
        })
        .collect::<Vec<_>>();
    let order = InitiativeOrder::build(initiative);
    let combatants = entries
        .iter()
        .map(|entry| {
            let mut combatant = Combatant::new(
                CombatantId(entry.id),
                entry.name.clone(),
                entry.max_hp,
                entry.hp,
                entry.ac,
            );
            combatant.initiative_roll = entry.roll;
            combatant.dex_mod = entry.dex_mod;
            combatant
        })
        .collect();
    CombatProjection {
        schema_version: COMBAT_PROJECTION_VERSION,
        encounter_id,
        revision: 0,
        snapshot: CombatSnapshot {
            active: true,
            round: 1,
            current_combatant: (!order.is_empty()).then(|| order.current().id),
            initiative: order.as_slice().to_vec(),
            combatants,
        },
        events: Vec::new(),
    }
}

fn parse_damage_dice(input: &str) -> Option<DiceExpr> {
    let normalized = input.trim().to_ascii_lowercase();
    let (count, remainder) = normalized.split_once('d')?;
    let (sides, modifier) = remainder
        .split_once('+')
        .map(|(sides, modifier)| (sides, modifier.parse::<i32>().ok()))
        .or_else(|| {
            remainder
                .split_once('-')
                .map(|(sides, modifier)| (sides, modifier.parse::<i32>().ok().map(|v| -v)))
        })
        .unwrap_or((remainder, Some(0)));
    let die = match sides.parse::<u16>().ok()? {
        4 => Die::D4,
        6 => Die::D6,
        8 => Die::D8,
        10 => Die::D10,
        12 => Die::D12,
        20 => Die::D20,
        100 => Die::D100,
        _ => return None,
    };
    let count = count.parse::<u8>().ok()?;
    (count > 0).then_some(DiceExpr {
        count,
        die,
        modifier: modifier?,
    })
}

fn combat_error(error: ResolveCombatError) -> AppError {
    match error {
        ResolveCombatError::NotFound => AppError::BadRequest("combat encounter not found".into()),
        ResolveCombatError::StaleRevision { expected, actual } => AppError::BadRequest(format!(
            "stale combat revision: expected {expected}, actual {actual}"
        )),
        ResolveCombatError::Rejected(error) => AppError::BadRequest(error.to_string()),
        ResolveCombatError::Persistence => AppError::Internal("combat persistence failed".into()),
    }
}
