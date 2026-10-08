use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::conditions::Condition;
use super::types::{CharacterId, CombatantId, Position};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionBudget {
    #[serde(default)]
    pub spell_slot_used: bool,
    pub action: bool,
    pub bonus_action: bool,
    pub reaction: bool,
    pub movement_ft: i32,
}

impl ActionBudget {
    pub fn fresh(speed_ft: i32) -> Self {
        Self {
            spell_slot_used: false,
            action: true,
            bonus_action: true,
            reaction: true,
            movement_ft: speed_ft,
        }
    }

    pub fn refresh_for_new_turn(&mut self, speed_ft: i32) {
        self.spell_slot_used = false;
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
        Self {
            successes: 0,
            failures: 0,
            stable: false,
        }
    }

    pub fn record_success(&mut self) -> bool {
        self.successes = self.successes.saturating_add(1).min(3);
        if self.successes >= 3 {
            self.stabilize();
            true
        } else {
            false
        }
    }

    pub fn record_failure(&mut self) -> bool {
        self.failures = self.failures.saturating_add(1).min(3);
        self.failures >= 3
    }
    pub fn stabilize(&mut self) {
        self.successes = 0;
        self.failures = 0;
        self.stable = true;
    }
}

impl Default for DeathSaves {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Combatant {
    #[serde(default)]
    pub untrained_armor: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name_key: Option<String>,
    pub id: CombatantId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub character_id: Option<CharacterId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loadout: Option<super::loadout::CombatLoadout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub casting: Option<super::magic::CastingProfile>,
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
    #[serde(default)]
    pub position: Position,
}

impl Combatant {
    pub fn new(id: CombatantId, name: String, max_hp: i32, current_hp: i32, ac: i32) -> Self {
        Self {
            display_name_key: None,
            id,
            character_id: None,
            untrained_armor: false,
            loadout: None,
            casting: None,
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
            position: Position::default(),
        }
    }

    /// Apply damage after resistances have been computed. Returns effective HP change.
    pub fn apply_raw_damage(&mut self, amount: i32) -> i32 {
        self.apply_raw_damage_with_critical(amount, false)
    }

    pub fn apply_raw_damage_with_critical(&mut self, amount: i32, critical: bool) -> i32 {
        let amount = amount.max(0);
        if amount == 0 || self.is_dead {
            return 0;
        }
        let hp_before = self.current_hp;
        let bleed_through = amount.saturating_sub(self.temp_hp).max(0);
        self.temp_hp = self.temp_hp.saturating_sub(amount).max(0);
        let actual = bleed_through.min(hp_before);
        self.current_hp = hp_before.saturating_sub(bleed_through).max(0);
        if hp_before == 0 {
            self.death_saves.stable = false;
            for _ in 0..if critical { 2 } else { 1 } {
                self.is_dead |= self.death_saves.record_failure();
            }
            self.is_dead |= amount >= self.max_hp;
        } else if self.current_hp == 0 {
            self.death_saves = DeathSaves::default();
            self.is_dead = bleed_through.saturating_sub(hp_before) >= self.max_hp;
        }
        if self.current_hp == 0 {
            self.add_condition(Condition::Unconscious);
        }
        actual
    }

    pub fn apply_healing(&mut self, amount: i32) {
        if self.is_dead {
            return;
        }
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
