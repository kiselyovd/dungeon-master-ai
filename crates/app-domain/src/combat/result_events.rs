use serde::{Deserialize, Serialize};

use super::types::{AbilityScore, CombatantId, CharacterId, DamageType, Position, SlotLevel};
use crate::dice::RollDetail;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiceRoll {
    pub purpose: String,
    pub detail: RollDetail,
    pub natural: i32, // the raw d20 face value for attack/save rolls
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageApplication {
    pub target: CombatantId,
    pub raw_amount: i32,
    pub effective_amount: i32, // after resistance/immunity/vulnerability
    pub damage_type: DamageType,
    pub new_hp: i32,
    pub was_critical: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealingApplication {
    pub target: CombatantId,
    pub amount: i32,
    pub new_hp: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionChange {
    pub target: CombatantId,
    pub condition: String, // serialized Condition key
    pub duration_rounds: Option<u32>,
    pub source: Option<CombatantId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovementEvent {
    pub combatant: CombatantId,
    pub from: Position,
    pub to: Position,
    pub feet_used: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeathSaveResult {
    pub combatant: CombatantId,
    pub roll: i32,
    pub success: bool,
    pub successes_total: u8,
    pub failures_total: u8,
    pub dead: bool,
    pub stable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellSlotConsumption {
    pub character: CharacterId,
    pub level: SlotLevel,
}

/// The complete mechanical output of Phase 2 (Resolution).
/// The LLM receives this in Phase 3 (Narration) and must narrate
/// exactly these events - it cannot add, remove, or contradict them.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResultEvents {
    pub rolls: Vec<DiceRoll>,
    pub damage: Vec<DamageApplication>,
    pub healing: Vec<HealingApplication>,
    pub conditions_added: Vec<ConditionChange>,
    pub conditions_removed: Vec<ConditionChange>,
    pub deaths: Vec<CombatantId>,
    pub death_saves: Vec<DeathSaveResult>,
    pub movement: Vec<MovementEvent>,
    pub spell_slots_consumed: Vec<SpellSlotConsumption>,
    pub ability_scores_checked: Vec<(CombatantId, AbilityScore, i32)>, // (who, which, DC)
}
