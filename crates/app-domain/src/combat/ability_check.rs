use serde::{Deserialize, Serialize};

use crate::dice::{Die, roll_one};
use crate::rng::SeededRng;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CheckOutcome {
    Success { roll: i32, total: i32 },
    Failure { roll: i32, total: i32 },
    CriticalSuccess { roll: i32 },
}

pub fn roll_ability_check(check_modifier: i32, dc: i32, rng: &mut SeededRng) -> CheckOutcome {
    let roll = roll_one(Die::D20, rng);
    let total = roll + check_modifier;
    if roll == 20 {
        CheckOutcome::CriticalSuccess { roll }
    } else if total >= dc {
        CheckOutcome::Success { roll, total }
    } else {
        CheckOutcome::Failure { roll, total }
    }
}
