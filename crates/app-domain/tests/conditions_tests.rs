use app_domain::combat::conditions::{
    AttackModifier, Condition, attacker_condition_modifier, condition_attack_modifier,
};

#[test]
fn prone_melee_attacker_has_advantage() {
    let m = condition_attack_modifier(Condition::Prone, true);
    assert_eq!(m, AttackModifier::Advantage);
}

#[test]
fn prone_ranged_attacker_has_disadvantage() {
    let m = condition_attack_modifier(Condition::Prone, false);
    assert_eq!(m, AttackModifier::Disadvantage);
}

#[test]
fn poisoned_gives_attack_disadvantage() {
    let m = attacker_condition_modifier(Condition::Poisoned);
    assert_eq!(m, AttackModifier::Disadvantage);
}
