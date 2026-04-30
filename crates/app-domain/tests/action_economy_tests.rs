use app_domain::combat::action_economy::{ActionEconomyError, consume_action};
use app_domain::combat::combatant::ActionBudget;
use app_domain::combat::types::ActionKind;

#[test]
fn consume_action_succeeds_when_available() {
    let mut budget = ActionBudget::fresh(30);
    consume_action(&mut budget, ActionKind::Action).expect("action available");
    assert!(!budget.action);
}

#[test]
fn consume_action_fails_when_exhausted() {
    let mut budget = ActionBudget::fresh(30);
    budget.action = false;
    let err = consume_action(&mut budget, ActionKind::Action).unwrap_err();
    assert!(matches!(err, ActionEconomyError::ActionExhausted));
}

#[test]
fn consume_bonus_action_independently() {
    let mut budget = ActionBudget::fresh(30);
    consume_action(&mut budget, ActionKind::BonusAction).expect("bonus available");
    assert!(!budget.bonus_action);
    assert!(budget.action, "action still available");
}

#[test]
fn consume_movement_decrements_budget() {
    let mut budget = ActionBudget::fresh(30);
    consume_action(&mut budget, ActionKind::Movement).expect("movement ok");
    assert_eq!(budget.movement_ft, 25, "5ft cell consumed");
}

#[test]
fn refresh_restores_all_kinds() {
    let mut budget = ActionBudget::fresh(30);
    budget.action = false;
    budget.bonus_action = false;
    budget.reaction = false;
    budget.movement_ft = 0;
    budget.refresh_for_new_turn(30);
    assert!(budget.action);
    assert!(budget.bonus_action);
    assert!(budget.reaction);
    assert_eq!(budget.movement_ft, 30);
}
