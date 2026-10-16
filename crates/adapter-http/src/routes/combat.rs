use std::convert::Infallible;
use std::pin::Pin;

use app_application::models::combat::CombatProjection;
use axum::extract::Extension;
use axum::http::{header, HeaderValue};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::Json;
use futures::{stream, Stream};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::services::{CombatActionCommand, CombatStartCommand, HttpServiceError, HttpServices};

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

fn close_sse<S>(sse: Sse<S>) -> axum::response::Response
where
    S: Stream<Item = Result<Event, Infallible>> + Send + 'static,
{
    let mut response = sse.into_response();
    response
        .headers_mut()
        .insert(header::CONNECTION, HeaderValue::from_static("close"));
    response
}

impl CombatSseEvent {
    pub fn event_name(&self) -> &'static str {
        match self {
            Self::CombatStarted { .. } => "combat_started",
            Self::TurnStarted { .. } => "turn_started",
            Self::DamageApplied { .. } => "damage_applied",
            Self::ConditionAdded { .. } => "condition_added",
            Self::ConditionRemoved { .. } => "condition_removed",
            Self::CombatEnded { .. } => "combat_ended",
            Self::CombatProjection { .. } => "combat_projection",
        }
    }

    pub fn to_sse_event(&self) -> Event {
        Event::default()
            .event(self.event_name())
            .json_data(self)
            .expect("combat event is JSON serializable")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
pub struct EndCombatRequest {
    pub encounter_id: Uuid,
}

pub async fn post_combat_start(
    Extension(services): Extension<HttpServices>,
    Json(request): Json<StartCombatRequest>,
) -> Result<axum::response::Response, HttpServiceError> {
    let initiative = request.initiative_entries;
    let encounter_id = services
        .combat
        .start(CombatStartCommand {
            campaign_id: request.campaign_id,
            session_id: request.session_id,
            initiative_entries: initiative.clone(),
        })
        .await?;
    let event = CombatSseEvent::CombatStarted {
        encounter_id,
        initiative,
    };
    Ok(close_sse(
        Sse::new(stream::once(async move {
            Ok::<Event, Infallible>(event.to_sse_event())
        }))
        .keep_alive(KeepAlive::default()),
    ))
}

pub async fn post_combat_action(
    Extension(services): Extension<HttpServices>,
    Json(request): Json<CombatActionRequest>,
) -> Result<impl IntoResponse, HttpServiceError> {
    let projection = services
        .combat
        .action(CombatActionCommand {
            encounter_id: request.encounter_id,
            action_type: request.action_type,
            args: request.args,
            request_id: request.request_id,
            expected_revision: request.expected_revision,
            rng_seed: request.rng_seed,
        })
        .await?;
    let stream: Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>> = match projection {
        Some(projection) => {
            let event = CombatSseEvent::CombatProjection { projection };
            Box::pin(stream::once(async move {
                Ok::<Event, Infallible>(event.to_sse_event())
            }))
        }
        None => Box::pin(stream::empty()),
    };
    Ok(close_sse(Sse::new(stream).keep_alive(KeepAlive::default())))
}

pub async fn post_combat_end(
    Extension(services): Extension<HttpServices>,
    Json(request): Json<EndCombatRequest>,
) -> Result<axum::response::Response, HttpServiceError> {
    services.combat.end(request.encounter_id).await?;
    let event = CombatSseEvent::CombatEnded {
        encounter_id: request.encounter_id,
        reason: "manual_end".into(),
    };
    Ok(close_sse(
        Sse::new(stream::once(async move {
            Ok::<Event, Infallible>(event.to_sse_event())
        }))
        .keep_alive(KeepAlive::default()),
    ))
}
