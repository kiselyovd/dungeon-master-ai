use app_domain::dice::{Die, DiceExpr, roll_expr};
use app_domain::rng::SeededRng;

#[test]
fn d20_result_is_in_range() {
    let mut rng = SeededRng::from_seed(42);
    for _ in 0..200 {
        let result = roll_expr(&DiceExpr { count: 1, die: Die::D20, modifier: 0 }, &mut rng);
        assert!(result >= 1 && result <= 20, "d20 out of range: {result}");
    }
}

#[test]
fn seed_determinism_same_sequence() {
    let expr = DiceExpr { count: 2, die: Die::D6, modifier: 3 };
    let mut rng1 = SeededRng::from_seed(999);
    let mut rng2 = SeededRng::from_seed(999);
    let a = roll_expr(&expr, &mut rng1);
    let b = roll_expr(&expr, &mut rng2);
    assert_eq!(a, b, "same seed must produce same roll");
}

#[test]
fn two_d6_plus_3_range() {
    let expr = DiceExpr { count: 2, die: Die::D6, modifier: 3 };
    let mut rng = SeededRng::from_seed(7);
    for _ in 0..500 {
        let result = roll_expr(&expr, &mut rng);
        assert!(result >= 5 && result <= 15, "2d6+3 out of range: {result}");
    }
}

#[test]
fn individual_rolls_tracked() {
    use app_domain::dice::roll_expr_detailed;
    let expr = DiceExpr { count: 3, die: Die::D6, modifier: 0 };
    let mut rng = SeededRng::from_seed(1);
    let detail = roll_expr_detailed(&expr, &mut rng);
    assert_eq!(detail.rolls.len(), 3);
    for r in &detail.rolls {
        assert!(*r >= 1 && *r <= 6);
    }
    assert_eq!(detail.total, detail.rolls.iter().sum::<i32>());
}

#[test]
fn advantage_takes_higher() {
    use app_domain::dice::roll_with_advantage;
    let mut rng = SeededRng::from_seed(55);
    for _ in 0..200 {
        let (total, rolls) = roll_with_advantage(Die::D20, 0, &mut rng);
        assert_eq!(total, *rolls.iter().max().unwrap());
    }
}

#[test]
fn disadvantage_takes_lower() {
    use app_domain::dice::roll_with_disadvantage;
    let mut rng = SeededRng::from_seed(55);
    for _ in 0..200 {
        let (total, rolls) = roll_with_disadvantage(Die::D20, 0, &mut rng);
        assert_eq!(total, *rolls.iter().min().unwrap());
    }
}
