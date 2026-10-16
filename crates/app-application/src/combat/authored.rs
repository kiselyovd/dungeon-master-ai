use crate::models::combat::{
    AuthoredEncounter, AuthoredEncounterKind, CombatProjection, CombatSnapshot, OpponentArchetype,
    COMBAT_PROJECTION_VERSION,
};
use app_domain::{
    adventure::DefenseOutcome,
    combat::{
        combatant::Combatant,
        geometry::bell_archive_map,
        initiative::InitiativeEntry,
        loadout::{CombatLoadout, WeaponAttackProfile},
        result_events::{HealingApplication, ResultEvents},
        types::{CombatantId, DamageType, Position},
    },
    dice::{DiceExpr, Die},
};
use uuid::Uuid;

pub fn bell_defense(encounter_id: Uuid) -> CombatProjection {
    let actors = [Position { x: 9, y: 1 }, Position { x: 9, y: 5 }]
        .into_iter()
        .enumerate()
        .map(|(index, position)| {
            let mut actor = Combatant::new(
                CombatantId::new(),
                format!("Water echo {}", index + 1),
                6,
                6,
                11,
            );
            actor.display_name_key = Some(format!("authored.water_echo_{}", index + 1));
            actor.position = position;
            actor.speed_ft = 15;
            actor.budget = app_domain::combat::combatant::ActionBudget::fresh(15);
            actor.loadout = Some(CombatLoadout {
                default_weapon: "echo-pulse".into(),
                attacks: vec![WeaponAttackProfile {
                    weapon_id: "echo-pulse".into(),
                    attack_bonus: 3,
                    damage: DiceExpr {
                        count: 1,
                        die: Die::D4,
                        modifier: 1,
                    },
                    damage_type: DamageType::Bludgeoning,
                    reach_ft: 5,
                    normal_range_ft: 5,
                    long_range_ft: 5,
                    ranged: false,
                    thrown: false,
                    ammunition: false,
                    two_handed: false,
                    versatile_damage: None,
                    heavy_disadvantage: false,
                }],
                quantities: std::collections::BTreeMap::from([("echo-pulse".into(), 1)]),
                arrows: 0,
                shield: false,
            });
            actor
        })
        .collect::<Vec<_>>();
    CombatProjection {
        authored: Some(AuthoredEncounter {
            map_skin_asset_id: Some("map.bell_archive.v2".into()),
            kind: AuthoredEncounterKind::BellArchiveDefense,
            map: bell_archive_map(),
            outcome: None,
        }),
        adventure: None,
        campaign_scope: None,
        appearances: actors
            .iter()
            .map(|actor| (actor.id.0, OpponentArchetype::WaterEcho))
            .collect(),
        schema_version: COMBAT_PROJECTION_VERSION,
        encounter_id,
        revision: 0,
        events: vec![],
        snapshot: CombatSnapshot {
            active: true,
            round: 1,
            current_combatant: None,
            initiative: actors
                .iter()
                .map(|actor| InitiativeEntry {
                    id: actor.id,
                    roll: 0,
                    dex_tiebreak: 0,
                })
                .collect(),
            combatants: actors,
        },
    }
}

pub fn finalize(projection: &mut CombatProjection, retreat: bool) -> Option<DefenseOutcome> {
    let authored = projection.authored.as_ref()?;
    if let Some(outcome) = authored.outcome {
        return Some(outcome);
    }
    let heroes = projection
        .snapshot
        .combatants
        .iter()
        .filter(|actor| actor.character_id.is_some() && !actor.withdrawn)
        .collect::<Vec<_>>();
    let living = heroes
        .iter()
        .any(|actor| actor.current_hp > 0 && !actor.is_dead);
    let enemies = projection
        .snapshot
        .combatants
        .iter()
        .any(|actor| actor.character_id.is_none() && actor.current_hp > 0 && !actor.is_dead);
    let outcome = if !living {
        DefenseOutcome::Defeated
    } else if retreat {
        DefenseOutcome::Retreated
    } else if !enemies || projection.snapshot.round >= 4 {
        DefenseOutcome::Held
    } else if !projection.snapshot.active {
        DefenseOutcome::Retreated
    } else {
        return None;
    };
    projection.snapshot.active = false;
    projection.snapshot.current_combatant = None;
    projection.authored.as_mut().unwrap().outcome = Some(outcome);
    Some(outcome)
}

pub fn recover_defeated_hero(projection: &mut CombatProjection) -> Result<(), &'static str> {
    let heroes = projection
        .snapshot
        .combatants
        .iter_mut()
        .filter(|actor| actor.character_id.is_some() && !actor.withdrawn)
        .collect::<Vec<_>>();
    if heroes.is_empty() {
        return Err("authored_hero_missing");
    }
    for hero in heroes {
        if hero.current_hp == 0 && !hero.is_dead {
            hero.apply_healing(1);
            let mut events = ResultEvents::default();
            events.healing.push(HealingApplication {
                target: hero.id,
                amount: 1,
                new_hp: hero.current_hp,
            });
            projection.events.push(events);
        }
    }
    Ok(())
}

pub fn prepare_end(projection: &mut CombatProjection) -> Result<(), &'static str> {
    let Some(authored) = &projection.authored else {
        return Ok(());
    };
    if authored.outcome.is_some()
        || projection.snapshot.round >= 4
        || !projection
            .snapshot
            .combatants
            .iter()
            .any(|actor| actor.character_id.is_none() && actor.current_hp > 0 && !actor.is_dead)
        || !projection.snapshot.combatants.iter().any(|actor| {
            actor.character_id.is_some()
                && !actor.withdrawn
                && actor.current_hp > 0
                && !actor.is_dead
        })
    {
        return Ok(());
    }
    let current = projection
        .snapshot
        .current_combatant
        .ok_or("authored_exit_required")?;
    let actor = projection
        .snapshot
        .combatants
        .iter_mut()
        .find(|actor| actor.id == current)
        .ok_or("authored_exit_required")?;
    if actor.character_id.is_none()
        || !authored.map.exits.contains(&actor.position)
        || !actor.budget.action
    {
        return Err("authored_exit_required");
    }
    app_domain::combat::action_economy::consume_action(
        &mut actor.budget,
        app_domain::combat::types::ActionKind::Action,
    )
    .map_err(|_| "authored_exit_required")?;
    Ok(())
}
