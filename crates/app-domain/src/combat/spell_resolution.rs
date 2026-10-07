use super::*;
use crate::combat::{
    conditions::{attacker_condition_modifier, Condition},
    magic::{cantrip_scale, spell_rule, SpellEffect},
    result_events::{HealingApplication, SpellSlotConsumption},
    types::SlotLevel,
};
use crate::dice::Die;

impl CombatResolver {
    pub(super) fn resolve_spell(
        &mut self,
        caster: CombatantId,
        spell_id: &str,
        targets: Vec<CombatantId>,
        slot_level: u8,
    ) -> Result<ResultEvents, ValidationError> {
        self.require_current(caster)?;
        self.require_conscious(caster)?;
        let actor = &self.combatants[&caster];
        let casting = actor
            .casting
            .clone()
            .ok_or(ValidationError::SpellUnavailable)?;
        if !casting.prepared.iter().any(|id| id == spell_id) {
            return Err(ValidationError::SpellUnavailable);
        }
        let rule = spell_rule(spell_id).ok_or(ValidationError::SpellUnavailable)?;
        if rule.level == 0 {
            if slot_level != 0 {
                return Err(ValidationError::SpellSlotUnavailable);
            }
        } else if slot_level < rule.level
            || slot_level > 9
            || actor.budget.spell_slot_used
            || !casting
                .slots
                .get(&slot_level)
                .is_some_and(|slots| slots.remaining > 0)
        {
            return Err(ValidationError::SpellSlotUnavailable);
        }
        let count = if spell_id == "eldritch-blast" {
            cantrip_scale(casting.level) as usize
        } else if matches!(rule.effect, SpellEffect::Missiles) {
            usize::from(slot_level) + 2
        } else {
            1
        };
        if targets.is_empty() || (targets.len() != 1 && targets.len() != count) {
            return Err(ValidationError::InvalidTarget);
        }
        for target in &targets {
            let victim = self
                .combatants
                .get(target)
                .ok_or(ValidationError::TargetNotFound)?;
            if !self.clear_line(actor.position, victim.position) {
                return Err(ValidationError::Obstructed);
            }
            if victim.is_dead
                || (!matches!(rule.effect, SpellEffect::Heal(_))
                    && (victim.current_hp <= 0
                        || victim.character_id.is_some() && actor.character_id.is_some()))
            {
                return Err(ValidationError::InvalidTarget);
            }
            if actor.position.distance_ft(victim.position) > rule.range_ft {
                return Err(ValidationError::OutOfRange);
            }
            if actor.has_condition(Condition::Blinded)
                && matches!(spell_id, "healing-word" | "magic-missile" | "sacred-flame")
            {
                return Err(ValidationError::InvalidTarget);
            }
        }
        let mut budget = actor.budget.clone();
        consume_action(
            &mut budget,
            if rule.bonus_action {
                ActionKind::BonusAction
            } else {
                ActionKind::Action
            },
        )
        .map_err(|e| ValidationError::ActionEconomy(e.to_string()))?;
        if rule.level > 0 {
            budget.spell_slot_used = true;
        }
        self.combatants.get_mut(&caster).unwrap().budget = budget;
        let mut events = ResultEvents::default();
        match rule.effect {
            SpellEffect::Heal(die) => {
                let detail = roll_expr_detailed(
                    &DiceExpr {
                        count: 2 * slot_level,
                        die,
                        modifier: casting.ability_modifier,
                    },
                    &mut self.rng,
                );
                let amount = detail.total.max(0);
                let target = targets[0];
                let victim = self.combatants.get_mut(&target).unwrap();
                let before = victim.current_hp;
                victim.apply_healing(amount);
                events.healing.push(HealingApplication {
                    target,
                    amount: victim.current_hp - before,
                    new_hp: victim.current_hp,
                });
                events.rolls.push(DiceRoll {
                    purpose: spell_id.into(),
                    natural: 0,
                    detail,
                });
            }
            SpellEffect::Missiles => {
                let detail = roll_expr_detailed(
                    &DiceExpr {
                        count: 1,
                        die: Die::D4,
                        modifier: 1,
                    },
                    &mut self.rng,
                );
                let mut damage = std::collections::BTreeMap::new();
                for index in 0..count {
                    let target = targets[if targets.len() == 1 { 0 } else { index }];
                    *damage.entry(target.0).or_insert(0) += detail.total;
                }
                for (target, amount) in damage {
                    self.spell_damage(
                        CombatantId(target),
                        amount,
                        DamageType::Force,
                        false,
                        &mut events,
                    );
                }
                events.rolls.push(DiceRoll {
                    purpose: spell_id.into(),
                    natural: 0,
                    detail,
                });
            }
            SpellEffect::DexteritySave(die, damage_type) => {
                let target = targets[0];
                let automatic = self.combatants[&target].has_condition(Condition::Unconscious);
                let detail = if automatic {
                    crate::dice::RollDetail {
                        rolls: vec![],
                        modifier: 0,
                        total: 0,
                    }
                } else {
                    roll_expr_detailed(
                        &DiceExpr {
                            count: 1,
                            die: Die::D20,
                            modifier: self.combatants[&target].dex_mod,
                        },
                        &mut self.rng,
                    )
                };
                let success = !automatic && detail.total >= casting.save_dc;
                events.rolls.push(DiceRoll {
                    purpose: if automatic {
                        format!("{spell_id} automatic save failure")
                    } else {
                        format!("{spell_id} save vs DC {}", casting.save_dc)
                    },
                    natural: detail.rolls.first().copied().unwrap_or(0),
                    detail,
                });
                if !success {
                    let detail = roll_expr_detailed(
                        &DiceExpr {
                            count: cantrip_scale(casting.level),
                            die,
                            modifier: 0,
                        },
                        &mut self.rng,
                    );
                    self.spell_damage(target, detail.total, damage_type, false, &mut events);
                    events.rolls.push(DiceRoll {
                        purpose: spell_id.into(),
                        natural: 0,
                        detail,
                    });
                }
            }
            SpellEffect::Attack(die, damage_type) => {
                for index in 0..count {
                    let target = targets[if targets.len() == 1 { 0 } else { index }];
                    let actor = &self.combatants[&caster];
                    let victim = &self.combatants[&target];
                    let advantage = victim.has_condition(Condition::Unconscious)
                        || victim.has_condition(Condition::Blinded)
                        || victim.has_condition(Condition::Prone)
                            && actor.position.distance_ft(victim.position) <= 5;
                    let disadvantage =
                        actor.conditions.iter().any(|c| {
                            attacker_condition_modifier(*c) == AttackModifier::Disadvantage
                        }) || victim.has_condition(Condition::Prone)
                            && actor.position.distance_ft(victim.position) > 5
                            || self.combatants.values().any(|enemy| {
                                enemy.character_id.is_some() != actor.character_id.is_some()
                                    && enemy.current_hp > 0
                                    && !enemy.is_dead
                                    && !enemy.has_condition(Condition::Blinded)
                                    && !enemy.has_condition(Condition::Unconscious)
                                    && actor.position.distance_ft(enemy.position) <= 5
                            });
                    let modifier = match (advantage, disadvantage) {
                        (true, false) => AttackModifier::Advantage,
                        (false, true) => AttackModifier::Disadvantage,
                        _ => AttackModifier::Normal,
                    };
                    let unconscious_critical = victim.has_condition(Condition::Unconscious)
                        && actor.position.distance_ft(victim.position) <= 5;
                    let rolled = roll_attack_detailed(
                        casting.attack_bonus,
                        victim.ac,
                        modifier,
                        &mut self.rng,
                    );
                    let (hit, critical, natural, total) = match rolled.outcome {
                        AttackOutcome::CriticalHit { natural, total } => {
                            (true, true, natural, total)
                        }
                        AttackOutcome::Hit { natural, total } => (true, false, natural, total),
                        AttackOutcome::CriticalMiss { natural } => {
                            (false, false, natural, natural + casting.attack_bonus)
                        }
                        AttackOutcome::Miss { natural, total } => (false, false, natural, total),
                    };
                    let critical = critical || hit && unconscious_critical;
                    events.rolls.push(DiceRoll {
                        purpose: format!("{spell_id} attack"),
                        natural,
                        detail: crate::dice::RollDetail {
                            rolls: rolled.rolls,
                            modifier: casting.attack_bonus,
                            total,
                        },
                    });
                    if hit {
                        let dice = if spell_id == "eldritch-blast" {
                            1
                        } else {
                            cantrip_scale(casting.level)
                        };
                        let detail = roll_expr_detailed(
                            &DiceExpr {
                                count: dice * if critical { 2 } else { 1 },
                                die,
                                modifier: 0,
                            },
                            &mut self.rng,
                        );
                        self.spell_damage(target, detail.total, damage_type, critical, &mut events);
                        events.rolls.push(DiceRoll {
                            purpose: spell_id.into(),
                            natural: 0,
                            detail,
                        });
                    }
                }
            }
        }
        if rule.level > 0 {
            let actor = self.combatants.get_mut(&caster).unwrap();
            actor
                .casting
                .as_mut()
                .unwrap()
                .slots
                .get_mut(&slot_level)
                .unwrap()
                .remaining -= 1;
            if let Some(character) = actor.character_id {
                events.spell_slots_consumed.push(SpellSlotConsumption {
                    character,
                    level: SlotLevel(slot_level),
                });
            }
        }
        Ok(events)
    }

    fn spell_damage(
        &mut self,
        target: CombatantId,
        amount: i32,
        damage_type: DamageType,
        critical: bool,
        events: &mut ResultEvents,
    ) {
        let victim = self.combatants.get_mut(&target).unwrap();
        let effective = apply_damage_to_combatant(
            victim,
            amount.max(0),
            damage_type,
            &DamageResistance::default(),
        );
        events.damage.push(DamageApplication {
            target,
            raw_amount: amount.max(0),
            effective_amount: effective,
            damage_type,
            new_hp: victim.current_hp,
            was_critical: critical,
        });
        if victim.current_hp == 0 {
            victim.add_condition(Condition::Unconscious);
        }
    }
}
