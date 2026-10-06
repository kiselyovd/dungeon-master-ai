use crate::models::campaign_state::valid_character_document;
use app_domain::combat::combatant::{ActionBudget, Combatant};
use app_domain::combat::types::{ability_modifier, CharacterId, CombatantId, Position};
use serde_json::Value;
use uuid::Uuid;

/// Accepts an already committed durable sheet, never a model's hero entry.
/// Equipment/action derivation is a separate domain operation.
pub fn hero_combatant(character_id: Uuid, sheet: &Value) -> Result<Combatant, &'static str> {
    if character_id.is_nil() || !valid_character_document(sheet) {
        return Err("combat_hero_invalid");
    }
    let speed = match sheet.get("speedFt") {
        None => 30,
        Some(value) => value
            .as_i64()
            .filter(|speed| (1..=120).contains(speed))
            .ok_or("combat_hero_speed_invalid")? as i32,
    };
    let mut actor = Combatant::new(
        CombatantId::new(),
        sheet["name"].as_str().ok_or("combat_hero_invalid")?.into(),
        sheet["hpMax"].as_i64().ok_or("combat_hero_invalid")? as i32,
        sheet["hp"].as_i64().ok_or("combat_hero_invalid")? as i32,
        sheet["ac"].as_i64().ok_or("combat_hero_invalid")? as i32,
    );
    actor.character_id = Some(CharacterId(character_id));
    actor.dex_mod = ability_modifier(
        sheet["abilities"]["dex"]
            .as_i64()
            .ok_or("combat_hero_invalid")? as i32,
    );
    actor.speed_ft = speed;
    actor.budget = ActionBudget::fresh(speed);
    actor.position = Position { x: 8, y: 8 };
    Ok(actor)
}

pub fn bind_hero(
    character_id: Uuid,
    sheet: &Value,
    projection: &mut crate::models::combat::CombatProjection,
    seed: u64,
) -> Result<(), &'static str> {
    use app_domain::combat::initiative::{InitiativeEntry, InitiativeOrder};
    use app_domain::{
        dice::{roll_one, Die},
        rng::SeededRng,
    };
    let mut hero = hero_combatant(character_id, sheet)?;
    hero.initiative_roll = roll_one(Die::D20, &mut SeededRng::from_seed(seed)) + hero.dex_mod;
    let mut npcs = projection
        .snapshot
        .combatants
        .iter()
        .filter(|actor| actor.id.0 != character_id)
        .cloned()
        .collect::<Vec<_>>();
    for (index, npc) in npcs.iter_mut().enumerate() {
        npc.character_id = None;
        npc.position = Position {
            x: 9 + index as i32,
            y: 8,
        };
        if let Some(entry) = projection
            .snapshot
            .initiative
            .iter()
            .find(|entry| entry.id == npc.id)
        {
            npc.initiative_roll = entry.roll;
            npc.dex_mod = entry.dex_tiebreak;
        }
    }
    npcs.insert(0, hero);
    let order = InitiativeOrder::build(
        npcs.iter()
            .map(|actor| InitiativeEntry {
                id: actor.id,
                roll: actor.initiative_roll,
                dex_tiebreak: actor.dex_mod,
            })
            .collect(),
    );
    projection.snapshot.active = true;
    projection.snapshot.round = 1;
    projection.snapshot.current_combatant = Some(order.current().id);
    projection.snapshot.initiative = order.as_slice().to_vec();
    projection.snapshot.combatants = npcs;
    Ok(())
}
