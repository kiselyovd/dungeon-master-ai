use super::{combatant::Combatant, result_events::DeathSaveResult};
use crate::dice::{roll_one, Die};
use crate::rng::SeededRng;
use serde::{Deserialize, Serialize};

/// Healing at zero HP revives a living combatant and clears death saves.
pub fn apply_healing(combatant: &mut Combatant, amount: i32) {
    combatant.apply_healing(amount);
}

fn dying(combatant: &Combatant) -> bool {
    combatant.current_hp == 0
        && combatant.max_hp > 0
        && !combatant.is_dead
        && !combatant.death_saves.stable
        && combatant.death_saves.failures < 3
        && combatant.death_saves.successes < 3
}

/// No die is consumed for a conscious, stable or dead creature.
pub fn resolve_death_save(
    combatant: &mut Combatant,
    rng: &mut SeededRng,
) -> Option<DeathSaveResult> {
    if !dying(combatant) {
        return None;
    }
    let roll = roll_one(Die::D20, rng);
    match roll {
        20 => combatant.apply_healing(1),
        10..=19 => {
            combatant.death_saves.record_success();
        }
        _ => {
            let failures = if roll == 1 { 2 } else { 1 };
            for _ in 0..failures {
                if combatant.death_saves.record_failure() {
                    combatant.is_dead = true;
                }
            }
        }
    }
    Some(DeathSaveResult {
        combatant: combatant.id,
        roll,
        success: roll >= 10,
        successes_total: combatant.death_saves.successes,
        failures_total: combatant.death_saves.failures,
        dead: combatant.is_dead,
        stable: combatant.death_saves.stable,
    })
}

/// Compatibility wrapper. Ineligible creatures return false without rolling.
pub fn roll_death_save(combatant: &mut Combatant, rng: &mut SeededRng) -> bool {
    resolve_death_save(combatant, rng).is_some_and(|result| result.success)
}

/// The authoritative helper owns the Wisdom (Medicine) roll; DC is ten.
pub fn stabilize_with_check(combatant: &mut Combatant, medicine_total: i32) -> bool {
    if !dying(combatant) || medicine_total < 10 {
        return false;
    }
    combatant.death_saves.stabilize();
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NaturalRecovery {
    pub combatant: super::types::CombatantId,
    pub started_at_seconds: u64,
    pub ready_at_seconds: u64,
}
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RecoveryError {
    #[error("natural recovery requires a living stable creature at zero hit points")]
    Ineligible,
    #[error("natural recovery deadline is invalid")]
    InvalidDeadline,
}

pub fn begin_natural_recovery(
    combatant: &Combatant,
    now: u64,
    rng: &mut SeededRng,
) -> Result<NaturalRecovery, RecoveryError> {
    if combatant.current_hp != 0
        || combatant.max_hp <= 0
        || combatant.is_dead
        || !combatant.death_saves.stable
    {
        return Err(RecoveryError::Ineligible);
    }
    // Reject invalid clocks before consuming randomness.
    if now > (i64::MAX as u64).saturating_sub(4 * 3600) {
        return Err(RecoveryError::InvalidDeadline);
    }
    let hours = roll_one(Die::D4, rng) as u64;
    Ok(NaturalRecovery {
        combatant: combatant.id,
        started_at_seconds: now,
        ready_at_seconds: now + hours * 3600,
    })
}

pub fn complete_natural_recovery(
    combatant: &mut Combatant,
    deadline: &NaturalRecovery,
    now: u64,
) -> Result<bool, RecoveryError> {
    let duration = deadline
        .ready_at_seconds
        .checked_sub(deadline.started_at_seconds)
        .ok_or(RecoveryError::InvalidDeadline)?;
    if ![3600, 7200, 10800, 14400].contains(&duration)
        || deadline.ready_at_seconds > i64::MAX as u64
    {
        return Err(RecoveryError::InvalidDeadline);
    }
    if deadline.combatant != combatant.id {
        return Err(RecoveryError::InvalidDeadline);
    }
    if now < deadline.ready_at_seconds
        || combatant.current_hp != 0
        || combatant.max_hp <= 0
        || combatant.is_dead
        || !combatant.death_saves.stable
    {
        return Ok(false);
    }
    combatant.apply_healing(1);
    Ok(true)
}
