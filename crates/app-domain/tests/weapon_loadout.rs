use app_domain::combat::loadout::{CombatLoadout, LoadoutInput};
use app_domain::combat::types::DamageType;

#[test]
fn weapons_derive_strength_finesse_proficiency_and_ranges() {
    let fighter = CombatLoadout::derive(LoadoutInput {
        class_id: "fighter",
        level: 1,
        strength: 16,
        dexterity: 14,
        owned: vec![("fighter-longsword".into(), 1)],
        arrows: 0,
    })
    .unwrap();
    let sword = fighter
        .attacks
        .iter()
        .find(|a| a.weapon_id == "longsword")
        .unwrap();
    assert_eq!(sword.attack_bonus, 5);
    assert_eq!(sword.damage.modifier, 3);
    assert_eq!(sword.damage_type, DamageType::Slashing);
    let rogue = CombatLoadout::derive(LoadoutInput {
        class_id: "rogue",
        level: 1,
        strength: 10,
        dexterity: 16,
        owned: vec![("dagger".into(), 2), ("shortbow".into(), 1)],
        arrows: 20,
    })
    .unwrap();
    let dagger = rogue
        .attacks
        .iter()
        .find(|a| a.weapon_id == "dagger")
        .unwrap();
    assert_eq!(dagger.attack_bonus, 5);
    assert_eq!(dagger.damage.modifier, 3);
    let bow = rogue
        .attacks
        .iter()
        .find(|a| a.weapon_id == "shortbow")
        .unwrap();
    assert_eq!((bow.normal_range_ft, bow.long_range_ft), (80, 320));
    assert!(bow.ammunition);
}

#[test]
fn an_untrained_weapon_does_not_gain_the_class_proficiency_bonus() {
    let wizard = CombatLoadout::derive(LoadoutInput {
        class_id: "wizard",
        level: 1,
        strength: 10,
        dexterity: 16,
        owned: vec![("longsword".into(), 1)],
        arrows: 0,
    })
    .unwrap();
    assert_eq!(
        wizard
            .attacks
            .iter()
            .find(|a| a.weapon_id == "longsword")
            .unwrap()
            .attack_bonus,
        0
    );
    assert!(wizard.attacks.iter().any(|a| a.weapon_id == "unarmed"));
}
