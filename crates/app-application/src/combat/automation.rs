use crate::models::combat::CombatProjection;
use app_domain::combat::{
    resolver::{CombatAction, CombatResolver},
    result_events::ResultEvents,
};

pub fn settle_npcs(
    engine: &mut CombatResolver,
    events: &mut Vec<ResultEvents>,
) -> Result<(), &'static str> {
    settle_npcs_until(engine, events, None)
}

pub fn settle_npcs_until(
    engine: &mut CombatResolver,
    events: &mut Vec<ResultEvents>,
    stop_round: Option<u32>,
) -> Result<(), &'static str> {
    if !engine
        .combatants
        .values()
        .any(|actor| actor.character_id.is_some())
    {
        return Ok(());
    }
    for _ in 0..=engine.order.len() {
        if stop_round.is_some_and(|round| engine.order.round() >= round) {
            return Ok(());
        }
        let heroes = engine
            .combatants
            .values()
            .any(|actor| actor.character_id.is_some() && actor.current_hp > 0 && !actor.is_dead);
        let enemies = engine
            .combatants
            .values()
            .any(|actor| actor.character_id.is_none() && actor.current_hp > 0 && !actor.is_dead);
        if !heroes || !enemies {
            return Ok(());
        }
        let current = engine.order.current().id;
        if engine.combatants[&current].character_id.is_some() {
            return Ok(());
        }
        events.push(
            engine
                .resolve(CombatAction::NpcTurn { combatant: current })
                .map_err(|_| "combat_npc_action_rejected")?,
        );
    }
    Err("combat_npc_cycle_invalid")
}

pub fn finish_if_resolved(projection: &mut CombatProjection) {
    let bound = projection
        .snapshot
        .combatants
        .iter()
        .any(|actor| actor.character_id.is_some());
    if !bound {
        return;
    }
    let heroes = projection
        .snapshot
        .combatants
        .iter()
        .any(|actor| actor.character_id.is_some() && actor.current_hp > 0 && !actor.is_dead);
    let enemies = projection
        .snapshot
        .combatants
        .iter()
        .any(|actor| actor.character_id.is_none() && actor.current_hp > 0 && !actor.is_dead);
    if !heroes || !enemies {
        projection.snapshot.active = false;
        projection.snapshot.current_combatant = None;
    }
}
