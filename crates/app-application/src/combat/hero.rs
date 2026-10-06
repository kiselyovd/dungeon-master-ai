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
    actor.loadout = Some(loadout_from_sheet(sheet)?);
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
        npc.loadout = Some(app_domain::combat::loadout::CombatLoadout {
            default_weapon: "unarmed".into(),
            attacks: vec![app_domain::combat::loadout::WeaponAttackProfile::unarmed(
                0, 2,
            )],
            quantities: Default::default(),
            arrows: 0,
            shield: false,
        });
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
    let (actors, order) =
        super::projection::resolver_state(&projection.snapshot).ok_or("combat_state_invalid")?;
    let mut engine = app_domain::combat::resolver::CombatResolver::new(
        actors,
        order,
        SeededRng::from_seed(seed),
    );
    let mut events = Vec::new();
    super::automation::settle_npcs(&mut engine, &mut events)?;
    super::projection::replace_resolved_state(
        projection,
        engine.combatants,
        engine.order,
        Default::default(),
    );
    projection.revision = 0;
    projection.events = events;
    super::automation::finish_if_resolved(projection);
    Ok(())
}

/// Upgrade only missing profiles; preserve the accepted encounter timeline.
pub fn recover_bound_loadouts(
    character_id: Uuid,
    sheet: &Value,
    projection: &mut crate::models::combat::CombatProjection,
) -> Result<(), &'static str> {
    for actor in &mut projection.snapshot.combatants {
        if actor.loadout.is_some() {
            continue;
        }
        actor.loadout = Some(if let Some(owner) = actor.character_id {
            if owner.0 != character_id {
                return Err("combat_hero_identity_mismatch");
            }
            loadout_from_sheet(sheet)?
        } else {
            app_domain::combat::loadout::CombatLoadout {
                default_weapon: "unarmed".into(),
                attacks: vec![app_domain::combat::loadout::WeaponAttackProfile::unarmed(
                    0, 2,
                )],
                quantities: Default::default(),
                arrows: 0,
                shield: false,
            }
        });
    }
    Ok(())
}

fn loadout_from_sheet(
    sheet: &Value,
) -> Result<app_domain::combat::loadout::CombatLoadout, &'static str> {
    use app_domain::combat::loadout::{CombatLoadout, LoadoutInput};
    let class = sheet["heroClass"].as_str().ok_or("combat_hero_invalid")?;
    let mut owned = Vec::new();
    let mut arrows = 0u32;
    for item in sheet["inventory"]
        .as_array()
        .ok_or("combat_hero_inventory_invalid")?
    {
        let id = item["id"].as_str().ok_or("combat_hero_inventory_invalid")?;
        let count = u32::try_from(
            item["count"]
                .as_u64()
                .ok_or("combat_hero_inventory_invalid")?,
        )
        .map_err(|_| "combat_hero_inventory_invalid")?;
        let normalized = id.strip_prefix(&format!("{class}-")).unwrap_or(id);
        let ammunition = match normalized {
            "arrows-20" => count.checked_mul(20),
            "arrows" => Some(count),
            _ => Some(0),
        }
        .ok_or("combat_hero_inventory_invalid")?;
        arrows = arrows
            .checked_add(ammunition)
            .ok_or("combat_hero_inventory_invalid")?;
        owned.push((id.into(), count));
    }
    if let Some(saved) = sheet.get("combatResources").and_then(|r| r.get("arrows")) {
        arrows = u32::try_from(saved.as_u64().ok_or("combat_resources_invalid")?)
            .map_err(|_| "combat_resources_invalid")?
            .min(arrows);
    }
    let mut loadout = CombatLoadout::derive(LoadoutInput {
        class_id: class,
        level: sheet["level"].as_u64().ok_or("combat_hero_invalid")? as u8,
        strength: sheet["abilities"]["str"]
            .as_i64()
            .ok_or("combat_hero_invalid")? as i32,
        dexterity: sheet["abilities"]["dex"]
            .as_i64()
            .ok_or("combat_hero_invalid")? as i32,
        owned,
        arrows,
    })?;
    if let Some(saved) = sheet
        .get("combatResources")
        .and_then(|r| r.get("quantities"))
        .and_then(Value::as_object)
    {
        for (id, quantity) in &mut loadout.quantities {
            if let Some(value) = saved.get(id) {
                *quantity = u32::try_from(value.as_u64().ok_or("combat_resources_invalid")?)
                    .map_err(|_| "combat_resources_invalid")?
                    .min(*quantity);
            }
        }
    }
    Ok(loadout)
}
