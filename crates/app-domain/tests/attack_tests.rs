use app_domain::combat::attack::{AttackOutcome, roll_attack};
use app_domain::combat::conditions::AttackModifier;
use app_domain::rng::SeededRng;

#[test]
fn natural_20_is_critical_hit() {
    let mut found = false;
    for seed in 0..1000u64 {
        let mut rng = SeededRng::from_seed(seed);
        let outcome = roll_attack(0, 30, AttackModifier::Normal, &mut rng);
        if let AttackOutcome::CriticalHit { natural, .. } = outcome {
            assert_eq!(natural, 20);
            found = true;
            break;
        }
    }
    assert!(found, "could not find a natural 20 in first 1000 seeds");
}

#[test]
fn natural_1_is_critical_miss() {
    let mut found = false;
    for seed in 0..1000u64 {
        let mut rng = SeededRng::from_seed(seed);
        let outcome = roll_attack(100, 0, AttackModifier::Normal, &mut rng);
        if let AttackOutcome::CriticalMiss { natural } = outcome {
            assert_eq!(natural, 1);
            found = true;
            break;
        }
    }
    assert!(found, "could not find a natural 1 in first 1000 seeds");
}

#[test]
fn hit_when_total_meets_ac() {
    let mut rng = SeededRng::from_seed(100);
    let mut hits = 0i32;
    for _ in 0..50 {
        match roll_attack(5, 10, AttackModifier::Normal, &mut rng) {
            AttackOutcome::Hit { .. } | AttackOutcome::CriticalHit { .. } => hits += 1,
            _ => {}
        }
    }
    assert!(hits > 30, "expected majority hits with +5 vs AC10, got {hits}");
}

#[test]
fn advantage_increases_hit_rate() {
    let mut rng_normal = SeededRng::from_seed(42);
    let mut rng_adv = SeededRng::from_seed(42);
    let trials = 100;
    let mut normal_hits = 0i32;
    let mut adv_hits = 0i32;
    for _ in 0..trials {
        if matches!(
            roll_attack(0, 15, AttackModifier::Normal, &mut rng_normal),
            AttackOutcome::Hit { .. } | AttackOutcome::CriticalHit { .. }
        ) {
            normal_hits += 1;
        }
        if matches!(
            roll_attack(0, 15, AttackModifier::Advantage, &mut rng_adv),
            AttackOutcome::Hit { .. } | AttackOutcome::CriticalHit { .. }
        ) {
            adv_hits += 1;
        }
    }
    assert!(adv_hits >= normal_hits, "advantage should not decrease hit rate");
}
