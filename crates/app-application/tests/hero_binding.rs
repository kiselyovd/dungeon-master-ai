use app_application::combat::hero::hero_combatant;
use serde_json::json;
use uuid::Uuid;

#[test]
fn hero_actor_uses_durable_identity_stats_and_derived_initiative() {
    let character_id = Uuid::new_v4();
    let actor=hero_combatant(character_id,&json!({"name":"Mira","heroClass":"fighter","level":1,"hp":7,"hpMax":12,"ac":16,"speedFt":35,"initiative":99,"abilities":{"str":16,"dex":14,"con":14,"int":10,"wis":12,"cha":8},"inventory":[]})).unwrap();
    assert_eq!(actor.character_id.unwrap().0, character_id);
    assert_ne!(
        actor.id.0, character_id,
        "actor token belongs to this encounter, character ID persists across encounters"
    );
    assert_eq!(actor.name, "Mira");
    assert_eq!(
        (
            actor.current_hp,
            actor.max_hp,
            actor.ac,
            actor.dex_mod,
            actor.speed_ft
        ),
        (7, 12, 16, 2, 35)
    );
    assert_eq!(actor.budget.movement_ft, 35);
}

#[test]
fn malformed_speed_and_sheet_cannot_create_a_bound_actor() {
    assert!(hero_combatant(Uuid::new_v4(), &json!({"name":"Mira"})).is_err());
    let sheet = json!({"name":"Mira","heroClass":"fighter","level":1,"hp":7,"hpMax":12,"ac":16,"speedFt":-1,"abilities":{"str":16,"dex":14,"con":14,"int":10,"wis":12,"cha":8},"inventory":[]});
    assert!(hero_combatant(Uuid::new_v4(), &sheet).is_err());
}
