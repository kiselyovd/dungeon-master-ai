//! Deterministic recovery decisions. Adapters own transactions, never dice policy.
use crate::{
    models::{
        combat::Combatant,
        room_recovery::{MedicineCheck, RecoveryDeathSave},
    },
    vitality,
};
use app_domain::{
    combat::{
        conditions::Condition,
        healing::{
            begin_natural_recovery, complete_natural_recovery, resolve_death_save,
            stabilize_with_check, NaturalRecovery,
        },
        types::ability_modifier,
    },
    dice::{roll_one, Die},
    rng::SeededRng,
};
use serde_json::Value;
use uuid::Uuid;

pub fn actor(sheet: &Value, id: Uuid) -> Result<Combatant, &'static str> {
    let mut actor = crate::combat::hero::hero_combatant(id, sheet)?;
    if let Some(state) = vitality::read(sheet, id)? {
        actor.id = state.combatant_id;
    }
    Ok(actor)
}
pub fn safe(state: &crate::models::campaign_state::CampaignSessionState) -> bool {
    state
        .projection
        .adventure
        .as_ref()
        .is_none_or(|a| a.state.location == app_domain::adventure::Location::Town)
}
pub struct RecoveryEngine {
    rng: SeededRng,
}
impl RecoveryEngine {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: SeededRng::from_seed(seed),
        }
    }
    pub fn random() -> Self {
        Self {
            rng: SeededRng::new_random(),
        }
    }
    pub fn medicine(
        &mut self,
        helper: &Combatant,
        sheet: &Value,
    ) -> Result<MedicineCheck, &'static str> {
        let level = sheet["level"]
            .as_u64()
            .filter(|n| (1..=20).contains(n))
            .ok_or("hero_aid_invalid")?;
        let wis = sheet["abilities"]["wis"]
            .as_i64()
            .ok_or("hero_aid_invalid")? as i32;
        let proficient = sheet
            .get("skillProfs")
            .and_then(|s| s.get("medicine"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let modifier = ability_modifier(wis)
            + if proficient {
                2 + ((level - 1) / 4) as i32
            } else {
                0
            };
        let disadvantage = helper.has_condition(Condition::Poisoned);
        let mut rolls = vec![roll_one(Die::D20, &mut self.rng)];
        if disadvantage {
            rolls.push(roll_one(Die::D20, &mut self.rng));
        }
        let total = *rolls.iter().min().ok_or("hero_aid_invalid")? + modifier;
        Ok(MedicineCheck {
            rolls,
            modifier,
            total,
            disadvantage,
        })
    }
    pub fn natural_deadline(
        &mut self,
        target: &Combatant,
        now: u64,
    ) -> Result<NaturalRecovery, &'static str> {
        begin_natural_recovery(target, now, &mut self.rng).map_err(|_| "hero_recovery_invalid")
    }
    pub fn stabilize(
        &mut self,
        target: &mut Combatant,
        total: i32,
        now: u64,
    ) -> Result<Option<NaturalRecovery>, &'static str> {
        if !stabilize_with_check(target, total) {
            return Ok(None);
        }
        begin_natural_recovery(target, now, &mut self.rng)
            .map(Some)
            .map_err(|_| "hero_recovery_invalid")
    }
    pub fn passage(
        &mut self,
        sheet: &mut Value,
        id: Uuid,
        previous: u64,
        now: u64,
    ) -> Result<Vec<RecoveryDeathSave>, &'static str> {
        if now < previous {
            return Err("adventuring_clock_invalid");
        }
        let old = vitality::read(sheet, id)?;
        if sheet["hp"].as_i64() != Some(0) || old.as_ref().is_some_and(|s| s.is_dead) {
            return Ok(vec![]);
        }
        let mut actor = actor(sheet, id)?;
        let mut deadline = old.and_then(|s| s.natural_recovery);
        let mut stable_at = previous;
        let mut events = vec![];
        for tick in 0..((now - previous) / 6).min(6) {
            let Some(result) = resolve_death_save(&mut actor, &mut self.rng) else {
                break;
            };
            if result.stable {
                stable_at = previous + (tick + 1) * 6;
            }
            events.push(RecoveryDeathSave {
                character_id: id,
                result,
            });
        }
        if actor.current_hp == 0 && actor.death_saves.stable && !actor.is_dead && deadline.is_none()
        {
            deadline = Some(
                begin_natural_recovery(&actor, stable_at, &mut self.rng)
                    .map_err(|_| "hero_recovery_invalid")?,
            );
        }
        if let Some(timer) = &deadline {
            complete_natural_recovery(&mut actor, timer, now)
                .map_err(|_| "hero_recovery_invalid")?;
        }
        vitality::store_actor(sheet, &actor)?;
        if actor.current_hp == 0 && actor.death_saves.stable && !actor.is_dead {
            if let Some(timer) = deadline {
                vitality::schedule_natural_recovery(sheet, id, timer)?;
            }
        }
        Ok(events)
    }
}

pub fn summaries(
    party: &crate::models::party::PartyState,
    room: &crate::models::room::RoomRecord,
    owner: Option<Uuid>,
    combat_active: bool,
    safe: bool,
) -> Result<Vec<crate::models::room_recovery::PublicHeroRecovery>, &'static str> {
    use crate::models::room_recovery::{PublicHeroRecovery, RecoveryIntent, RecoveryStatus};
    let conscious = if let Some(id) = owner {
        let sheet = party
            .characters
            .get(&id)
            .ok_or("hero_vitality_identity_mismatch")?;
        let state = vitality::read(sheet, id)?;
        sheet["hp"].as_i64().is_some_and(|hp| hp > 0)
            && state.is_none_or(|s| !s.is_dead && !s.conditions.contains(&Condition::Unconscious))
    } else {
        false
    };
    let mut result = vec![];
    for member in &party.roster.members {
        if !room
            .members
            .iter()
            .any(|m| m.member_id == member.member_id && m.active && m.role == member.role)
        {
            continue;
        }
        let id = member.character_id;
        let sheet = party
            .characters
            .get(&id)
            .ok_or("hero_vitality_identity_mismatch")?;
        let state = vitality::read(sheet, id)?;
        let hp = sheet["hp"].as_i64().ok_or("hero_vitality_invalid")?;
        let dead = state.as_ref().is_some_and(|s| s.is_dead);
        let stable = state.as_ref().is_some_and(|s| s.death_saves.stable);
        let deadline = state.as_ref().and_then(|s| s.natural_recovery.as_ref());
        let mut actions = vec![];
        let status = if dead {
            RecoveryStatus::Dead
        } else if hp > 0 {
            RecoveryStatus::Healthy
        } else if stable {
            RecoveryStatus::Stable
        } else {
            RecoveryStatus::Dying
        };
        if !combat_active && hp == 0 && !dead {
            if stable && safe && (conscious || owner == Some(id)) {
                actions.push(RecoveryIntent::Wait {
                    target_character_id: id,
                });
            } else if !stable {
                if conscious && owner != Some(id) {
                    actions.push(RecoveryIntent::Aid {
                        target_character_id: id,
                    });
                }
                if owner == Some(id) {
                    actions.push(RecoveryIntent::DeathSave {});
                }
            }
        }
        result.push(PublicHeroRecovery {
            character_id: id,
            status,
            successes: state.as_ref().map_or(0, |s| s.death_saves.successes),
            failures: state.as_ref().map_or(0, |s| s.death_saves.failures),
            ready_at_seconds: deadline.map(|d| d.ready_at_seconds),
            actions,
        });
    }
    Ok(result)
}
