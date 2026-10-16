use app_domain::combat::combatant::Combatant;
use app_domain::combat::damage::{
    apply_damage_to_combatant, compute_effective_damage, DamageRelation, DamageResistance,
};
use app_domain::combat::types::{CombatantId, DamageType};

fn goblin() -> Combatant {
    Combatant::new(CombatantId::new(), "Goblin".into(), 7, 7, 13)
}

#[test]
fn no_resistance_full_damage() {
    let resist = DamageResistance::default();
    let effective = compute_effective_damage(8, DamageType::Slashing, &resist);
    assert_eq!(effective, 8);
}

#[test]
fn resistance_halves_damage() {
    let mut resist = DamageResistance::default();
    resist.set(DamageType::Fire, DamageRelation::Resistant);
    let effective = compute_effective_damage(8, DamageType::Fire, &resist);
    assert_eq!(effective, 4);
}

#[test]
fn resistance_rounds_down() {
    let mut resist = DamageResistance::default();
    resist.set(DamageType::Fire, DamageRelation::Resistant);
    let effective = compute_effective_damage(7, DamageType::Fire, &resist);
    assert_eq!(effective, 3, "7 / 2 = 3.5 rounds down to 3");
}

#[test]
fn immunity_reduces_to_zero() {
    let mut resist = DamageResistance::default();
    resist.set(DamageType::Poison, DamageRelation::Immune);
    let effective = compute_effective_damage(15, DamageType::Poison, &resist);
    assert_eq!(effective, 0);
}

#[test]
fn vulnerability_doubles_damage() {
    let mut resist = DamageResistance::default();
    resist.set(DamageType::Bludgeoning, DamageRelation::Vulnerable);
    let effective = compute_effective_damage(6, DamageType::Bludgeoning, &resist);
    assert_eq!(effective, 12);
}

#[test]
fn apply_damage_reduces_hp() {
    let mut c = goblin();
    let resist = DamageResistance::default();
    apply_damage_to_combatant(&mut c, 5, DamageType::Slashing, &resist);
    assert_eq!(c.current_hp, 2);
}

#[test]
fn lethal_damage_drops_to_zero() {
    let mut c = goblin();
    let resist = DamageResistance::default();
    apply_damage_to_combatant(&mut c, 100, DamageType::Fire, &resist);
    assert_eq!(c.current_hp, 0);
}

#[test]
fn temp_hp_absorbs_first_via_apply() {
    let mut c = goblin();
    c.temp_hp = 3;
    let resist = DamageResistance::default();
    apply_damage_to_combatant(&mut c, 5, DamageType::Fire, &resist);
    assert_eq!(c.temp_hp, 0);
    assert_eq!(c.current_hp, 5, "only 2 bleed through to HP (7 - 2 = 5)");
}

#[test]
fn damage_at_zero_breaks_stability_and_records_failure_even_with_temporary_hp() {
    let mut c = goblin();
    c.current_hp = 0;
    c.death_saves.stabilize();
    c.temp_hp = 5;
    c.apply_raw_damage(1);
    assert!(!c.death_saves.stable);
    assert_eq!(c.death_saves.failures, 1);
    assert_eq!(c.temp_hp, 4);
    assert!(!c.is_dead);
}
#[test]
fn massive_damage_uses_the_remainder_after_current_and_temporary_hp() {
    for (amount, dead) in [(20, false), (21, true)] {
        let mut c = Combatant::new(CombatantId::new(), "Hero".into(), 12, 6, 14);
        c.temp_hp = 3;
        c.apply_raw_damage(amount);
        assert_eq!(c.current_hp, 0);
        assert_eq!(c.is_dead, dead);
    }
}
#[test]
fn zero_damage_does_not_change_stability_or_saves() {
    let mut c = goblin();
    c.current_hp = 0;
    c.death_saves.stabilize();
    c.apply_raw_damage(0);
    assert!(c.death_saves.stable);
    assert_eq!(c.death_saves.failures, 0);
}

#[test]
fn critical_damage_at_zero_gives_two_failures_and_third_failure_kills() {
    let mut c = goblin();
    c.current_hp = 0;
    c.death_saves.failures = 1;
    c.apply_raw_damage_with_critical(1, true);
    assert_eq!(c.death_saves.failures, 3);
    assert!(c.is_dead);
    c.apply_healing(5);
    assert_eq!(c.current_hp, 0);
}
#[test]
fn immunity_does_not_restart_death_saves_and_zero_hp_massive_damage_kills() {
    let mut c = goblin();
    c.current_hp = 0;
    c.death_saves.stabilize();
    let mut resist = DamageResistance::default();
    resist.set(DamageType::Fire, DamageRelation::Immune);
    apply_damage_to_combatant(&mut c, 100, DamageType::Fire, &resist);
    assert!(c.death_saves.stable);
    assert!(!c.is_dead);
    c.apply_raw_damage(7);
    assert!(c.is_dead);
}
