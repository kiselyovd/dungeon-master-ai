use app_domain::combat::types::DamageType;
use app_domain::compendium::character_catalog;

#[test]
fn equipment_attack_data_has_real_damage_types_ranges_and_versatile_values() {
    let catalog = character_catalog();
    for weapon in &catalog.equipment.weapons {
        serde_json::from_value::<DamageType>(serde_json::json!(weapon.damage.damage_type))
            .unwrap_or_else(|_| panic!("unsupported damage type for {}", weapon.id));
    }
    for (id, normal, long) in [
        ("shortbow", 80, 320),
        ("longbow", 150, 600),
        ("dagger", 20, 60),
        ("javelin", 30, 120),
    ] {
        let weapon = catalog
            .equipment
            .weapons
            .iter()
            .find(|w| w.id == id)
            .unwrap();
        assert_eq!(
            (weapon.range_ft.normal, weapon.range_ft.long),
            (Some(normal), Some(long)),
            "{id}"
        );
    }
    assert_eq!(
        catalog
            .equipment
            .weapons
            .iter()
            .find(|w| w.id == "longsword")
            .unwrap()
            .damage
            .versatile_dice
            .as_deref(),
        Some("1d10")
    );
    let chain = catalog
        .equipment
        .armor
        .iter()
        .find(|a| a.id == "chain-shirt")
        .unwrap();
    assert_eq!(chain.ac_dex_bonus_cap, Some(2));
    let heavy = catalog
        .equipment
        .armor
        .iter()
        .find(|a| a.id == "chain-mail")
        .unwrap();
    assert_eq!(heavy.ac_dex_bonus_cap, Some(0));
    assert_eq!(heavy.str_req, Some(13));
}
