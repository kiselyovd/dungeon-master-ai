use std::collections::HashMap;

use app_domain::combat::combatant::Combatant;
use app_domain::combat::initiative::InitiativeOrder;
use app_domain::combat::types::CombatantId;

use crate::models::combat::{CombatProjection, CombatSnapshot};

pub fn resolver_state(
    snapshot: &CombatSnapshot,
) -> Option<(HashMap<CombatantId, Combatant>, InitiativeOrder)> {
    let combatants = snapshot
        .combatants
        .iter()
        .cloned()
        .map(|combatant| (combatant.id, combatant))
        .collect();
    let order = InitiativeOrder::restore(
        snapshot.initiative.clone(),
        snapshot.current_combatant?,
        snapshot.round,
    )?;
    Some((combatants, order))
}

pub fn replace_resolved_state(
    projection: &mut CombatProjection,
    combatants: HashMap<CombatantId, Combatant>,
    order: InitiativeOrder,
    events: app_domain::combat::result_events::ResultEvents,
) {
    projection.revision += 1;
    projection.snapshot.combatants = projection
        .snapshot
        .initiative
        .iter()
        .filter_map(|entry| combatants.get(&entry.id).cloned())
        .collect();
    projection.snapshot.round = order.round();
    projection.snapshot.current_combatant = (!order.is_empty()).then(|| order.current().id);
    projection.snapshot.initiative = order.as_slice().to_vec();
    projection.events = vec![events];
}
