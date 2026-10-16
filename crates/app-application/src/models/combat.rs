use app_domain::combat::combatant::Combatant;
use app_domain::combat::initiative::InitiativeEntry;
use app_domain::combat::result_events::ResultEvents;
use app_domain::combat::types::CombatantId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const COMBAT_PROJECTION_VERSION: u16 = 1;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CombatSnapshot {
    pub active: bool,
    pub round: u32,
    pub current_combatant: Option<CombatantId>,
    pub initiative: Vec<InitiativeEntry>,
    pub combatants: Vec<Combatant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatProjection {
    pub schema_version: u16,
    pub encounter_id: Uuid,
    pub revision: u64,
    pub snapshot: CombatSnapshot,
    pub events: Vec<ResultEvents>,
}

impl CombatProjection {
    pub fn empty(encounter_id: Uuid, revision: u64) -> Self {
        Self {
            schema_version: COMBAT_PROJECTION_VERSION,
            encounter_id,
            revision,
            snapshot: CombatSnapshot::default(),
            events: Vec::new(),
        }
    }
}
