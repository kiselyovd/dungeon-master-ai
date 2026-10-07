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

fn seed_for(face: i32) -> u64 {
    (0..10000)
        .find(|seed| {
            app_domain::dice::roll_one(app_domain::dice::Die::D20, &mut SeededRng::from_seed(*seed))
                == face
        })
        .unwrap()
}
#[test]
fn natural_twenty_death_save_restores_one_hit_point() {
    let mut c = downed();
    c.death_saves.failures = 2;
    assert!(roll_death_save(
        &mut c,
        &mut SeededRng::from_seed(seed_for(20))
    ));
    assert_eq!(c.current_hp, 1);
    assert_eq!(c.death_saves.failures, 0);
    assert!(!c.has_condition(Condition::Unconscious));
}
#[test]
fn natural_one_death_save_records_two_failures_and_can_kill() {
    let mut c = downed();
    c.death_saves.failures = 1;
    assert!(!roll_death_save(
        &mut c,
        &mut SeededRng::from_seed(seed_for(1))
    ));
    assert_eq!(c.death_saves.failures, 3);
    assert!(c.is_dead);
}
#[test]
fn stable_and_dead_heroes_do_not_roll_or_advance_randomness() {
    for dead in [false, true] {
        let mut c = downed();
        c.is_dead = dead;
        c.death_saves.stable = !dead;
        let mut actual = SeededRng::from_seed(seed_for(9));
        let mut expected = SeededRng::from_seed(seed_for(9));
        assert!(!roll_death_save(&mut c, &mut actual));
        assert_eq!(c.death_saves.failures, 0);
        assert_eq!(
            app_domain::dice::roll_one(app_domain::dice::Die::D20, &mut actual),
            app_domain::dice::roll_one(app_domain::dice::Die::D20, &mut expected)
        );
    }
}

#[test]
fn third_success_stabilizes_and_resets_both_counters() {
    let mut c = downed();
    c.death_saves.successes = 2;
    c.death_saves.failures = 2;
    assert!(roll_death_save(
        &mut c,
        &mut SeededRng::from_seed(seed_for(10))
    ));
    assert!(c.death_saves.stable);
    assert_eq!(c.current_hp, 0);
    assert_eq!(c.death_saves.successes, 0);
    assert_eq!(c.death_saves.failures, 0);
    assert!(c.has_condition(Condition::Unconscious));
}
#[test]
fn medicine_stabilizes_at_dc_ten_then_natural_recovery_waits_for_its_deadline() {
    use app_domain::combat::healing::{
        begin_natural_recovery, complete_natural_recovery, stabilize_with_check,
    };
    let mut c = downed();
    c.death_saves.failures = 2;
    assert!(!stabilize_with_check(&mut c, 9));
    assert_eq!(c.death_saves.failures, 2);
    assert!(stabilize_with_check(&mut c, 10));
    assert_eq!(c.current_hp, 0);
    assert_eq!(c.death_saves.failures, 0);
    assert!(c.has_condition(Condition::Unconscious));
    let deadline = begin_natural_recovery(&c, 100, &mut SeededRng::from_seed(4)).unwrap();
    assert!([3600, 7200, 10800, 14400].contains(&(deadline.ready_at_seconds - 100)));
    let serialized = serde_json::to_string(&deadline).unwrap();
    let restored = serde_json::from_str(&serialized).unwrap();
    assert!(!complete_natural_recovery(&mut c, &restored, deadline.ready_at_seconds - 1).unwrap());
    assert!(complete_natural_recovery(&mut c, &restored, deadline.ready_at_seconds).unwrap());
    assert_eq!(c.current_hp, 1);
    assert!(!c.has_condition(Condition::Unconscious));
    assert!(!complete_natural_recovery(&mut c, &restored, deadline.ready_at_seconds).unwrap());
}
#[test]
fn recovery_rejects_dead_unstable_or_other_creatures_and_invalid_clocks() {
    use app_domain::combat::healing::{
        begin_natural_recovery, complete_natural_recovery, stabilize_with_check, RecoveryError,
    };
    let mut c = downed();
    let mut rng = SeededRng::from_seed(7);
    let mut control = SeededRng::from_seed(7);
    assert_eq!(
        begin_natural_recovery(&c, 0, &mut rng),
        Err(RecoveryError::Ineligible)
    );
    stabilize_with_check(&mut c, 10);
    assert_eq!(
        begin_natural_recovery(&c, i64::MAX as u64, &mut rng),
        Err(RecoveryError::InvalidDeadline)
    );
    assert_eq!(
        app_domain::dice::roll_one(app_domain::dice::Die::D4, &mut rng),
        app_domain::dice::roll_one(app_domain::dice::Die::D4, &mut control)
    );
    let deadline = begin_natural_recovery(&c, 0, &mut rng).unwrap();
    let mut other = downed();
    stabilize_with_check(&mut other, 10);
    assert_eq!(
        complete_natural_recovery(&mut other, &deadline, deadline.ready_at_seconds),
        Err(RecoveryError::InvalidDeadline)
    );
    c.is_dead = true;
    assert!(!complete_natural_recovery(&mut c, &deadline, deadline.ready_at_seconds).unwrap());
    assert_eq!(c.current_hp, 0);
    c.is_dead = false;
    c.death_saves.stable = false;
    assert!(!complete_natural_recovery(&mut c, &deadline, deadline.ready_at_seconds).unwrap());
    c.death_saves.stable = true;
    let mut invalid = deadline.clone();
    invalid.ready_at_seconds = 1;
    assert_eq!(
        complete_natural_recovery(&mut c, &invalid, 1),
        Err(RecoveryError::InvalidDeadline)
    );
}
