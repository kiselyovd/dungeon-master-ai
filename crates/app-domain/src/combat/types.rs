use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---- Identifiers ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CombatantId(pub Uuid);

impl CombatantId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for CombatantId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CharacterId(pub Uuid);

impl CharacterId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for CharacterId {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Grid ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    /// Chebyshev distance (5e diagonal rule: every other diagonal = 10ft).
    pub fn chebyshev(self, other: Position) -> i32 {
        let dx = (self.x - other.x).abs();
        let dy = (self.y - other.y).abs();
        dx.max(dy)
    }
    /// Cell distance in feet (5-foot squares).
    pub fn distance_ft(self, other: Position) -> i32 {
        self.chebyshev(other) * 5
    }
}

// ---- Ability scores ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbilityScore {
    Str,
    Dex,
    Con,
    Int,
    Wis,
    Cha,
}

pub fn ability_modifier(score: i32) -> i32 {
    (score - 10).div_euclid(2)
}

// ---- Damage ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DamageType {
    Acid,
    Bludgeoning,
    Cold,
    Fire,
    Force,
    Lightning,
    Necrotic,
    Piercing,
    Poison,
    Psychic,
    Radiant,
    Slashing,
    Thunder,
}

// ---- Spell slots ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SlotLevel(pub u8); // 1-9

impl SlotLevel {
    pub fn new(level: u8) -> Option<Self> {
        if (1..=9).contains(&level) { Some(Self(level)) } else { None }
    }
}

// ---- Action kinds ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Action,
    BonusAction,
    Reaction,
    Movement,
}
