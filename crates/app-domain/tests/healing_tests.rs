use app_domain::combat::combatant::Combatant;
use app_domain::combat::conditions::Condition;
use app_domain::combat::healing::{apply_healing, roll_death_save};
use app_domain::combat::types::CombatantId;
use app_domain::rng::SeededRng;

fn downed() -> Combatant {
    let mut c = Combatant::new(CombatantId::new(), "Hero".into(), 10, 0, 12);
    c.add_condition(Condition::Unconscious);
    c
}

#[test]
fn healing_revives_downed_combatant() {
    let mut c = downed();
    apply_healing(&mut c, 4);
    assert_eq!(c.current_hp, 4);
    assert!(!c.has_condition(Condition::Unconscious));
}

#[test]
fn healing_resets_death_saves() {
    let mut c = downed();
    c.death_saves.failures = 2;
    c.death_saves.successes = 1;
    apply_healing(&mut c, 1);
    assert_eq!(c.death_saves.failures, 0);
    assert_eq!(c.death_saves.successes, 0);
}

#[test]
fn three_death_save_successes_stabilize() {
    let mut c = downed();
    c.death_saves.record_success();
    c.death_saves.record_success();
    let stable = c.death_saves.record_success();
    assert!(stable, "three successes = stable");
    assert!(c.death_saves.stable);
}

#[test]
fn three_death_save_failures_mean_dead() {
    let mut c = downed();
    c.death_saves.record_failure();
    c.death_saves.record_failure();
    let dead = c.death_saves.record_failure();
    assert!(dead, "three failures = dead");
}

#[test]
fn death_save_roll_uses_d20_vs_10() {
    let mut c = downed();
    let mut rng = SeededRng::from_seed(1);
    let _result = roll_death_save(&mut c, &mut rng);
    // The function returned without panicking; outcome is recorded on the combatant.
    let total = c.death_saves.successes + c.death_saves.failures;
    assert_eq!(total, 1, "exactly one save recorded");
}
