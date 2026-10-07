use app_application::{combat::hero::hero_combatant, vitality};
use app_domain::{
    combat::{conditions::Condition, healing::begin_natural_recovery},
    rng::SeededRng,
};
use serde_json::json;
use uuid::Uuid;
fn sheet(id: Uuid) -> serde_json::Value {
    json!({"characterId":id,"name":"Hero","heroClass":"fighter","level":1,"hp":0,"hpMax":12,"ac":14,
        "abilities":{"str":15,"dex":14,"con":13,"int":12,"wis":10,"cha":8},"inventory":[]})
}
#[test]
fn vitality_keeps_death_saves_and_conditions_when_binding_another_encounter() {
    let id = Uuid::new_v4();
    let mut sheet = sheet(id);
    let mut actor = hero_combatant(id, &sheet).unwrap();
    actor.death_saves.successes = 1;
    actor.death_saves.failures = 2;
    actor.add_condition(Condition::Poisoned);
    vitality::store_actor(&mut sheet, &actor).unwrap();
    let next = hero_combatant(id, &sheet).unwrap();
    assert_ne!(next.id, actor.id);
    assert_eq!(next.death_saves.successes, 1);
    assert_eq!(next.death_saves.failures, 2);
    assert!(next.has_condition(Condition::Poisoned));
    assert!(next.has_condition(Condition::Unconscious));
    assert!(!next.is_dead);
}
#[test]
fn scheduled_recovery_survives_json_and_rebinding_without_rerolling_duration() {
    let id = Uuid::new_v4();
    let mut sheet = sheet(id);
    let mut actor = hero_combatant(id, &sheet).unwrap();
    actor.death_saves.stabilize();
    vitality::store_actor(&mut sheet, &actor).unwrap();
    let deadline = begin_natural_recovery(&actor, 100, &mut SeededRng::from_seed(11)).unwrap();
    vitality::schedule_natural_recovery(&mut sheet, id, deadline.clone()).unwrap();
    vitality::schedule_natural_recovery(&mut sheet, id, deadline.clone()).unwrap();
    let reopened = serde_json::from_str(&sheet.to_string()).unwrap();
    let next = hero_combatant(id, &reopened).unwrap();
    vitality::store_actor(&mut sheet, &next).unwrap();
    let persisted = vitality::read(&sheet, id)
        .unwrap()
        .unwrap()
        .natural_recovery
        .unwrap();
    assert_eq!(persisted.combatant, next.id);
    assert_eq!(persisted.ready_at_seconds, deadline.ready_at_seconds);
    let mut changed = persisted.clone();
    changed.ready_at_seconds += 3600;
    assert!(vitality::schedule_natural_recovery(&mut sheet, id, changed).is_err());
}
#[test]
fn dead_state_and_foreign_identity_cannot_be_healed_or_rebound() {
    let id = Uuid::new_v4();
    let mut sheet = sheet(id);
    let mut actor = hero_combatant(id, &sheet).unwrap();
    actor.is_dead = true;
    actor.death_saves.failures = 3;
    vitality::store_actor(&mut sheet, &actor).unwrap();
    assert!(hero_combatant(id, &sheet).unwrap().is_dead);
    assert_eq!(
        vitality::clear_after_healing(&mut sheet, 1),
        Err("hero_dead")
    );
    assert!(hero_combatant(Uuid::new_v4(), &sheet).is_err());
}
#[test]
fn positive_healing_clears_unconsciousness_saves_and_deadline_but_keeps_other_conditions() {
    let id = Uuid::new_v4();
    let mut sheet = sheet(id);
    let mut actor = hero_combatant(id, &sheet).unwrap();
    actor.death_saves.stabilize();
    actor.add_condition(Condition::Poisoned);
    vitality::store_actor(&mut sheet, &actor).unwrap();
    let deadline = begin_natural_recovery(&actor, 0, &mut SeededRng::from_seed(11)).unwrap();
    vitality::schedule_natural_recovery(&mut sheet, id, deadline).unwrap();
    vitality::clear_after_healing(&mut sheet, 1).unwrap();
    sheet["hp"] = json!(1);
    let state = vitality::read(&sheet, id).unwrap().unwrap();
    assert!(!state.death_saves.stable);
    assert!(state.natural_recovery.is_none());
    assert!(!state.conditions.contains(&Condition::Unconscious));
    assert!(state.conditions.contains(&Condition::Poisoned));
}
#[test]
fn player_drafts_cannot_supply_vitality_and_invalid_updates_leave_state_unchanged() {
    let id = Uuid::new_v4();
    let mut sheet = sheet(id);
    sheet["combatResources"] = json!({"vitality":{"is_dead":false},"spellSlots":{"1":1}});
    vitality::strip_client_state(&mut sheet);
    assert!(sheet["combatResources"].get("vitality").is_none());
    assert_eq!(sheet["combatResources"]["spellSlots"]["1"], 1);
    let mut actor = hero_combatant(id, &sheet).unwrap();
    actor.death_saves.failures = 3;
    let before = sheet.clone();
    assert!(vitality::store_actor(&mut sheet, &actor).is_err());
    assert_eq!(sheet, before);
}

#[test]
fn unchanged_hp_wait_preserves_unconsciousness_and_other_conditions() {
    let id = Uuid::new_v4();
    let mut sheet = sheet(id);
    sheet["hp"] = json!(12);
    let mut actor = hero_combatant(id, &sheet).unwrap();
    actor.add_condition(Condition::Unconscious);
    actor.add_condition(Condition::Poisoned);
    vitality::store_actor(&mut sheet, &actor).unwrap();
    let outcome = app_domain::rest::RestOutcome {
        hp: 12,
        elapsed_seconds: 60,
        duration_minutes: 1,
        resources: app_domain::rest::RestResources {
            hit_dice_remaining: 1,
            last_long_rest_completed: None,
            spell_slots: Default::default(),
        },
        rolls: vec![],
    };
    app_application::rest::apply_rest_to_sheet(&mut sheet, &outcome).unwrap();
    let state = vitality::read(&sheet, id).unwrap().unwrap();
    assert!(state.conditions.contains(&Condition::Unconscious));
    assert!(state.conditions.contains(&Condition::Poisoned));
}
