use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::conditions::Condition;
use super::types::CombatantId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionBudget {
    pub action: bool,
    pub bonus_action: bool,
    pub reaction: bool,
    pub movement_ft: i32,
}

impl ActionBudget {
    pub fn fresh(speed_ft: i32) -> Self {
        Self { action: true, bonus_action: true, reaction: true, movement_ft: speed_ft }
    }

    pub fn refresh_for_new_turn(&mut self, speed_ft: i32) {
        self.action = true;
        self.bonus_action = true;
        // Reaction refreshes at start of turn (RAW).
        self.reaction = true;
        self.movement_ft = speed_ft;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeathSaves {
    pub successes: u8,
    pub failures: u8,
    pub stable: bool,
}

impl DeathSaves {
    pub fn new() -> Self {
        Self { successes: 0, failures: 0, stable: false }
    }

    pub fn record_success(&mut self) -> bool {
        self.successes += 1;
        if self.successes >= 3 {
            self.stable = true;
            true
        } else {
            false
        }
    }

    pub fn record_failure(&mut self) -> bool {
        self.failures += 1;
        self.failures >= 3
    }
}

impl Default for DeathSaves {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Combatant {
    pub id: CombatantId,
    pub name: String,
    pub max_hp: i32,
    pub current_hp: i32,
    pub temp_hp: i32,
    pub ac: i32,
    pub speed_ft: i32,
    pub initiative_roll: i32,
    pub dex_mod: i32,
    pub conditions: HashSet<Condition>,
    pub budget: ActionBudget,
    pub death_saves: DeathSaves,
    pub is_dead: bool,
}

impl Combatant {
    pub fn new(id: CombatantId, name: String, max_hp: i32, current_hp: i32, ac: i32) -> Self {
        Self {
            id,
            name,
            max_hp,
            current_hp,
            temp_hp: 0,
            ac,
            speed_ft: 30,
            initiative_roll: 0,
            dex_mod: 0,
            conditions: HashSet::new(),
            budget: ActionBudget::fresh(30),
            death_saves: DeathSaves::new(),
            is_dead: false,
        }
    }

    /// Apply damage after resistances have been computed. Returns effective HP change.
    pub fn apply_raw_damage(&mut self, amount: i32) -> i32 {
        let bleed_through = (amount - self.temp_hp).max(0);
        self.temp_hp = (self.temp_hp - amount).max(0);
        let actual = bleed_through.min(self.current_hp);
        self.current_hp = (self.current_hp - bleed_through).max(0);
        actual
    }

    pub fn apply_healing(&mut self, amount: i32) {
        if self.is_dead { return; }
        self.current_hp = (self.current_hp + amount).min(self.max_hp);
        if self.current_hp > 0 {
            self.death_saves = DeathSaves::new();
            self.remove_condition(Condition::Unconscious);
        }
    }

    pub fn add_condition(&mut self, c: Condition) {
        self.conditions.insert(c);
    }

    pub fn remove_condition(&mut self, c: Condition) {
        self.conditions.remove(&c);
    }

    pub fn has_condition(&self, c: Condition) -> bool {
        self.conditions.contains(&c)
    }

    pub fn is_incapacitated(&self) -> bool {
        self.has_condition(Condition::Unconscious)
    }

    pub fn is_at_zero_hp(&self) -> bool {
        self.current_hp == 0
    }
}
