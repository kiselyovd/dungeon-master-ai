use super::types::DamageType;
use crate::dice::Die;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellSlots {
    pub maximum: u8,
    pub remaining: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CastingProfile {
    #[serde(default)]
    pub choices: Vec<SpellChoice>,
    pub level: u8,
    pub ability_modifier: i32,
    pub attack_bonus: i32,
    pub save_dc: i32,
    pub prepared: Vec<String>,
    pub slots: BTreeMap<u8, SpellSlots>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellChoice {
    pub id: String,
    pub level: u8,
    pub range_ft: i32,
    pub bonus_action: bool,
    pub healing: bool,
    pub supported: bool,
}

pub fn spell_choices(prepared: &[String]) -> Vec<SpellChoice> {
    prepared
        .iter()
        .map(|id| {
            let rule = spell_rule(id);
            let level = crate::compendium::compendium()
                .spells
                .iter()
                .find(|spell| spell.id == *id)
                .map_or(0, |spell| spell.level as u8);
            SpellChoice {
                id: id.clone(),
                level,
                range_ft: rule.map_or(0, |r| r.range_ft),
                bonus_action: rule.is_some_and(|r| r.bonus_action),
                healing: rule.is_some_and(|r| matches!(r.effect, SpellEffect::Heal(_))),
                supported: rule.is_some(),
            }
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
pub enum SpellEffect {
    Attack(Die, DamageType),
    DexteritySave(Die, DamageType),
    Heal(Die),
    Missiles,
}

#[derive(Debug, Clone, Copy)]
pub struct SpellRule {
    pub level: u8,
    pub range_ft: i32,
    pub bonus_action: bool,
    pub effect: SpellEffect,
}

pub fn spell_rule(id: &str) -> Option<SpellRule> {
    let (level, range_ft, bonus_action, effect) = match id {
        "fire-bolt" => (
            0,
            120,
            false,
            SpellEffect::Attack(Die::D10, DamageType::Fire),
        ),
        "eldritch-blast" => (
            0,
            120,
            false,
            SpellEffect::Attack(Die::D10, DamageType::Force),
        ),
        "sacred-flame" => (
            0,
            60,
            false,
            SpellEffect::DexteritySave(Die::D8, DamageType::Radiant),
        ),
        "cure-wounds" => (1, 5, false, SpellEffect::Heal(Die::D8)),
        "healing-word" => (1, 60, true, SpellEffect::Heal(Die::D4)),
        "magic-missile" => (1, 120, false, SpellEffect::Missiles),
        _ => return None,
    };
    Some(SpellRule {
        level,
        range_ft,
        bonus_action,
        effect,
    })
}

pub fn cantrip_scale(level: u8) -> u8 {
    1 + u8::from(level >= 5) + u8::from(level >= 11) + u8::from(level >= 17)
}

/// Compatibility helper for the first slot level.
pub fn first_level_slots(class: &str, level: u8) -> u8 {
    slot_capacity(class, level).get(&1).copied().unwrap_or(0)
}

pub fn slot_capacity(class: &str, level: u8) -> BTreeMap<u8, u8> {
    if !(1..=20).contains(&level) {
        return BTreeMap::new();
    }
    if class == "warlock" {
        let slot_level = match level {
            1..=2 => 1,
            3..=4 => 2,
            5..=6 => 3,
            7..=8 => 4,
            _ => 5,
        };
        let count = match level {
            1 => 1,
            2..=10 => 2,
            11..=16 => 3,
            _ => 4,
        };
        return BTreeMap::from([(slot_level, count)]);
    }
    let effective = match class {
        "bard" | "cleric" | "druid" | "sorcerer" | "wizard" => level,
        "paladin" | "ranger" => level.div_ceil(2),
        _ => return BTreeMap::new(),
    };
    const FULL: [[u8; 9]; 20] = [
        [2, 0, 0, 0, 0, 0, 0, 0, 0],
        [3, 0, 0, 0, 0, 0, 0, 0, 0],
        [4, 2, 0, 0, 0, 0, 0, 0, 0],
        [4, 3, 0, 0, 0, 0, 0, 0, 0],
        [4, 3, 2, 0, 0, 0, 0, 0, 0],
        [4, 3, 3, 0, 0, 0, 0, 0, 0],
        [4, 3, 3, 1, 0, 0, 0, 0, 0],
        [4, 3, 3, 2, 0, 0, 0, 0, 0],
        [4, 3, 3, 3, 1, 0, 0, 0, 0],
        [4, 3, 3, 3, 2, 0, 0, 0, 0],
        [4, 3, 3, 3, 2, 1, 0, 0, 0],
        [4, 3, 3, 3, 2, 1, 0, 0, 0],
        [4, 3, 3, 3, 2, 1, 1, 0, 0],
        [4, 3, 3, 3, 2, 1, 1, 0, 0],
        [4, 3, 3, 3, 2, 1, 1, 1, 0],
        [4, 3, 3, 3, 2, 1, 1, 1, 0],
        [4, 3, 3, 3, 2, 1, 1, 1, 1],
        [4, 3, 3, 3, 3, 1, 1, 1, 1],
        [4, 3, 3, 3, 3, 2, 1, 1, 1],
        [4, 3, 3, 3, 3, 2, 2, 1, 1],
    ];
    FULL[usize::from(effective - 1)]
        .iter()
        .enumerate()
        .filter(|(_, count)| **count > 0)
        .map(|(index, count)| ((index + 1) as u8, *count))
        .collect()
}
