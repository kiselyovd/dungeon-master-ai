use app_application::combat::{authored::bell_defense, automation::settle_npcs_until};
use app_domain::{
    combat::{
        combatant::Combatant,
        initiative::{InitiativeEntry, InitiativeOrder},
        resolver::{CombatAction, CombatResolver},
        types::{CharacterId, CombatantId, Position},
    },
    rng::SeededRng,
};
#[test]
fn holding_three_rounds_stops_before_npcs_in_the_fourth_even_when_hero_is_last() {
    let projection = bell_defense(uuid::Uuid::new_v4());
    let hero = CombatantId::new();
    let mut actor = Combatant::new(hero, "Hero".into(), 8, 1, 1);
    actor.character_id = Some(CharacterId(uuid::Uuid::new_v4()));
    actor.position = Position { x: 3, y: 3 };
    let mut npcs = projection.snapshot.combatants;
    for (index, npc) in npcs.iter_mut().enumerate() {
        npc.position = Position {
            x: 4,
            y: 2 + index as i32 * 2,
        };
    }
    let entries = vec![
        InitiativeEntry {
            id: npcs[0].id,
            roll: 10,
            dex_tiebreak: 0,
        },
        InitiativeEntry {
            id: npcs[1].id,
            roll: 9,
            dex_tiebreak: 0,
        },
        InitiativeEntry {
            id: hero,
            roll: 0,
            dex_tiebreak: 0,
        },
    ];
    let order = InitiativeOrder::restore(entries, hero, 3).unwrap();
    npcs.push(actor);
    let mut engine = CombatResolver::new(
        npcs.into_iter().map(|actor| (actor.id, actor)).collect(),
        order,
        SeededRng::from_seed(42),
    );
    engine.set_map(projection.authored.unwrap().map).unwrap();
    engine
        .resolve(CombatAction::EndTurn { combatant: hero })
        .unwrap();
    let mut events = vec![];
    settle_npcs_until(&mut engine, &mut events, Some(4)).unwrap();
    assert_eq!(engine.combatants[&hero].current_hp, 1);
    assert!(events.is_empty());
}
