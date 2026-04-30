use std::collections::HashMap;

use thiserror::Error;

use super::action_economy::consume_action;
use super::attack::{AttackOutcome, roll_attack};
use super::combatant::Combatant;
use super::conditions::AttackModifier;
use super::damage::{DamageResistance, apply_damage_to_combatant};
use super::initiative::InitiativeOrder;
use super::result_events::{DamageApplication, DiceRoll, ResultEvents};
use super::types::{ActionKind, CombatantId, DamageType};
use crate::dice::{DiceExpr, roll_expr_detailed};
use crate::rng::SeededRng;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("it is not your turn")]
    NotYourTurn,
    #[error("target is invalid (self-targeting, dead, or unknown)")]
    InvalidTarget,
    #[error("action economy: {0}")]
    ActionEconomy(String),
    #[error("action already used this turn")]
    ActionExhausted,
    #[error("target not found")]
    TargetNotFound,
    #[error("attacker not found")]
    AttackerNotFound,
}

pub enum CombatAction {
    Attack {
        attacker: CombatantId,
        target: CombatantId,
        attack_modifier: i32,
        damage_expr: DiceExpr,
        damage_type: DamageType,
    },
    // M3 will add: CastSpell, Move, Dash, Disengage, Dodge, UseItem, Improvised
}

pub struct CombatResolver {
    pub combatants: HashMap<CombatantId, Combatant>,
    pub order: InitiativeOrder,
    rng: SeededRng,
}

impl CombatResolver {
    pub fn new(
        combatants: HashMap<CombatantId, Combatant>,
        order: InitiativeOrder,
        rng: SeededRng,
    ) -> Self {
        Self { combatants, order, rng }
    }

    /// Phase 1 (Validate) + Phase 2 (Resolve) in one call.
    /// Returns ResultEvents on success, or ValidationError if the action is illegal.
    pub fn resolve(&mut self, action: CombatAction) -> Result<ResultEvents, ValidationError> {
        match action {
            CombatAction::Attack {
                attacker,
                target,
                attack_modifier,
                damage_expr,
                damage_type,
            } => self.resolve_attack(attacker, target, attack_modifier, damage_expr, damage_type),
        }
    }

    fn resolve_attack(
        &mut self,
        attacker_id: CombatantId,
        target_id: CombatantId,
        attack_modifier: i32,
        damage_expr: DiceExpr,
        damage_type: DamageType,
    ) -> Result<ResultEvents, ValidationError> {
        // --- Phase 1: Validate ---
        let current_id = self.order.current().id;
        if attacker_id != current_id {
            return Err(ValidationError::NotYourTurn);
        }
        if attacker_id == target_id {
            return Err(ValidationError::InvalidTarget);
        }
        if !self.combatants.contains_key(&target_id) {
            return Err(ValidationError::TargetNotFound);
        }
        {
            let attacker = self
                .combatants
                .get_mut(&attacker_id)
                .ok_or(ValidationError::AttackerNotFound)?;
            consume_action(&mut attacker.budget, ActionKind::Action)
                .map_err(|e| ValidationError::ActionEconomy(e.to_string()))?;
        }

        // --- Phase 2: Resolve ---
        let target_ac = self.combatants[&target_id].ac;
        let attack_outcome =
            roll_attack(attack_modifier, target_ac, AttackModifier::Normal, &mut self.rng);

        let mut events = ResultEvents::default();

        match &attack_outcome {
            AttackOutcome::CriticalHit { natural, total } => {
                // PHB crit: roll damage dice twice, add modifier once.
                let normal_detail = roll_expr_detailed(&damage_expr, &mut self.rng);
                let extra_detail = roll_expr_detailed(
                    &DiceExpr { modifier: 0, ..damage_expr.clone() },
                    &mut self.rng,
                );
                let raw_damage = normal_detail.total + extra_detail.rolls.iter().sum::<i32>();
                events.rolls.push(DiceRoll {
                    purpose: format!("crit attack roll vs AC {target_ac}"),
                    detail: crate::dice::RollDetail {
                        rolls: vec![*natural],
                        modifier: attack_modifier,
                        total: *total,
                    },
                    natural: *natural,
                });
                let target = self.combatants.get_mut(&target_id).unwrap();
                let resist = DamageResistance::default();
                let effective = apply_damage_to_combatant(target, raw_damage, damage_type, &resist);
                events.damage.push(DamageApplication {
                    target: target_id,
                    raw_amount: raw_damage,
                    effective_amount: effective,
                    damage_type,
                    new_hp: target.current_hp,
                    was_critical: true,
                });
                if target.is_at_zero_hp() {
                    events.deaths.push(target_id);
                }
            }
            AttackOutcome::CriticalMiss { natural } => {
                events.rolls.push(DiceRoll {
                    purpose: format!("critical miss vs AC {target_ac}"),
                    detail: crate::dice::RollDetail {
                        rolls: vec![*natural],
                        modifier: attack_modifier,
                        total: *natural + attack_modifier,
                    },
                    natural: *natural,
                });
            }
            AttackOutcome::Hit { natural, total } => {
                let dmg_detail = roll_expr_detailed(&damage_expr, &mut self.rng);
                events.rolls.push(DiceRoll {
                    purpose: format!("attack roll vs AC {target_ac}"),
                    detail: crate::dice::RollDetail {
                        rolls: vec![*natural],
                        modifier: attack_modifier,
                        total: *total,
                    },
                    natural: *natural,
                });
                let raw_damage = dmg_detail.total;
                let target = self.combatants.get_mut(&target_id).unwrap();
                let resist = DamageResistance::default();
                let effective = apply_damage_to_combatant(target, raw_damage, damage_type, &resist);
                events.damage.push(DamageApplication {
                    target: target_id,
                    raw_amount: raw_damage,
                    effective_amount: effective,
                    damage_type,
                    new_hp: target.current_hp,
                    was_critical: false,
                });
                if target.is_at_zero_hp() {
                    events.deaths.push(target_id);
                }
            }
            AttackOutcome::Miss { natural, total } => {
                events.rolls.push(DiceRoll {
                    purpose: format!("miss vs AC {target_ac}"),
                    detail: crate::dice::RollDetail {
                        rolls: vec![*natural],
                        modifier: attack_modifier,
                        total: *total,
                    },
                    natural: *natural,
                });
            }
        }

        Ok(events)
    }
}
