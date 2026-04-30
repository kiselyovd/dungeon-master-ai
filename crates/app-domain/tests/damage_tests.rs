use app_domain::combat::combatant::Combatant;
use app_domain::combat::damage::{
    DamageRelation, DamageResistance, apply_damage_to_combatant, compute_effective_damage,
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
