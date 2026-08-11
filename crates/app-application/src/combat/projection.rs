use std::collections::HashMap;

use app_domain::combat::combatant::Combatant;
use app_domain::combat::initiative::InitiativeOrder;
use app_domain::combat::types::CombatantId;

use crate::models::combat::{CombatProjection, CombatSnapshot};

pub fn resolver_state(
    snapshot: &CombatSnapshot,
) -> (HashMap<CombatantId, Combatant>, InitiativeOrder) {
    let combatants = snapshot
        .combatants
        .iter()
        .cloned()
        .map(|combatant| (combatant.id, combatant))
        .collect();
    let mut order = InitiativeOrder::build(snapshot.initiative.clone());
    if let Some(current) = snapshot.current_combatant {
        for _ in 0..order.len() {
            if order.current().id == current {
                break;
            }
            order.advance();
        }
    }
    (combatants, order)
}

pub fn replace_resolved_state(
    projection: &mut CombatProjection,
    combatants: HashMap<CombatantId, Combatant>,
    events: app_domain::combat::result_events::ResultEvents,
) {
    projection.revision += 1;
    projection.snapshot.combatants = projection
        .snapshot
        .initiative
        .iter()
        .filter_map(|entry| combatants.get(&entry.id).cloned())
        .collect();
    projection.events = vec![events];
}
