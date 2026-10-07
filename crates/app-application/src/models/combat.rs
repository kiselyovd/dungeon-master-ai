pub use app_domain::combat::combatant::Combatant;
use app_domain::combat::initiative::InitiativeEntry;
use app_domain::combat::result_events::ResultEvents;
use app_domain::combat::types::CombatantId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const COMBAT_PROJECTION_VERSION: u16 = 1;

#[derive(Debug, Clone)]
pub struct CombatActionCommit {
    pub projection: CombatProjection,
    pub duplicate: bool,
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authored: Option<AuthoredEncounter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adventure: Option<crate::models::adventure::AdventureProjection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub campaign_scope: Option<CombatCampaignScope>,
    pub schema_version: u16,
    pub encounter_id: Uuid,
    pub revision: u64,
    pub snapshot: CombatSnapshot,
    pub events: Vec<ResultEvents>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthoredEncounterKind {
    BellArchiveDefense,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthoredEncounter {
    pub kind: AuthoredEncounterKind,
    pub map: app_domain::combat::geometry::BattleMap,
    pub outcome: Option<app_domain::adventure::DefenseOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatCampaignScope {
    pub campaign_id: Uuid,
    pub session_id: Uuid,
    pub character_id: Uuid,
    pub revision: u64,
}

impl CombatProjection {
    pub fn empty(encounter_id: Uuid, revision: u64) -> Self {
        Self {
            authored: None,
            adventure: None,
            campaign_scope: None,
            schema_version: COMBAT_PROJECTION_VERSION,
            encounter_id,
            revision,
            snapshot: CombatSnapshot::default(),
            events: Vec::new(),
        }
    }
}
