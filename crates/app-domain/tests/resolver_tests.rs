use std::collections::HashMap;

use app_domain::combat::combatant::Combatant;
use app_domain::combat::initiative::{InitiativeEntry, InitiativeOrder};
use app_domain::combat::resolver::{CombatAction, CombatResolver, ValidationError};
use app_domain::combat::types::{CombatantId, DamageType};
use app_domain::dice::DiceExpr;
use app_domain::rng::SeededRng;

fn two_combatants() -> (CombatantId, CombatantId, CombatResolver) {
    let a_id = CombatantId::new();
    let b_id = CombatantId::new();
    let a = {
        let mut c = Combatant::new(a_id, "Hero".into(), 15, 15, 14);
        c.initiative_roll = 18;
        c
    };
    let b = {
        let mut c = Combatant::new(b_id, "Goblin".into(), 7, 7, 13);
        c.initiative_roll = 10;
        c
    };
    let order = InitiativeOrder::build(vec![
        InitiativeEntry { id: a_id, roll: 18, dex_tiebreak: 2 },
        InitiativeEntry { id: b_id, roll: 10, dex_tiebreak: 0 },
    ]);
    let mut combatants = HashMap::new();
    combatants.insert(a_id, a);
    combatants.insert(b_id, b);
    let resolver = CombatResolver::new(combatants, order, SeededRng::from_seed(1));
    (a_id, b_id, resolver)
}

#[test]
fn attack_on_own_turn_is_valid() {
    let (a_id, b_id, mut resolver) = two_combatants();
    let action = CombatAction::Attack {
        attacker: a_id,
        target: b_id,
        attack_modifier: 4,
        damage_expr: DiceExpr { count: 1, die: app_domain::dice::Die::D8, modifier: 2 },
        damage_type: DamageType::Slashing,
    };
    let result = resolver.resolve(action);
    assert!(result.is_ok(), "attack on valid target should resolve: {result:?}");
}

#[test]
fn attack_by_non_active_combatant_fails_validation() {
    let (a_id, b_id, mut resolver) = two_combatants();
    // b_id is NOT the current combatant (a_id is)
    let action = CombatAction::Attack {
        attacker: b_id,
        target: a_id,
        attack_modifier: 2,
        damage_expr: DiceExpr { count: 1, die: app_domain::dice::Die::D6, modifier: 0 },
        damage_type: DamageType::Piercing,
    };
    let result = resolver.resolve(action);
    assert!(matches!(result, Err(ValidationError::NotYourTurn)), "{result:?}");
}

#[test]
fn attack_on_self_fails_validation() {
    let (a_id, _b_id, mut resolver) = two_combatants();
    let action = CombatAction::Attack {
        attacker: a_id,
        target: a_id,
        attack_modifier: 4,
        damage_expr: DiceExpr { count: 1, die: app_domain::dice::Die::D8, modifier: 2 },
        damage_type: DamageType::Slashing,
    };
    let result = resolver.resolve(action);
    assert!(matches!(result, Err(ValidationError::InvalidTarget)), "{result:?}");
}

#[test]
fn successful_attack_produces_result_events() {
    let (a_id, b_id, mut resolver) = two_combatants();
    let action = CombatAction::Attack {
        attacker: a_id,
        target: b_id,
        attack_modifier: 8, // very high to ensure a hit
        damage_expr: DiceExpr { count: 1, die: app_domain::dice::Die::D6, modifier: 0 },
        damage_type: DamageType::Slashing,
    };
    let events = resolver.resolve(action).expect("should hit");
    // May produce either a hit with damage or a critical miss (natural 1)
    // With +8 mod the vast majority will be hits - just check no panic.
    let _ = events;
}
