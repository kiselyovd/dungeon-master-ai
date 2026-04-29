use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::rng::SeededRng;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Die {
    D4,
    D6,
    D8,
    D10,
    D12,
    D20,
    D100,
}

impl Die {
    pub fn sides(self) -> i32 {
        match self {
            Die::D4 => 4,
            Die::D6 => 6,
            Die::D8 => 8,
            Die::D10 => 10,
            Die::D12 => 12,
            Die::D20 => 20,
            Die::D100 => 100,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiceExpr {
    pub count: u8,
    pub die: Die,
    pub modifier: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollDetail {
    pub rolls: Vec<i32>,
    pub modifier: i32,
    pub total: i32,
}

pub fn roll_one(die: Die, rng: &mut SeededRng) -> i32 {
    rng.inner_mut().random_range(1..=die.sides())
}

pub fn roll_expr(expr: &DiceExpr, rng: &mut SeededRng) -> i32 {
    roll_expr_detailed(expr, rng).total
}

pub fn roll_expr_detailed(expr: &DiceExpr, rng: &mut SeededRng) -> RollDetail {
    let rolls: Vec<i32> = (0..expr.count).map(|_| roll_one(expr.die, rng)).collect();
    let total = rolls.iter().sum::<i32>() + expr.modifier;
    RollDetail { rolls, modifier: expr.modifier, total }
}

/// Roll 2d20, keep the higher. Returns `(kept_total, [roll1, roll2])`.
pub fn roll_with_advantage(die: Die, modifier: i32, rng: &mut SeededRng) -> (i32, [i32; 2]) {
    let a = roll_one(die, rng);
    let b = roll_one(die, rng);
    let kept = a.max(b) + modifier;
    (kept, [a, b])
}

/// Roll 2d20, keep the lower. Returns `(kept_total, [roll1, roll2])`.
pub fn roll_with_disadvantage(die: Die, modifier: i32, rng: &mut SeededRng) -> (i32, [i32; 2]) {
    let a = roll_one(die, rng);
    let b = roll_one(die, rng);
    let kept = a.min(b) + modifier;
    (kept, [a, b])
}
