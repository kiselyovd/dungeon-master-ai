use serde::{Deserialize, Serialize};

use crate::dice::{Die, roll_one};
use crate::rng::SeededRng;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SaveOutcome {
    Success { roll: i32, total: i32 },
    Failure { roll: i32, total: i32 },
}

pub fn roll_save(save_modifier: i32, dc: i32, rng: &mut SeededRng) -> SaveOutcome {
    let roll = roll_one(Die::D20, rng);
    let total = roll + save_modifier;
    if total >= dc {
        SaveOutcome::Success { roll, total }
    } else {
        SaveOutcome::Failure { roll, total }
    }
}

/// Concentration check DC = max(10, damage_taken / 2), per RAW PHB p203.
pub fn concentration_check_dc(damage_taken: i32) -> i32 {
    10.max(damage_taken / 2)
}
