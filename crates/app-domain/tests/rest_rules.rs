use app_domain::{
    combat::magic::slot_capacity,
    rest::{resolve_rest, RestInput, RestIntent, RestResources},
    rng::SeededRng,
};
use std::collections::BTreeMap;

fn input(class: &str) -> RestInput {
    RestInput {
        class_id: class.into(),
        level: 1,
        hp: 2,
        max_hp: 20,
        constitution_modifier: 2,
        elapsed_seconds: 0,
        resources: RestResources {
            hit_dice_remaining: 1,
            last_long_rest_completed: None,
            spell_slots: BTreeMap::from([(1, 0)]),
        },
        safe: true,
        combat_active: false,
    }
}

#[test]
fn short_rest_spends_a_class_die_and_advances_one_hour() {
    let result = resolve_rest(
        input("cleric"),
        RestIntent::Short { hit_dice: 1 },
        &mut SeededRng::from_seed(42),
    )
    .unwrap();
    assert!(result.hp > 2);
    assert_eq!(result.elapsed_seconds, 3600);
    assert_eq!(result.resources.hit_dice_remaining, 0);
    assert_eq!(result.resources.spell_slots[&1], 0);
    assert_eq!(result.rolls.len(), 1);
}
#[test]
fn short_rest_has_a_one_hp_minimum_even_with_negative_constitution() {
    let mut state = input("wizard");
    state.constitution_modifier = -10;
    let result = resolve_rest(
        state,
        RestIntent::Short { hit_dice: 1 },
        &mut SeededRng::from_seed(42),
    )
    .unwrap();
    assert_eq!(result.hp, 3);
}
#[test]
fn long_rest_restores_resources_and_enforces_sixteen_hours_before_another_start() {
    let result = resolve_rest(
        input("cleric"),
        RestIntent::Long,
        &mut SeededRng::from_seed(42),
    )
    .unwrap();
    assert_eq!(result.hp, 20);
    assert_eq!(result.elapsed_seconds, 28800);
    assert_eq!(result.resources.spell_slots[&1], 2);
    assert_eq!(result.resources.hit_dice_remaining, 1);
    let mut second = input("cleric");
    second.elapsed_seconds = result.elapsed_seconds;
    second.resources = result.resources;
    assert!(resolve_rest(
        second.clone(),
        RestIntent::Long,
        &mut SeededRng::from_seed(42)
    )
    .is_err());
    let waited = resolve_rest(
        second.clone(),
        RestIntent::Wait { minutes: 960 },
        &mut SeededRng::from_seed(42),
    )
    .unwrap();
    second.elapsed_seconds = waited.elapsed_seconds;
    assert!(resolve_rest(second, RestIntent::Long, &mut SeededRng::from_seed(42)).is_ok());
}
#[test]
fn warlock_short_rest_restores_pact_slots_at_their_actual_level() {
    let mut state = input("warlock");
    state.level = 5;
    state.resources.spell_slots = BTreeMap::from([(3, 0)]);
    let result = resolve_rest(
        state,
        RestIntent::Short { hit_dice: 0 },
        &mut SeededRng::from_seed(42),
    )
    .unwrap();
    assert_eq!(result.resources.spell_slots[&3], 2);
    assert!(!result.resources.spell_slots.contains_key(&1));
}
#[test]
fn unsafe_combat_downed_or_overspent_rests_are_rejected() {
    for mutate in 0..4 {
        let mut state = input("fighter");
        match mutate {
            0 => state.safe = false,
            1 => state.combat_active = true,
            2 => state.hp = 0,
            _ => state.resources.hit_dice_remaining = 0,
        };
        assert!(resolve_rest(
            state,
            RestIntent::Short { hit_dice: 1 },
            &mut SeededRng::from_seed(42)
        )
        .is_err());
    }
}
#[test]
fn class_slot_tables_cover_full_half_and_pact_progression() {
    assert_eq!(slot_capacity("wizard", 3), BTreeMap::from([(1, 4), (2, 2)]));
    assert_eq!(slot_capacity("paladin", 1), BTreeMap::from([(1, 2)]));
    assert_eq!(slot_capacity("ranger", 5), BTreeMap::from([(1, 4), (2, 2)]));
    assert_eq!(slot_capacity("warlock", 17), BTreeMap::from([(5, 4)]));
    assert_eq!(slot_capacity("wizard", 20)[&7], 2);
    assert!(slot_capacity("fighter", 20).is_empty());
}
