use app_domain::combat::ability_check::{CheckOutcome, roll_ability_check};
use app_domain::combat::saving_throw::{SaveOutcome, roll_save};
use app_domain::rng::SeededRng;

#[test]
fn save_succeeds_when_total_meets_dc() {
    // save_modifier = 5, DC = 10 -> need d20 >= 5 -> very likely
    let mut hits = 0u32;
    let mut rng = SeededRng::from_seed(1);
    for _ in 0..100 {
        if matches!(roll_save(5, 10, &mut rng), SaveOutcome::Success { .. }) {
            hits += 1;
        }
    }
    assert!(hits > 70, "with +5 vs DC10 expected >70% success, got {hits}%");
}

#[test]
fn save_fails_when_total_misses_dc() {
    let mut fails = 0u32;
    let mut rng = SeededRng::from_seed(1);
    for _ in 0..100 {
        if matches!(roll_save(-5, 20, &mut rng), SaveOutcome::Failure { .. }) {
            fails += 1;
        }
    }
    assert!(fails > 70, "with -5 vs DC20 expected >70% failure, got {fails}%");
}

#[test]
fn concentration_check_dc_is_max_of_10_and_half_damage() {
    use app_domain::combat::saving_throw::concentration_check_dc;
    assert_eq!(concentration_check_dc(8), 10, "damage 8 -> DC max(10, 4) = 10");
    assert_eq!(concentration_check_dc(25), 12, "damage 25 -> DC max(10, 12) = 12");
}

#[test]
fn ability_check_succeeds_above_dc() {
    let mut rng = SeededRng::from_seed(99);
    let mut pass = 0u32;
    for _ in 0..100 {
        match roll_ability_check(4, 8, &mut rng) {
            CheckOutcome::Success { .. } | CheckOutcome::CriticalSuccess { .. } => pass += 1,
            _ => {}
        }
    }
    assert!(pass > 60, "+4 vs DC8 should pass >60%, got {pass}");
}
