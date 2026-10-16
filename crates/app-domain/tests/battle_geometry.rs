use app_domain::combat::{geometry::BattleMap, types::Position};
use std::collections::HashSet;
#[test]
fn path_cost_uses_a_passage_and_rejects_walls_occupied_cells_and_outside_targets() {
    let map = BattleMap {
        width: 5,
        height: 5,
        blocked: vec![
            Position { x: 2, y: 0 },
            Position { x: 2, y: 1 },
            Position { x: 2, y: 2 },
            Position { x: 2, y: 4 },
        ],
        exits: vec![Position { x: 0, y: 2 }],
    };
    let from = Position { x: 0, y: 1 };
    let to = Position { x: 4, y: 1 };
    let path = map.path(from, to, &HashSet::new()).unwrap();
    assert!(path.len() > 4);
    assert!(path.contains(&Position { x: 2, y: 3 }));
    assert!(map
        .path(from, Position { x: 2, y: 2 }, &HashSet::new())
        .is_none());
    assert!(map
        .path(from, Position { x: 6, y: 1 }, &HashSet::new())
        .is_none());
    assert!(map.path(from, to, &HashSet::from([to])).is_none());
    assert!(!map.clear_line(from, to));
}
#[test]
fn diagonal_paths_do_not_cut_a_solid_corner() {
    let map = BattleMap {
        width: 3,
        height: 3,
        blocked: vec![Position { x: 1, y: 0 }, Position { x: 0, y: 1 }],
        exits: vec![Position { x: 2, y: 2 }],
    };
    assert!(map
        .path(
            Position { x: 0, y: 0 },
            Position { x: 1, y: 1 },
            &HashSet::new()
        )
        .is_none());
}

#[test]
fn movement_uses_the_real_path_cost_and_retreat_requires_the_exit() {
    use app_domain::{
        combat::{
            combatant::Combatant,
            initiative::{InitiativeEntry, InitiativeOrder},
            resolver::{CombatAction, CombatResolver},
            types::{CharacterId, CombatantId},
        },
        rng::SeededRng,
    };
    let hero = CombatantId::new();
    let mut actor = Combatant::new(hero, "Hero".into(), 12, 12, 16);
    actor.character_id = Some(CharacterId(uuid::Uuid::new_v4()));
    actor.position = Position { x: 1, y: 1 };
    actor.budget.movement_ft = 20;
    let order = InitiativeOrder::build(vec![InitiativeEntry {
        id: hero,
        roll: 10,
        dex_tiebreak: 0,
    }]);
    let mut engine = CombatResolver::new(
        std::collections::HashMap::from([(hero, actor)]),
        order,
        SeededRng::from_seed(42),
    );
    engine
        .set_map(BattleMap {
            width: 5,
            height: 5,
            blocked: vec![
                Position { x: 2, y: 0 },
                Position { x: 2, y: 1 },
                Position { x: 2, y: 2 },
                Position { x: 2, y: 4 },
            ],
            exits: vec![Position { x: 0, y: 2 }],
        })
        .unwrap();
    assert!(engine
        .resolve(CombatAction::Move {
            combatant: hero,
            to: Position { x: 3, y: 1 }
        })
        .is_err());
    assert_eq!(engine.combatants[&hero].position, Position { x: 1, y: 1 });
    assert_eq!(engine.combatants[&hero].budget.movement_ft, 20);
    assert!(engine
        .resolve(CombatAction::Retreat { combatant: hero })
        .is_err());
    engine
        .resolve(CombatAction::Move {
            combatant: hero,
            to: Position { x: 0, y: 2 },
        })
        .unwrap();
    engine
        .resolve(CombatAction::Retreat { combatant: hero })
        .unwrap();
    assert!(!engine.combatants[&hero].budget.action);
}
