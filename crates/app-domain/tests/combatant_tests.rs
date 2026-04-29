use app_domain::combat::combatant::Combatant;
use app_domain::combat::conditions::Condition;
use app_domain::combat::types::{AbilityScore, CombatantId, DamageType, Position, SlotLevel};

#[test]
fn combatant_id_is_uuid() {
    let id1 = CombatantId::new();
    let id2 = CombatantId::new();
    assert_ne!(id1, id2, "UUIDs must be unique");
}

#[test]
fn position_equality() {
    let a = Position { x: 3, y: 5 };
    let b = Position { x: 3, y: 5 };
    assert_eq!(a, b);
}

#[test]
fn damage_type_serde_roundtrip() {
    let t = DamageType::Fire;
    let json = serde_json::to_string(&t).expect("serialize");
    let back: DamageType = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(t, back);
}

#[test]
fn slot_level_validates_range() {
    assert!(SlotLevel::new(1).is_some());
    assert!(SlotLevel::new(9).is_some());
    assert!(SlotLevel::new(0).is_none());
    assert!(SlotLevel::new(10).is_none());
}

#[test]
fn ability_score_serde_roundtrip() {
    let s = AbilityScore::Wis;
    let json = serde_json::to_string(&s).expect("serialize");
    let back: AbilityScore = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(s, back);
}

#[test]
fn position_distance_chebyshev() {
    let a = Position { x: 0, y: 0 };
    let b = Position { x: 3, y: 4 };
    assert_eq!(a.chebyshev(b), 4);
    assert_eq!(a.distance_ft(b), 20);
}

#[test]
fn hp_clamp_never_below_zero() {
    let id = CombatantId::new();
    let mut c = Combatant::new(id, "Goblin".into(), 7, 7, 13);
    c.apply_raw_damage(100);
    assert_eq!(c.current_hp, 0);
}

#[test]
fn hp_clamp_never_above_max() {
    let id = CombatantId::new();
    let mut c = Combatant::new(id, "Cleric".into(), 10, 10, 14);
    c.apply_healing(50);
    assert_eq!(c.current_hp, 10);
}

#[test]
fn temp_hp_absorbs_damage_first() {
    let id = CombatantId::new();
    let mut c = Combatant::new(id, "Fighter".into(), 12, 12, 16);
    c.temp_hp = 5;
    c.apply_raw_damage(8);
    assert_eq!(c.temp_hp, 0);
    assert_eq!(c.current_hp, 9, "3 damage bleeds through to HP");
}

#[test]
fn add_and_remove_condition() {
    let id = CombatantId::new();
    let mut c = Combatant::new(id, "Rogue".into(), 9, 9, 12);
    c.add_condition(Condition::Prone);
    assert!(c.has_condition(Condition::Prone));
    c.remove_condition(Condition::Prone);
    assert!(!c.has_condition(Condition::Prone));
}

#[test]
fn unconscious_implies_incapacitated() {
    let id = CombatantId::new();
    let mut c = Combatant::new(id, "Wizard".into(), 8, 8, 11);
    c.add_condition(Condition::Unconscious);
    assert!(c.is_incapacitated(), "unconscious is incapacitated");
}
