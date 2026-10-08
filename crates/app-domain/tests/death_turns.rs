use app_domain::{
    combat::{
        combatant::Combatant,
        conditions::Condition,
        initiative::{InitiativeEntry, InitiativeOrder},
        resolver::{CombatAction, CombatResolver},
        types::{CharacterId, CombatantId},
    },
    dice::{roll_one, Die},
    rng::SeededRng,
};
use std::collections::HashMap;
fn seed(face: i32) -> u64 {
    (0..10000)
        .find(|s| roll_one(Die::D20, &mut SeededRng::from_seed(*s)) == face)
        .unwrap()
}
fn engine(face: i32) -> (CombatResolver, CombatantId, CombatantId, CombatantId) {
    let first = CombatantId::new();
    let dying = CombatantId::new();
    let last = CombatantId::new();
    let mut a = Combatant::new(first, "Human A".into(), 12, 12, 14);
    a.character_id = Some(CharacterId(first.0));
    let mut b = Combatant::new(dying, "Human B".into(), 12, 0, 14);
    b.character_id = Some(CharacterId(dying.0));
    b.add_condition(Condition::Unconscious);
    let mut c = Combatant::new(last, "Human C".into(), 12, 12, 14);
    c.character_id = Some(CharacterId(last.0));
    let order = InitiativeOrder::build(vec![
        InitiativeEntry {
            id: first,
            roll: 20,
            dex_tiebreak: 0,
        },
        InitiativeEntry {
            id: dying,
            roll: 15,
            dex_tiebreak: 0,
        },
        InitiativeEntry {
            id: last,
            roll: 10,
            dex_tiebreak: 0,
        },
    ]);
    (
        CombatResolver::new(
            HashMap::from([(first, a), (dying, b), (last, c)]),
            order,
            SeededRng::from_seed(seed(face)),
        ),
        first,
        dying,
        last,
    )
}
#[test]
fn dying_turn_rolls_once_then_yields_to_the_next_human() {
    let (mut e, a, b, c) = engine(9);
    let result = e.resolve(CombatAction::EndTurn { combatant: a }).unwrap();
    assert_eq!(result.death_saves.len(), 1);
    assert_eq!(result.death_saves[0].combatant, b);
    assert_eq!(result.death_saves[0].roll, 9);
    assert_eq!(e.combatants[&b].death_saves.failures, 1);
    assert_eq!(e.order.current().id, c);
}
#[test]
fn natural_twenty_returns_control_of_the_revived_hero_without_choosing_an_action() {
    let (mut e, a, b, _) = engine(20);
    let result = e.resolve(CombatAction::EndTurn { combatant: a }).unwrap();
    assert_eq!(result.death_saves[0].roll, 20);
    assert_eq!(e.order.current().id, b);
    assert_eq!(e.combatants[&b].current_hp, 1);
    assert!(e.combatants[&b].budget.action);
    assert!(!e.combatants[&b].has_condition(Condition::Unconscious));
    assert!(result.damage.is_empty());
    assert!(result.movement.is_empty());
}
#[test]
fn stable_and_dead_heroes_skip_without_a_save() {
    for dead in [false, true] {
        let (mut e, a, b, c) = engine(1);
        let actor = e.combatants.get_mut(&b).unwrap();
        actor.is_dead = dead;
        actor.death_saves.stable = !dead;
        let result = e.resolve(CombatAction::EndTurn { combatant: a }).unwrap();
        assert!(result.death_saves.is_empty());
        assert_eq!(e.order.current().id, c);
    }
}
#[test]
fn encounter_boundary_stops_before_a_new_round_death_save() {
    let (mut e, a, b, _) = engine(1);
    e.set_turn_boundary(Some(1), false);
    let result = e.resolve(CombatAction::EndTurn { combatant: a }).unwrap();
    assert!(result.death_saves.is_empty());
    assert_eq!(e.combatants[&b].death_saves.failures, 0);
}
#[test]
fn explicit_opening_turn_rolls_a_dying_first_actor() {
    let (mut e, _, b, c) = engine(10);
    e.order = InitiativeOrder::restore(e.order.as_slice().to_vec(), b, 1).unwrap();
    let result = e.begin_current_turn().unwrap();
    assert_eq!(result.death_saves.len(), 1);
    assert_eq!(e.order.current().id, c);
}

#[test]
fn withdrawn_dying_hero_neither_rolls_nor_refreshes_budget_or_consumes_rng() {
    for face in [1, 20] {
        let (mut e, a, b, c) = engine(face);
        let actor = e.combatants.get_mut(&b).unwrap();
        actor.withdrawn = true;
        actor.budget.action = false;
        actor.budget.movement_ft = 7;
        let before = serde_json::to_value(&*actor).unwrap();
        let result = e.resolve(CombatAction::EndTurn { combatant: a }).unwrap();
        assert_eq!(e.order.current().id, c);
        assert!(result.death_saves.is_empty());
        assert!(result.healing.is_empty());
        assert_eq!(serde_json::to_value(&e.combatants[&b]).unwrap(), before);
        let actor = e.combatants.get_mut(&c).unwrap();
        actor.current_hp = 0;
        actor.add_condition(Condition::Unconscious);
        e.resolve(CombatAction::EndTurn { combatant: c }).unwrap();
        let next = e.resolve(CombatAction::EndTurn { combatant: a }).unwrap();
        assert_eq!(next.death_saves[0].combatant, c);
        assert_eq!(next.death_saves[0].roll, face);
    }
}
