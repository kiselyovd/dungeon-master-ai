use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::combatant::Combatant;
use super::types::DamageType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DamageRelation {
    Normal,
    Resistant,
    Immune,
    Vulnerable,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DamageResistance {
    table: HashMap<String, DamageRelation>,
}

impl DamageResistance {
    pub fn set(&mut self, damage_type: DamageType, relation: DamageRelation) {
        let key = format!("{damage_type:?}").to_lowercase();
        self.table.insert(key, relation);
    }

    pub fn get(&self, damage_type: DamageType) -> DamageRelation {
        let key = format!("{damage_type:?}").to_lowercase();
        self.table.get(&key).copied().unwrap_or(DamageRelation::Normal)
    }
}

/// Compute effective damage after applying resistance/immunity/vulnerability.
pub fn compute_effective_damage(
    raw: i32,
    damage_type: DamageType,
    resist: &DamageResistance,
) -> i32 {
    match resist.get(damage_type) {
        DamageRelation::Normal => raw,
        DamageRelation::Resistant => raw / 2,
        DamageRelation::Immune => 0,
        DamageRelation::Vulnerable => raw * 2,
    }
}

/// Apply damage to a combatant, running through temp HP first.
/// Returns the effective HP reduction.
pub fn apply_damage_to_combatant(
    combatant: &mut Combatant,
    raw_amount: i32,
    damage_type: DamageType,
    resist: &DamageResistance,
) -> i32 {
    let effective = compute_effective_damage(raw_amount, damage_type, resist);
    combatant.apply_raw_damage(effective)
}
