use app_domain::combat::initiative::{InitiativeEntry, InitiativeOrder};
use app_domain::combat::types::CombatantId;

fn make_entry(id: CombatantId, roll: i32, dex: i32) -> InitiativeEntry {
    InitiativeEntry { id, roll, dex_tiebreak: dex }
}

#[test]
fn order_sorts_highest_first() {
    let a = CombatantId::new();
    let b = CombatantId::new();
    let c = CombatantId::new();
    let order = InitiativeOrder::build(vec![
        make_entry(a, 12, 2),
        make_entry(b, 18, 1),
        make_entry(c, 15, 3),
    ]);
    let ids: Vec<_> = order.as_slice().iter().map(|e| e.id).collect();
    assert_eq!(ids[0], b);
    assert_eq!(ids[1], c);
    assert_eq!(ids[2], a);
}

#[test]
fn tie_broken_by_dex() {
    let a = CombatantId::new();
    let b = CombatantId::new();
    let order = InitiativeOrder::build(vec![
        make_entry(a, 15, 1),
        make_entry(b, 15, 3),
    ]);
    assert_eq!(order.as_slice()[0].id, b, "higher DEX wins tie");
}

#[test]
fn current_starts_at_index_zero() {
    let a = CombatantId::new();
    let order = InitiativeOrder::build(vec![make_entry(a, 10, 0)]);
    assert_eq!(order.current().id, a);
}

#[test]
fn advance_wraps_and_increments_round() {
    let a = CombatantId::new();
    let b = CombatantId::new();
    let mut order = InitiativeOrder::build(vec![
        make_entry(a, 20, 0),
        make_entry(b, 5, 0),
    ]);
    assert_eq!(order.round(), 1);
    order.advance();
    assert_eq!(order.current().id, b);
    assert_eq!(order.round(), 1);
    order.advance();
    assert_eq!(order.current().id, a);
    assert_eq!(order.round(), 2);
}

#[test]
fn remove_combatant_skips_correctly() {
    let a = CombatantId::new();
    let b = CombatantId::new();
    let c = CombatantId::new();
    let mut order = InitiativeOrder::build(vec![
        make_entry(a, 20, 0),
        make_entry(b, 15, 0),
        make_entry(c, 5, 0),
    ]);
    order.remove(b);
    assert_eq!(order.len(), 2);
    order.advance();
    assert_eq!(order.current().id, c);
}
