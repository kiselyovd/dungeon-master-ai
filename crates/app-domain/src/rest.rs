use crate::{
    combat::magic::slot_capacity,
    dice::{roll_expr_detailed, DiceExpr, Die, RollDetail},
    rng::SeededRng,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RestIntent {
    Short { hit_dice: u8 },
    Long,
    Wait { minutes: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestResources {
    pub hit_dice_remaining: u8,
    pub last_long_rest_completed: Option<u64>, // campaign seconds
    pub spell_slots: BTreeMap<u8, u8>,
}
#[derive(Debug, Clone)]
pub struct RestInput {
    pub class_id: String,
    pub level: u8,
    pub hp: i32,
    pub max_hp: i32,
    pub constitution_modifier: i32,
    pub elapsed_seconds: u64,
    pub resources: RestResources,
    pub safe: bool,
    pub combat_active: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestOutcome {
    pub hp: i32,
    pub elapsed_seconds: u64,
    pub duration_minutes: u16,
    pub resources: RestResources,
    pub rolls: Vec<RollDetail>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RestError {
    #[error("rest requires a safe location outside combat")]
    Unsafe,
    #[error("rest requires a living character with at least one hit point")]
    Downed,
    #[error("rest resources or intention are invalid")]
    Invalid,
    #[error("sixteen hours must pass before starting another long rest")]
    TooSoon,
}

pub fn resolve_rest(
    mut state: RestInput,
    intent: RestIntent,
    rng: &mut SeededRng,
) -> Result<RestOutcome, RestError> {
    if !state.safe || state.combat_active {
        return Err(RestError::Unsafe);
    }
    if state.hp <= 0 {
        return Err(RestError::Downed);
    }
    if !(1..=20).contains(&state.level)
        || state.hp > state.max_hp
        || state.resources.hit_dice_remaining > state.level
        || state
            .resources
            .last_long_rest_completed
            .is_some_and(|time| time > state.elapsed_seconds)
    {
        return Err(RestError::Invalid);
    }
    let class = crate::compendium::compendium()
        .classes
        .iter()
        .find(|class| class.id == state.class_id)
        .ok_or(RestError::Invalid)?;
    let die = match class.hit_die {
        6 => Die::D6,
        8 => Die::D8,
        10 => Die::D10,
        12 => Die::D12,
        _ => return Err(RestError::Invalid),
    };
    let duration_minutes = match intent {
        RestIntent::Short { hit_dice } => {
            if hit_dice > state.resources.hit_dice_remaining {
                return Err(RestError::Invalid);
            }
            60
        }
        RestIntent::Long => {
            if state
                .resources
                .last_long_rest_completed
                .is_some_and(|last| state.elapsed_seconds.saturating_sub(last) < 16 * 60 * 60)
            {
                return Err(RestError::TooSoon);
            }
            480
        }
        RestIntent::Wait { minutes } => {
            if !(1..=1440).contains(&minutes) {
                return Err(RestError::Invalid);
            }
            minutes
        }
    };
    let elapsed_seconds = state
        .elapsed_seconds
        .checked_add(u64::from(duration_minutes) * 60)
        .filter(|time| *time <= i64::MAX as u64)
        .ok_or(RestError::Invalid)?;
    let capacity = slot_capacity(&state.class_id, state.level);
    let mut rolls = Vec::new();
    match intent {
        RestIntent::Short { hit_dice } => {
            for _ in 0..hit_dice {
                let detail = roll_expr_detailed(
                    &DiceExpr {
                        count: 1,
                        die,
                        modifier: state.constitution_modifier,
                    },
                    rng,
                );
                state.hp = state
                    .hp
                    .saturating_add(detail.total.max(1))
                    .min(state.max_hp);
                rolls.push(detail);
            }
            state.resources.hit_dice_remaining -= hit_dice;
            if state.class_id == "warlock" {
                state.resources.spell_slots = capacity;
            }
        }
        RestIntent::Long => {
            state.hp = state.max_hp;
            state.resources.hit_dice_remaining = state.level;
            state.resources.spell_slots = capacity;
            state.resources.last_long_rest_completed = Some(elapsed_seconds);
        }
        RestIntent::Wait { .. } => {}
    }
    Ok(RestOutcome {
        hp: state.hp,
        elapsed_seconds,
        duration_minutes,
        resources: state.resources,
        rolls,
    })
}
