pub use app_domain::combat::combatant::Combatant;
use app_domain::combat::initiative::InitiativeEntry;
use app_domain::combat::result_events::ResultEvents;
use app_domain::combat::types::CombatantId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
    /// Public cosmetic bindings for actors already present in this encounter.
    /// These never supply combat statistics or geometry.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub appearances: BTreeMap<Uuid, OpponentArchetype>,
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
pub enum OpponentArchetype {
    WaterEcho,
    Goblin,
    Skeleton,
    Zombie,
    Wolf,
    Bandit,
    Orc,
    GiantRat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthoredEncounterKind {
    BellArchiveDefense,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthoredEncounter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map_skin_asset_id: Option<String>,
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
            appearances: Default::default(),
            schema_version: COMBAT_PROJECTION_VERSION,
            encounter_id,
            revision,
            snapshot: CombatSnapshot::default(),
            events: Vec::new(),
        }
    }
}

#[cfg(test)]
mod appearance_tests {
    use super::*;

    #[test]
    fn tool_boundary_retains_explicit_art_and_rejects_unknown_archetypes() {
        use crate::agent::tool_decoder::decode_tool_call;
        let args = serde_json::json!({"initiative_entries":[{"role":"npc","name":"Unrelated label","appearance":"skeleton"}]});
        let command = decode_tool_call("start_combat", &args).unwrap();
        assert_eq!(
            command.to_args()["initiative_entries"][0]["appearance"],
            "skeleton"
        );
        let invalid = serde_json::json!({"initiative_entries":[{"role":"npc","name":"Skeleton","appearance":"unknown"}]});
        assert!(decode_tool_call("start_combat", &invalid).is_err());
        let legacy = serde_json::json!({"initiative_entries":[{"name":"Skeleton"}]});
        let command = decode_tool_call("start_combat", &legacy).unwrap();
        assert!(command.to_args()["initiative_entries"][0]
            .get("appearance")
            .is_none());
    }

    #[test]
    fn authored_appearance_follows_actor_id_and_survives_storage_without_rules_changes() {
        let mut projection = crate::combat::authored::bell_defense(Uuid::new_v4());
        assert_eq!(projection.appearances.len(), 2);
        for actor in &mut projection.snapshot.combatants {
            assert_eq!(
                projection.appearances[&actor.id.0],
                OpponentArchetype::WaterEcho
            );
            assert_eq!((actor.max_hp, actor.ac, actor.speed_ft), (6, 11, 15));
            actor.name = "Another public label".into();
        }
        let encoded = serde_json::to_value(&projection).unwrap();
        let restored: CombatProjection = serde_json::from_value(encoded.clone()).unwrap();
        assert_eq!(restored.appearances, projection.appearances);
        assert_eq!(
            encoded["snapshot"],
            serde_json::to_value(&restored.snapshot).unwrap()
        );
        let mut legacy = encoded;
        legacy.as_object_mut().unwrap().remove("appearances");
        legacy["authored"]
            .as_object_mut()
            .unwrap()
            .remove("map_skin_asset_id");
        assert!(serde_json::from_value::<CombatProjection>(legacy.clone())
            .unwrap()
            .authored
            .unwrap()
            .map_skin_asset_id
            .is_none());
        assert!(serde_json::from_value::<CombatProjection>(legacy)
            .unwrap()
            .appearances
            .is_empty());
    }
}
