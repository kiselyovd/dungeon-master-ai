use app_domain::combat::defense::{derive, starting_choice, DefenseChoice, DefenseInput};
fn input<'a>(class: &'a str, owned: &'a [(String, u32)], dex: i32) -> DefenseInput<'a> {
    DefenseInput {
        class_id: class,
        strength: 12,
        dexterity: dex,
        constitution: 16,
        wisdom: 16,
        owned,
    }
}
fn choice(armor: Option<&str>, shield: bool) -> DefenseChoice {
    DefenseChoice {
        schema_version: 1,
        armor_id: armor.map(str::to_owned),
        shield,
    }
}
#[test]
fn heavy_armor_ignores_even_negative_dex_and_reports_strength_and_stealth() {
    let owned = vec![("fighter-chain-mail".into(), 1)];
    for dex in [8, 18] {
        let result = derive(
            &input("fighter", &owned, dex),
            &choice(Some("chain-mail"), false),
        )
        .unwrap();
        assert_eq!(result.ac, 16);
        assert_eq!(result.speed_penalty_ft, 10);
        assert!(result.stealth_disadvantage);
    }
}
#[test]
fn medium_caps_positive_dex_but_keeps_negative_and_shields_stack_once() {
    let owned = vec![("chain-shirt".into(), 1), ("shield".into(), 2)];
    assert_eq!(
        derive(
            &input("fighter", &owned, 18),
            &choice(Some("chain-shirt"), true)
        )
        .unwrap()
        .ac,
        17
    );
    assert_eq!(
        derive(
            &input("fighter", &owned, 8),
            &choice(Some("chain-shirt"), false)
        )
        .unwrap()
        .ac,
        12
    );
}
#[test]
fn carrying_armor_is_not_wearing_and_unowned_declarations_fail() {
    let owned = vec![("chain-mail".into(), 1)];
    assert_eq!(
        derive(&input("fighter", &owned, 14), &choice(None, false))
            .unwrap()
            .ac,
        12
    );
    assert!(derive(
        &input("fighter", &[], 14),
        &choice(Some("chain-mail"), false)
    )
    .is_err());
    let untrained = derive(
        &input("wizard", &owned, 14),
        &choice(Some("chain-mail"), false),
    )
    .unwrap();
    assert_eq!(untrained.ac, 16);
    assert!(untrained.untrained_armor);
    assert!(derive(&input("fighter", &owned, 14), &choice(None, true)).is_err());
}
#[test]
fn unarmored_class_defenses_and_starting_kit_recommendations_are_consistent() {
    assert_eq!(
        derive(&input("monk", &[], 14), &choice(None, false))
            .unwrap()
            .ac,
        15
    );
    let owned = vec![("shield".into(), 1)];
    assert_eq!(
        derive(&input("barbarian", &owned, 14), &choice(None, true))
            .unwrap()
            .ac,
        17
    );
    let owned = vec![("chain-mail".into(), 1), ("shield".into(), 1)];
    let result = starting_choice(&input("fighter", &owned, 14)).unwrap();
    assert_eq!(result.ac, 18);
    assert_eq!(result.choice.armor_id.as_deref(), Some("chain-mail"));
}

#[test]
fn untrained_shield_has_no_ac_bonus_and_does_not_block_spells() {
    let owned = vec![("shield".into(), 1)];
    let result = derive(&input("wizard", &owned, 14), &choice(None, true)).unwrap();
    assert_eq!(result.ac, 12);
    assert!(!result.untrained_armor);
    assert!(
        !starting_choice(&input("wizard", &owned, 14))
            .unwrap()
            .choice
            .shield
    );
}
