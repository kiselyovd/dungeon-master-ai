use app_application::{
    models::room_recovery::RecoveryIntent,
    recovery::{self, RecoveryEngine},
};
use app_domain::combat::conditions::Condition;
use serde_json::json;
use uuid::Uuid;
fn sheet() -> serde_json::Value {
    json!({"characterId":Uuid::new_v4(),"name":"Medic","heroClass":"fighter","level":5,"hp":12,"hpMax":12,"ac":14,
 "abilities":{"str":15,"dex":14,"con":13,"int":12,"wis":16,"cha":8},"skillProfs":{"medicine":true},"inventory":[]})
}
#[test]
fn medicine_uses_committed_proficiency_and_poison_disadvantage() {
    let sheet = sheet();
    let id = Uuid::parse_str(sheet["characterId"].as_str().unwrap()).unwrap();
    let mut actor = recovery::actor(&sheet, id).unwrap();
    actor.add_condition(Condition::Poisoned);
    let roll = RecoveryEngine::new(7).medicine(&actor, &sheet).unwrap();
    assert_eq!(roll.modifier, 6);
    assert!(roll.disadvantage);
    assert_eq!(roll.rolls.len(), 2);
    assert_eq!(roll.total, *roll.rolls.iter().min().unwrap() + 6);
}
#[test]
fn death_save_intent_rejects_client_roll_or_foreign_actor_fields() {
    assert!(serde_json::from_value::<RecoveryIntent>(json!({"kind":"death_save"})).is_ok());
    for extra in [
        json!({"kind":"death_save","roll":20}),
        json!({"kind":"death_save","target_character_id":Uuid::new_v4()}),
    ] {
        assert!(serde_json::from_value::<RecoveryIntent>(extra).is_err());
    }
}
#[test]
fn unattended_dying_resolves_with_bounded_rolls_and_character_identity() {
    let mut sheet = sheet();
    sheet["hp"] = json!(0);
    let id = Uuid::parse_str(sheet["characterId"].as_str().unwrap()).unwrap();
    let result = RecoveryEngine::new(9)
        .passage(&mut sheet, id, 0, 8 * 3600)
        .unwrap();
    assert!(!result.is_empty());
    assert!(result.len() <= 5);
    assert!(result.iter().all(|s| s.character_id == id));
    let state = app_application::vitality::read(&sheet, id)
        .unwrap()
        .unwrap();
    assert!(state.is_dead || state.death_saves.stable || sheet["hp"].as_i64().unwrap() > 0);
}
