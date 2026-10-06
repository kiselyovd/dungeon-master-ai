use app_domain::combat::{
    combatant::Combatant,
    initiative::{InitiativeEntry, InitiativeOrder},
    loadout::{CombatLoadout, WeaponAttackProfile},
    resolver::{CombatAction, CombatResolver},
    types::{CharacterId, CombatantId, Position},
};
use app_domain::rng::SeededRng;
use uuid::Uuid;
#[test]
fn npc_closes_distance_attacks_and_returns_the_turn_to_the_hero() {
    let npc = CombatantId::new();
    let hero = CombatantId::new();
    let mut enemy = Combatant::new(npc, "Enemy".into(), 10, 10, 10);
    enemy.position = Position { x: 5, y: 0 };
    enemy.loadout = Some(CombatLoadout {
        default_weapon: "unarmed".into(),
        attacks: vec![WeaponAttackProfile::unarmed(0, 2)],
        quantities: Default::default(),
        arrows: 0,
        shield: false,
    });
    let mut player = Combatant::new(hero, "Hero".into(), 20, 20, 10);
    player.character_id = Some(CharacterId(Uuid::new_v4()));
    let order = InitiativeOrder::build(vec![
        InitiativeEntry {
            id: npc,
            roll: 20,
            dex_tiebreak: 0,
        },
        InitiativeEntry {
            id: hero,
            roll: 10,
            dex_tiebreak: 0,
        },
    ]);
    let mut engine = CombatResolver::new(
        std::collections::HashMap::from([(npc, enemy), (hero, player)]),
        order,
        SeededRng::from_seed(42),
    );
    let events = engine
        .resolve(CombatAction::NpcTurn { combatant: npc })
        .unwrap();
    assert!(!events.movement.is_empty());
    assert_eq!(events.rolls.len(), 1);
    assert_eq!(engine.order.current().id, hero);
    assert!(
        engine.combatants[&npc]
            .position
            .distance_ft(engine.combatants[&hero].position)
            <= 5
    );
    assert!(!engine.combatants[&npc].budget.action);
}
