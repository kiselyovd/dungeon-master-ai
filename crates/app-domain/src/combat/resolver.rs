use std::collections::HashMap;

use thiserror::Error;

use super::action_economy::{consume_action, consume_movement_ft};
use super::attack::{roll_attack_detailed, AttackOutcome};
use super::combatant::Combatant;
use super::conditions::AttackModifier;
use super::damage::{apply_damage_to_combatant, DamageResistance};
use super::initiative::InitiativeOrder;
use super::result_events::{DamageApplication, DiceRoll, MovementEvent, ResultEvents};
use super::types::{ActionKind, CombatantId, DamageType, Position};
use crate::dice::{roll_expr_detailed, DiceExpr};
use crate::rng::SeededRng;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("combat encounter is empty or invalid")]
    InvalidEncounter,
    #[error("actor cannot act while unconscious or defeated")]
    Incapacitated,
    #[error("weapon is not owned or the requested mode is unavailable")]
    WeaponUnavailable,
    #[error("target is outside weapon range")]
    OutOfRange,
    #[error("ammunition is exhausted")]
    AmmunitionExhausted,
    #[error("the selected attack needs a free second hand")]
    HandsOccupied,
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
    #[error("combatant not found")]
    CombatantNotFound,
}

pub enum CombatAction {
    NpcTurn {
        combatant: CombatantId,
    },
    WeaponAttack {
        attacker: CombatantId,
        target: CombatantId,
        weapon_id: Option<String>,
        mode: super::loadout::WeaponMode,
        two_handed: bool,
    },
    Attack {
        attacker: CombatantId,
        target: CombatantId,
        attack_modifier: i32,
        damage_expr: DiceExpr,
        damage_type: DamageType,
    },
    Cast {
        combatant: CombatantId,
    },
    Move {
        combatant: CombatantId,
        to: Position,
    },
    EndTurn {
        combatant: CombatantId,
    },
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
        Self {
            combatants,
            order,
            rng,
        }
    }

    /// Phase 1 (Validate) + Phase 2 (Resolve) in one call.
    /// Returns ResultEvents on success, or ValidationError if the action is illegal.
    pub fn resolve(&mut self, action: CombatAction) -> Result<ResultEvents, ValidationError> {
        if self.order.is_empty() {
            return Err(ValidationError::InvalidEncounter);
        }
        match action {
            CombatAction::NpcTurn { combatant } => self.resolve_npc_turn(combatant),
            CombatAction::WeaponAttack {
                attacker,
                target,
                weapon_id,
                mode,
                two_handed,
            } => self.resolve_weapon_attack(attacker, target, weapon_id, mode, two_handed),
            CombatAction::Attack {
                attacker,
                target,
                attack_modifier,
                damage_expr,
                damage_type,
            } => {
                if self
                    .combatants
                    .get(&attacker)
                    .is_some_and(|actor| actor.loadout.is_some() || actor.character_id.is_some())
                {
                    self.resolve_weapon_attack(
                        attacker,
                        target,
                        None,
                        super::loadout::WeaponMode::Default,
                        false,
                    )
                } else {
                    self.resolve_attack(
                        attacker,
                        target,
                        attack_modifier,
                        damage_expr,
                        damage_type,
                        AttackModifier::Normal,
                    )
                }
            }
            CombatAction::Cast { combatant } => self.resolve_cast(combatant),
            CombatAction::Move { combatant, to } => self.resolve_move(combatant, to),
            CombatAction::EndTurn { combatant } => self.resolve_end_turn(combatant),
        }
    }

    fn require_current(&self, combatant: CombatantId) -> Result<(), ValidationError> {
        if self.order.current().id != combatant {
            return Err(ValidationError::NotYourTurn);
        }
        Ok(())
    }

    fn require_conscious(&self, id: CombatantId) -> Result<(), ValidationError> {
        let actor = self
            .combatants
            .get(&id)
            .ok_or(ValidationError::CombatantNotFound)?;
        if actor.current_hp <= 0
            || actor.is_dead
            || actor.has_condition(super::conditions::Condition::Unconscious)
        {
            Err(ValidationError::Incapacitated)
        } else {
            Ok(())
        }
    }

    fn resolve_npc_turn(&mut self, id: CombatantId) -> Result<ResultEvents, ValidationError> {
        self.require_current(id)?;
        let actor = self
            .combatants
            .get(&id)
            .ok_or(ValidationError::CombatantNotFound)?;
        if actor.character_id.is_some() || actor.loadout.is_none() {
            return Err(ValidationError::WeaponUnavailable);
        }
        let mut events = ResultEvents::default();
        if actor.current_hp > 0
            && !actor.is_dead
            && !actor.has_condition(super::conditions::Condition::Unconscious)
        {
            let origin = actor.position;
            let target = self
                .combatants
                .values()
                .filter(|target| {
                    target.character_id.is_some() && target.current_hp > 0 && !target.is_dead
                })
                .min_by_key(|target| (origin.distance_ft(target.position), target.id.0.as_u128()))
                .map(|target| target.id);
            if let Some(target) = target {
                loop {
                    let actor = &self.combatants[&id];
                    let destination = self.combatants[&target].position;
                    if actor.position.distance_ft(destination) <= 5 || actor.budget.movement_ft < 5
                    {
                        break;
                    }
                    let current = actor.position;
                    let next = (-1..=1)
                        .flat_map(|x| (-1..=1).map(move |y| (x, y)))
                        .filter(|(x, y)| *x != 0 || *y != 0)
                        .map(|(x, y)| Position {
                            x: current.x.saturating_add(x),
                            y: current.y.saturating_add(y),
                        })
                        .filter(|next| {
                            next.distance_ft(destination) < current.distance_ft(destination)
                                && !self.combatants.values().any(|other| {
                                    other.id != id
                                        && other.current_hp > 0
                                        && other.position == *next
                                })
                        })
                        .min_by_key(|next| (next.distance_ft(destination), next.x, next.y));
                    let Some(next) = next else { break };
                    events.append(self.resolve_move(id, next)?);
                }
                if self.combatants[&id]
                    .position
                    .distance_ft(self.combatants[&target].position)
                    <= 5
                    && self.combatants[&id].budget.action
                {
                    events.append(self.resolve_weapon_attack(
                        id,
                        target,
                        None,
                        super::loadout::WeaponMode::Default,
                        false,
                    )?);
                }
            }
        }
        events.append(self.resolve_end_turn(id)?);
        Ok(events)
    }

    fn resolve_weapon_attack(
        &mut self,
        attacker: CombatantId,
        target: CombatantId,
        weapon_id: Option<String>,
        mode: super::loadout::WeaponMode,
        two_handed: bool,
    ) -> Result<ResultEvents, ValidationError> {
        use super::conditions::{attacker_condition_modifier, Condition};
        use super::loadout::WeaponMode;
        self.require_current(attacker)?;
        self.require_conscious(attacker)?;
        if attacker == target {
            return Err(ValidationError::InvalidTarget);
        }
        let actor = self
            .combatants
            .get(&attacker)
            .ok_or(ValidationError::AttackerNotFound)?;
        let victim = self
            .combatants
            .get(&target)
            .ok_or(ValidationError::TargetNotFound)?;
        if victim.current_hp <= 0
            || victim.is_dead
            || (actor.character_id.is_some() && victim.character_id.is_some())
        {
            return Err(ValidationError::InvalidTarget);
        }
        let loadout = actor
            .loadout
            .as_ref()
            .ok_or(ValidationError::WeaponUnavailable)?;
        let id = weapon_id.as_deref().unwrap_or(&loadout.default_weapon);
        let profile = loadout
            .attacks
            .iter()
            .find(|attack| attack.weapon_id == id)
            .ok_or(ValidationError::WeaponUnavailable)?
            .clone();
        if id != "unarmed" && loadout.quantities.get(id).copied().unwrap_or(0) == 0 {
            return Err(ValidationError::WeaponUnavailable);
        }
        let mode = if mode == WeaponMode::Default {
            if profile.ranged {
                WeaponMode::Ranged
            } else {
                WeaponMode::Melee
            }
        } else {
            mode
        };
        let ranged = matches!(mode, WeaponMode::Ranged | WeaponMode::Thrown);
        if ranged && two_handed {
            return Err(ValidationError::WeaponUnavailable);
        }
        if (mode == WeaponMode::Thrown && !profile.thrown)
            || (mode == WeaponMode::Ranged && !profile.ranged)
            || (mode == WeaponMode::Melee && profile.ranged)
        {
            return Err(ValidationError::WeaponUnavailable);
        }
        if loadout.shield && (profile.two_handed || two_handed) {
            return Err(ValidationError::HandsOccupied);
        }
        let distance = actor.position.distance_ft(victim.position);
        let maximum = if ranged {
            profile.long_range_ft
        } else {
            profile.reach_ft
        };
        if maximum <= 0 || distance > maximum {
            return Err(ValidationError::OutOfRange);
        }
        if profile.ammunition && loadout.arrows == 0 {
            return Err(ValidationError::AmmunitionExhausted);
        }
        let mut disadvantage =
            profile.heavy_disadvantage || (ranged && distance > profile.normal_range_ft);
        let mut advantage = false;
        for condition in &actor.conditions {
            if attacker_condition_modifier(*condition) == AttackModifier::Disadvantage {
                disadvantage = true;
            }
        }
        if victim.has_condition(Condition::Unconscious) {
            advantage = true;
        }
        if victim.has_condition(Condition::Prone) {
            if distance <= 5 {
                advantage = true;
            } else {
                disadvantage = true;
            }
        }
        if ranged
            && self.combatants.values().any(|enemy| {
                enemy.id != attacker
                    && enemy.character_id.is_some() != actor.character_id.is_some()
                    && enemy.current_hp > 0
                    && !enemy.is_dead
                    && !enemy.has_condition(Condition::Unconscious)
                    && !enemy.has_condition(Condition::Blinded)
                    && actor.position.distance_ft(enemy.position) <= 5
            })
        {
            disadvantage = true;
        }
        let modifier = match (advantage, disadvantage) {
            (true, false) => AttackModifier::Advantage,
            (false, true) => AttackModifier::Disadvantage,
            _ => AttackModifier::Normal,
        };
        let damage = if two_handed {
            profile
                .versatile_damage
                .clone()
                .ok_or(ValidationError::WeaponUnavailable)?
        } else {
            profile.damage.clone()
        };
        let events = self.resolve_attack(
            attacker,
            target,
            profile.attack_bonus,
            damage,
            profile.damage_type,
            modifier,
        )?;
        let loadout = self
            .combatants
            .get_mut(&attacker)
            .ok_or(ValidationError::AttackerNotFound)?
            .loadout
            .as_mut()
            .ok_or(ValidationError::WeaponUnavailable)?;
        if profile.ammunition {
            loadout.arrows -= 1;
        }
        if mode == WeaponMode::Thrown {
            if let Some(quantity) = loadout.quantities.get_mut(&profile.weapon_id) {
                *quantity -= 1;
            }
        }
        loadout.default_weapon = profile.weapon_id;
        Ok(events)
    }

    fn resolve_cast(&mut self, combatant: CombatantId) -> Result<ResultEvents, ValidationError> {
        self.require_current(combatant)?;
        self.require_conscious(combatant)?;
        let actor = self
            .combatants
            .get_mut(&combatant)
            .ok_or(ValidationError::CombatantNotFound)?;
        consume_action(&mut actor.budget, ActionKind::Action)
            .map_err(|error| ValidationError::ActionEconomy(error.to_string()))?;
        Ok(ResultEvents::default())
    }

    fn resolve_move(
        &mut self,
        combatant: CombatantId,
        to: Position,
    ) -> Result<ResultEvents, ValidationError> {
        self.require_current(combatant)?;
        self.require_conscious(combatant)?;
        let actor = self
            .combatants
            .get_mut(&combatant)
            .ok_or(ValidationError::CombatantNotFound)?;
        let from = actor.position;
        let feet_used = from.distance_ft(to);
        consume_movement_ft(&mut actor.budget, feet_used)
            .map_err(|error| ValidationError::ActionEconomy(error.to_string()))?;
        actor.position = to;
        let mut events = ResultEvents::default();
        events.movement.push(MovementEvent {
            combatant,
            from,
            to,
            feet_used,
        });
        Ok(events)
    }

    fn resolve_end_turn(
        &mut self,
        combatant: CombatantId,
    ) -> Result<ResultEvents, ValidationError> {
        self.require_current(combatant)?;
        self.order.advance();
        for _ in 0..self.order.len() {
            let next = self.order.current().id;
            if self
                .combatants
                .get(&next)
                .is_some_and(|actor| actor.current_hp > 0 && !actor.is_dead)
            {
                break;
            }
            self.order.advance();
        }
        let next = self.order.current().id;
        let actor = self
            .combatants
            .get_mut(&next)
            .ok_or(ValidationError::CombatantNotFound)?;
        actor.budget.refresh_for_new_turn(actor.speed_ft);
        Ok(ResultEvents::default())
    }

    fn resolve_attack(
        &mut self,
        attacker_id: CombatantId,
        target_id: CombatantId,
        attack_modifier: i32,
        damage_expr: DiceExpr,
        damage_type: DamageType,
        advantage: AttackModifier,
    ) -> Result<ResultEvents, ValidationError> {
        // --- Phase 1: Validate ---
        let current_id = self.order.current().id;
        if attacker_id != current_id {
            return Err(ValidationError::NotYourTurn);
        }
        self.require_conscious(attacker_id)?;
        if attacker_id == target_id {
            return Err(ValidationError::InvalidTarget);
        }
        if !self.combatants.contains_key(&target_id) {
            return Err(ValidationError::TargetNotFound);
        }
        if self.combatants[&target_id].current_hp <= 0 || self.combatants[&target_id].is_dead {
            return Err(ValidationError::InvalidTarget);
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
        let rolled = roll_attack_detailed(attack_modifier, target_ac, advantage, &mut self.rng);
        let attack_outcome = rolled.outcome;

        let mut events = ResultEvents::default();

        match &attack_outcome {
            AttackOutcome::CriticalHit { natural, total } => {
                // PHB crit: roll damage dice twice, add modifier once.
                let normal_detail = roll_expr_detailed(&damage_expr, &mut self.rng);
                let extra_detail = roll_expr_detailed(
                    &DiceExpr {
                        modifier: 0,
                        ..damage_expr.clone()
                    },
                    &mut self.rng,
                );
                let raw_damage =
                    (normal_detail.total + extra_detail.rolls.iter().sum::<i32>()).max(0);
                events.rolls.push(DiceRoll {
                    purpose: format!("crit attack roll vs AC {target_ac}"),
                    detail: crate::dice::RollDetail {
                        rolls: rolled.rolls.clone(),
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
                        rolls: rolled.rolls.clone(),
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
                        rolls: rolled.rolls.clone(),
                        modifier: attack_modifier,
                        total: *total,
                    },
                    natural: *natural,
                });
                let raw_damage = dmg_detail.total.max(0);
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
                        rolls: rolled.rolls.clone(),
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
