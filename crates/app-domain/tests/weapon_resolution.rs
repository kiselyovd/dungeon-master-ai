use app_domain::combat::{
    combatant::Combatant,
    initiative::{InitiativeEntry, InitiativeOrder},
    loadout::{CombatLoadout, LoadoutInput, WeaponMode},
    resolver::{CombatAction, CombatResolver},
    types::{CharacterId, CombatantId, Position},
};
use app_domain::rng::SeededRng;
use uuid::Uuid;
fn fixture(bow: bool, position: Position) -> (CombatResolver, CombatantId, CombatantId) {
    let hero = CombatantId::new();
    let npc = CombatantId::new();
    let mut actor = Combatant::new(hero, "Hero".into(), 20, 20, 16);
    actor.character_id = Some(CharacterId(Uuid::new_v4()));
    actor.loadout = Some(
        CombatLoadout::derive(LoadoutInput {
            class_id: "fighter",
            level: 1,
            strength: 16,
            dexterity: 16,
            owned: vec![(if bow { "shortbow" } else { "longsword" }.into(), 1)],
            arrows: 2,
        })
        .unwrap(),
    );
    let mut target = Combatant::new(npc, "Foe".into(), 100, 100, 10);
    target.position = position;
    let order = InitiativeOrder::build(vec![
        InitiativeEntry {
            id: hero,
            roll: 20,
            dex_tiebreak: 2,
        },
        InitiativeEntry {
            id: npc,
            roll: 10,
            dex_tiebreak: 0,
        },
    ]);
    (
        CombatResolver::new(
            std::collections::HashMap::from([(hero, actor), (npc, target)]),
            order,
            SeededRng::from_seed(42),
        ),
        hero,
        npc,
    )
}
#[test]
fn bound_actor_without_a_profile_never_accepts_client_damage() {
    let (mut engine, hero, npc) = fixture(false, Position { x: 1, y: 0 });
    engine.combatants.get_mut(&hero).unwrap().loadout = None;
    assert!(engine
        .resolve(CombatAction::Attack {
            attacker: hero,
            target: npc,
            attack_modifier: 100,
            damage_expr: app_domain::dice::DiceExpr {
                count: 20,
                die: app_domain::dice::Die::D20,
                modifier: 100
            },
            damage_type: app_domain::combat::types::DamageType::Slashing,
        })
        .is_err());
    assert_eq!(engine.combatants[&npc].current_hp, 100);
    assert!(engine.combatants[&hero].budget.action);
}
#[test]
fn out_of_reach_does_not_spend_an_action_or_ammunition() {
    let (mut engine, hero, npc) = fixture(false, Position { x: 4, y: 0 });
    assert!(engine
        .resolve(CombatAction::WeaponAttack {
            attacker: hero,
            target: npc,
            weapon_id: None,
            mode: WeaponMode::Default,
            two_handed: false
        })
        .is_err());
    assert!(engine.combatants[&hero].budget.action);
}
#[test]
fn long_range_uses_disadvantage_and_spends_one_arrow() {
    let (mut engine, hero, npc) = fixture(true, Position { x: 17, y: 0 });
    let events = engine
        .resolve(CombatAction::WeaponAttack {
            attacker: hero,
            target: npc,
            weapon_id: None,
            mode: WeaponMode::Default,
            two_handed: false,
        })
        .unwrap();
    assert_eq!(events.rolls[0].detail.rolls.len(), 2);
    assert_eq!(engine.combatants[&hero].loadout.as_ref().unwrap().arrows, 1);
    assert!(!engine.combatants[&hero].budget.action);
}
#[test]
fn empty_ammunition_and_unowned_weapons_are_rejected_without_partial_changes() {
    let (mut engine, hero, npc) = fixture(true, Position { x: 10, y: 0 });
    engine
        .combatants
        .get_mut(&hero)
        .unwrap()
        .loadout
        .as_mut()
        .unwrap()
        .arrows = 0;
    assert!(engine
        .resolve(CombatAction::WeaponAttack {
            attacker: hero,
            target: npc,
            weapon_id: None,
            mode: WeaponMode::Default,
            two_handed: false
        })
        .is_err());
    assert!(engine
        .resolve(CombatAction::WeaponAttack {
            attacker: hero,
            target: npc,
            weapon_id: Some("greataxe".into()),
            mode: WeaponMode::Default,
            two_handed: false
        })
        .is_err());
    assert!(engine.combatants[&hero].budget.action);
}

#[test]
fn low_strength_damage_never_grants_temporary_hit_points() {
    use app_domain::combat::{
        attack::{roll_attack, AttackOutcome},
        conditions::AttackModifier,
    };
    use app_domain::dice::{roll_expr, DiceExpr, Die};
    let seed = (0..100)
        .find(|seed| {
            let mut rng = SeededRng::from_seed(*seed);
            matches!(
                roll_attack(-2, 1, AttackModifier::Normal, &mut rng),
                AttackOutcome::Hit { .. }
            ) && roll_expr(
                &DiceExpr {
                    count: 1,
                    die: Die::D4,
                    modifier: -4,
                },
                &mut rng,
            ) < 0
        })
        .unwrap();
    let (mut engine, hero, npc) = fixture(false, Position { x: 1, y: 0 });
    engine.combatants.get_mut(&hero).unwrap().loadout = Some(
        CombatLoadout::derive(LoadoutInput {
            class_id: "fighter",
            level: 1,
            strength: 3,
            dexterity: 10,
            owned: vec![("club".into(), 1)],
            arrows: 0,
        })
        .unwrap(),
    );
    engine.combatants.get_mut(&npc).unwrap().ac = 1;
    engine.combatants.get_mut(&npc).unwrap().temp_hp = 2;
    engine = CombatResolver::new(engine.combatants, engine.order, SeededRng::from_seed(seed));
    let events = engine
        .resolve(CombatAction::WeaponAttack {
            attacker: hero,
            target: npc,
            weapon_id: None,
            mode: WeaponMode::Default,
            two_handed: false,
        })
        .unwrap();
    assert_eq!(engine.combatants[&npc].temp_hp, 2);
    assert_eq!(engine.combatants[&npc].current_hp, 100);
    assert_eq!(events.damage[0].raw_amount, 0);
}

#[test]
fn versatile_damage_is_not_available_for_a_thrown_attack() {
    let (mut engine, hero, npc) = fixture(false, Position { x: 2, y: 0 });
    engine.combatants.get_mut(&hero).unwrap().loadout = Some(
        CombatLoadout::derive(LoadoutInput {
            class_id: "fighter",
            level: 1,
            strength: 16,
            dexterity: 14,
            owned: vec![("spear".into(), 1)],
            arrows: 0,
        })
        .unwrap(),
    );
    assert!(engine
        .resolve(CombatAction::WeaponAttack {
            attacker: hero,
            target: npc,
            weapon_id: None,
            mode: WeaponMode::Thrown,
            two_handed: true
        })
        .is_err());
    assert!(engine.combatants[&hero].budget.action);
    assert_eq!(
        engine.combatants[&hero]
            .loadout
            .as_ref()
            .unwrap()
            .quantities["spear"],
        1
    );
}
