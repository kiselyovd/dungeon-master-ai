use app_domain::combat::types::{CombatantId, Position, DamageType, SlotLevel, AbilityScore};

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
