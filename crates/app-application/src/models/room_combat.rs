use crate::{agent::hero_intentions::HeroCombatIntent, models::combat::CombatProjection};
use serde::Serialize;

/// Actor, campaign, room epoch and RNG come from trusted server context.
#[derive(Debug, Clone, Serialize)]
pub struct RoomCombatCommand {
    pub request_id: String,
    pub expected_revision: u64,
    pub action: HeroCombatIntent,
}
#[derive(Debug, Clone, Serialize)]
pub struct RoomCombatResult {
    pub projection: CombatProjection,
    pub duplicate: bool,
    pub receipt_revision: u64,
    pub receipt_events: Vec<app_domain::combat::result_events::ResultEvents>,
}
pub fn project_for_owner(projection: &mut CombatProjection, character: uuid::Uuid) {
    for actor in &mut projection.snapshot.combatants {
        if actor.character_id.is_some_and(|id| id.0 != character) {
            actor.loadout = None;
            actor.casting = None;
        }
    }
}
