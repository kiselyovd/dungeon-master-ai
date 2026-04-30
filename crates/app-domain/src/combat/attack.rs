use serde::{Deserialize, Serialize};

use super::conditions::AttackModifier;
use crate::dice::{Die, roll_one, roll_with_advantage, roll_with_disadvantage};
use crate::rng::SeededRng;

/// Design note: `AttackOutcome` uses an explicit `natural` field (the raw d20
/// face value) rather than a bool flag because the engine needs the natural
/// value to detect crits (20) and fumbles (1) unambiguously, even when
/// modifiers would otherwise put the total above/below the threshold.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AttackOutcome {
    CriticalHit { natural: i32, total: i32 },
    CriticalMiss { natural: i32 },
    Hit { natural: i32, total: i32 },
    Miss { natural: i32, total: i32 },
}

pub struct AttackRoll {
    pub outcome: AttackOutcome,
    pub rolls: Vec<i32>, // the raw d20 roll(s), for ResultEvents logging
}

pub fn roll_attack(
    attack_modifier: i32,
    target_ac: i32,
    advantage: AttackModifier,
    rng: &mut SeededRng,
) -> AttackOutcome {
    let (total_with_mod, _rolls) = match advantage {
        AttackModifier::Advantage => {
            let (t, r) = roll_with_advantage(Die::D20, attack_modifier, rng);
            (t, r.to_vec())
        }
        AttackModifier::Disadvantage => {
            let (t, r) = roll_with_disadvantage(Die::D20, attack_modifier, rng);
            (t, r.to_vec())
        }
        AttackModifier::Normal => {
            let r = roll_one(Die::D20, rng);
            (r + attack_modifier, vec![r])
        }
    };

    // The natural value is the kept die face (without modifier).
    let natural = total_with_mod - attack_modifier;

    if natural == 20 {
        AttackOutcome::CriticalHit { natural, total: total_with_mod }
    } else if natural == 1 {
        AttackOutcome::CriticalMiss { natural }
    } else if total_with_mod >= target_ac {
        AttackOutcome::Hit { natural, total: total_with_mod }
    } else {
        AttackOutcome::Miss { natural, total: total_with_mod }
    }
}
