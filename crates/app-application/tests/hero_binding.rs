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

#[test]
fn returning_hero_uses_a_free_cell_without_refreshing_spent_budgets() {
    use app_application::{
        combat::resolve::reconcile_participants,
        models::combat::{
            AuthoredEncounter, AuthoredEncounterKind, CombatProjection, CombatSnapshot,
        },
    };
    use app_domain::combat::{
        combatant::Combatant,
        geometry::BattleMap,
        initiative::InitiativeEntry,
        types::{CharacterId, CombatantId, Position},
    };
    let root = CombatantId::new();
    let guest = CombatantId::new();
    let npc = CombatantId::new();
    let mut a = Combatant::new(root, "A".into(), 12, 8, 14);
    a.character_id = Some(CharacterId(root.0));
    a.position = Position { x: 0, y: 0 };
    a.budget.action = false;
    a.budget.movement_ft = 17;
    let mut b = Combatant::new(guest, "B".into(), 12, 6, 14);
    b.character_id = Some(CharacterId(guest.0));
    b.withdrawn = true;
    b.position = Position { x: 1, y: 0 };
    b.budget.action = false;
    b.budget.movement_ft = 7;
    let mut enemy = Combatant::new(npc, "NPC".into(), 6, 6, 11);
    enemy.position = Position { x: 1, y: 0 };
    let mut projection = CombatProjection::empty(Uuid::new_v4(), 0);
    projection.authored = Some(AuthoredEncounter {
        map_skin_asset_id: None,
        kind: AuthoredEncounterKind::BellArchiveDefense,
        outcome: None,
        map: BattleMap {
            width: 3,
            height: 1,
            blocked: vec![],
            exits: vec![a.position],
        },
    });
    projection.snapshot = CombatSnapshot {
        active: true,
        round: 1,
        current_combatant: Some(root),
        initiative: vec![
            InitiativeEntry {
                id: root,
                roll: 20,
                dex_tiebreak: 0,
            },
            InitiativeEntry {
                id: guest,
                roll: 15,
                dex_tiebreak: 0,
            },
            InitiativeEntry {
                id: npc,
                roll: 10,
                dex_tiebreak: 0,
            },
        ],
        combatants: vec![a, b, enemy],
    };
    let ready =
        reconcile_participants(projection, &[root.0, guest.0].into_iter().collect(), 42).unwrap();
    let returned = ready
        .snapshot
        .combatants
        .iter()
        .find(|a| a.id == guest)
        .unwrap();
    assert_eq!(returned.position, Position { x: 2, y: 0 });
    assert!(!returned.withdrawn);
    assert!(!returned.budget.action);
    assert_eq!(returned.budget.movement_ft, 7);
    assert_eq!(returned.current_hp, 6);
    let root = ready
        .snapshot
        .combatants
        .iter()
        .find(|a| a.id == root)
        .unwrap();
    assert!(!root.budget.action);
    assert_eq!(root.budget.movement_ft, 17);
    assert_eq!(ready.snapshot.round, 1);
    assert!(ready.events.is_empty());
}
