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

#[test]
fn untrained_defense_binds_and_repairs_legacy_snapshot_without_resetting_resources() {
    use app_application::{
        combat::hero::{bind_party, recover_party_loadouts},
        models::combat::CombatProjection,
    };
    let id = Uuid::new_v4();
    let sheet = json!({"name":"Mage","heroClass":"wizard","level":1,"hp":8,"hpMax":8,"ac":1,"abilities":{"str":12,"dex":14,"con":14,"int":16,"wis":12,"cha":10},"inventory":[{"id":"chain-mail","name":"Chain Mail","count":1},{"id":"quarterstaff","name":"Quarterstaff","count":1}],"defense":{"schema_version":1,"armor_id":"chain-mail","shield":false},"spells":{"cantrips":["fire-bolt"],"level1":["magic-missile"]}});
    let mut projection = CombatProjection::empty(Uuid::new_v4(), 1);
    bind_party(&[(id, &sheet)], &mut projection, 42).unwrap();
    let actor = &mut projection.snapshot.combatants[0];
    assert!(actor.untrained_armor);
    assert_eq!(actor.ac, 16);
    assert!(actor.casting.as_ref().unwrap().choices.is_empty());
    let expected = app_domain::dice::roll_d20_detailed(
        2,
        false,
        true,
        &mut app_domain::rng::SeededRng::from_seed(42),
    );
    assert_eq!(actor.initiative_roll, expected.total);
    actor
        .casting
        .as_mut()
        .unwrap()
        .slots
        .get_mut(&1)
        .unwrap()
        .remaining = 1;
    actor.budget.action = false;
    actor.untrained_armor = false;

    recover_party_loadouts(&[(id, &sheet)], &mut projection).unwrap();
    let actor = &projection.snapshot.combatants[0];
    assert!(actor.untrained_armor);
    assert!(!actor.budget.action);
    assert_eq!(actor.casting.as_ref().unwrap().slots[&1].remaining, 1);
    assert!(actor.casting.as_ref().unwrap().choices.is_empty());
    let mut wire = serde_json::to_value(actor).unwrap();
    wire.as_object_mut().unwrap().remove("untrained_armor");
    let legacy: app_domain::combat::combatant::Combatant = serde_json::from_value(wire).unwrap();
    assert!(!legacy.untrained_armor);
}
