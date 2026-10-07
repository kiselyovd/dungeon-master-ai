//! Server-owned vitality stored with the authoritative character document.
use app_domain::combat::{
    combatant::{Combatant, DeathSaves},
    conditions::Condition,
    healing::NaturalRecovery,
    types::CombatantId,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeroVitality {
    pub character_id: Uuid,
    pub combatant_id: CombatantId,
    pub death_saves: DeathSaves,
    pub conditions: Vec<Condition>,
    pub is_dead: bool,
    pub natural_recovery: Option<NaturalRecovery>,
}

pub fn read(sheet: &Value, character_id: Uuid) -> Result<Option<HeroVitality>, &'static str> {
    let Some(raw) = sheet.get("combatResources").and_then(|r| r.get("vitality")) else {
        return Ok(None);
    };
    let state: HeroVitality =
        serde_json::from_value(raw.clone()).map_err(|_| "hero_vitality_invalid")?;
    let hp = sheet["hp"].as_i64().ok_or("hero_vitality_invalid")?;
    let max_hp = sheet["hpMax"].as_i64().ok_or("hero_vitality_invalid")?;
    if max_hp <= 0 || max_hp > i32::MAX as i64 || hp < 0 || hp > max_hp {
        return Err("hero_vitality_invalid");
    }
    if sheet
        .get("characterId")
        .is_some_and(|id| id.as_str().and_then(|id| Uuid::parse_str(id).ok()) != Some(character_id))
    {
        return Err("hero_vitality_identity_mismatch");
    }
    if character_id.is_nil()
        || state.character_id != character_id
        || state.combatant_id.0.is_nil()
        || state.death_saves.successes > 2
        || state.death_saves.failures > 3
        || (state.death_saves.failures == 3 && !state.is_dead)
        || (state.is_dead && (hp != 0 || state.death_saves.stable))
        || (state.death_saves.stable
            && (hp != 0 || state.death_saves.successes != 0 || state.death_saves.failures != 0))
        || (hp > 0 && (state.death_saves.successes != 0 || state.death_saves.failures != 0))
        || state
            .conditions
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>()
            .len()
            != state.conditions.len()
    {
        return Err("hero_vitality_invalid");
    }
    if let Some(deadline) = &state.natural_recovery {
        let duration = deadline
            .ready_at_seconds
            .checked_sub(deadline.started_at_seconds)
            .ok_or("hero_vitality_invalid")?;
        if !state.death_saves.stable
            || state.is_dead
            || hp != 0
            || deadline.combatant != state.combatant_id
            || ![3600, 7200, 10800, 14400].contains(&duration)
            || deadline.ready_at_seconds > i64::MAX as u64
        {
            return Err("hero_vitality_invalid");
        }
    }
    Ok(Some(state))
}

pub fn restore_actor(
    sheet: &Value,
    character_id: Uuid,
    actor: &mut Combatant,
) -> Result<(), &'static str> {
    if actor.character_id.map(|id| id.0) != Some(character_id) {
        return Err("hero_vitality_identity_mismatch");
    }
    if let Some(state) = read(sheet, character_id)? {
        actor.death_saves = state.death_saves;
        actor.is_dead = state.is_dead;
        actor.conditions = state.conditions.into_iter().collect();
    }
    if actor.current_hp == 0 {
        actor.add_condition(Condition::Unconscious);
    }
    Ok(())
}

fn write(sheet: &mut Value, state: &HeroVitality) -> Result<(), &'static str> {
    if !sheet.get("combatResources").is_some_and(Value::is_object) {
        sheet["combatResources"] = serde_json::json!({});
    }
    sheet["combatResources"]["vitality"] =
        serde_json::to_value(state).map_err(|_| "hero_vitality_invalid")?;
    Ok(())
}

/// Reads the previous state before updating HP so corrupt state cannot be repaired silently.
pub fn store_actor(sheet: &mut Value, actor: &Combatant) -> Result<(), &'static str> {
    let character_id = actor
        .character_id
        .ok_or("hero_vitality_identity_mismatch")?
        .0;
    let previous = read(sheet, character_id)?;
    let mut natural_recovery = previous.and_then(|old| old.natural_recovery);
    if actor.current_hp > 0 || actor.is_dead || !actor.death_saves.stable {
        natural_recovery = None;
    }
    if let Some(deadline) = &mut natural_recovery {
        deadline.combatant = actor.id;
    }
    let mut conditions: Vec<_> = actor.conditions.iter().copied().collect();
    if actor.current_hp == 0 && !conditions.contains(&Condition::Unconscious) {
        conditions.push(Condition::Unconscious);
    }
    conditions.sort_by_key(ToString::to_string);
    let state = HeroVitality {
        character_id,
        combatant_id: actor.id,
        death_saves: actor.death_saves.clone(),
        conditions,
        is_dead: actor.is_dead,
        natural_recovery,
    };
    let mut candidate = sheet.clone();
    candidate["hp"] = serde_json::json!(actor.current_hp);
    candidate["characterId"] = serde_json::json!(character_id);
    write(&mut candidate, &state)?;
    read(&candidate, character_id)?;
    *sheet = candidate;
    Ok(())
}

pub fn clear_after_healing(sheet: &mut Value, hp_after: i32) -> Result<(), &'static str> {
    let Some(_raw) = sheet.get("combatResources").and_then(|r| r.get("vitality")) else {
        return Ok(());
    };
    let character_id = sheet
        .get("characterId")
        .and_then(Value::as_str)
        .and_then(|id| Uuid::parse_str(id).ok())
        .ok_or("hero_vitality_identity_mismatch")?;
    let mut state = read(sheet, character_id)?.ok_or("hero_vitality_invalid")?;
    if i64::from(hp_after) > sheet["hp"].as_i64().ok_or("hero_vitality_invalid")? {
        if state.is_dead {
            return Err("hero_dead");
        }
        state.death_saves = DeathSaves::default();
        state.natural_recovery = None;
        state.conditions.retain(|c| *c != Condition::Unconscious);
        write(sheet, &state)?;
    }
    Ok(())
}

/// Called by an owned application intention after the engine confirms stabilization.
pub fn schedule_natural_recovery(
    sheet: &mut Value,
    character_id: Uuid,
    deadline: NaturalRecovery,
) -> Result<(), &'static str> {
    let mut state = read(sheet, character_id)?.ok_or("hero_vitality_invalid")?;
    if let Some(existing) = &state.natural_recovery {
        return if existing == &deadline {
            Ok(())
        } else {
            Err("hero_recovery_already_scheduled")
        };
    }
    state.natural_recovery = Some(deadline);
    let mut candidate = sheet.clone();
    write(&mut candidate, &state)?;
    read(&candidate, character_id)?;
    *sheet = candidate;
    Ok(())
}

/// Incoming player drafts cannot establish server-owned recovery resources.
pub fn strip_client_state(sheet: &mut Value) {
    if let Some(resources) = sheet
        .get_mut("combatResources")
        .and_then(Value::as_object_mut)
    {
        resources.remove("vitality");
    }
}
