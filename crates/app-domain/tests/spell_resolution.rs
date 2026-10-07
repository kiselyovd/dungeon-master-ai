use app_domain::combat::{
    combatant::Combatant,
    initiative::{InitiativeEntry, InitiativeOrder},
    magic::{CastingProfile, SpellSlots},
    resolver::{CombatAction, CombatResolver},
    types::{CharacterId, CombatantId, Position},
};
use app_domain::rng::SeededRng;
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

fn fixture() -> (CombatResolver, CombatantId, CombatantId) {
    let hero = CombatantId::new();
    let enemy = CombatantId::new();
    let mut actor = Combatant::new(hero, "Cleric".into(), 20, 2, 16);
    actor.character_id = Some(CharacterId(Uuid::new_v4()));
    actor.casting = Some(CastingProfile {
        choices: vec![],
        level: 1,
        ability_modifier: 3,
        attack_bonus: 5,
        save_dc: 13,
        prepared: vec![
            "cure-wounds".into(),
            "healing-word".into(),
            "sacred-flame".into(),
            "magic-missile".into(),
        ],
        slots: BTreeMap::from([(
            1,
            SpellSlots {
                maximum: 2,
                remaining: 2,
            },
        )]),
    });
    let mut foe = Combatant::new(enemy, "Foe".into(), 100, 100, 10);
    foe.position = Position { x: 1, y: 0 };
    let order = InitiativeOrder::build(vec![
        InitiativeEntry {
            id: hero,
            roll: 20,
            dex_tiebreak: 0,
        },
        InitiativeEntry {
            id: enemy,
            roll: 10,
            dex_tiebreak: 0,
        },
    ]);
    (
        CombatResolver::new(
            HashMap::from([(hero, actor), (enemy, foe)]),
            order,
            SeededRng::from_seed(42),
        ),
        hero,
        enemy,
    )
}

#[test]
fn cure_wounds_heals_with_two_dice_and_spends_a_slot() {
    let (mut engine, hero, _) = fixture();
    let events = engine
        .resolve(CombatAction::Spell {
            caster: hero,
            spell_id: "cure-wounds".into(),
            targets: vec![hero],
            slot_level: 1,
        })
        .unwrap();
    assert!(engine.combatants[&hero].current_hp > 2);
    assert_eq!(events.rolls[0].detail.rolls.len(), 2);
    assert_eq!(events.healing.len(), 1);
    assert_eq!(events.spell_slots_consumed.len(), 1);
    assert_eq!(
        engine.combatants[&hero].casting.as_ref().unwrap().slots[&1].remaining,
        1
    );
    assert!(!engine.combatants[&hero].budget.action);
}

#[test]
fn healing_word_preserves_action_but_blocks_a_second_slotted_spell() {
    let (mut engine, hero, enemy) = fixture();
    engine
        .resolve(CombatAction::Spell {
            caster: hero,
            spell_id: "healing-word".into(),
            targets: vec![hero],
            slot_level: 1,
        })
        .unwrap();
    assert!(engine.combatants[&hero].budget.action);
    assert!(!engine.combatants[&hero].budget.bonus_action);
    assert!(engine
        .resolve(CombatAction::Spell {
            caster: hero,
            spell_id: "magic-missile".into(),
            targets: vec![enemy],
            slot_level: 1
        })
        .is_err());
    engine
        .resolve(CombatAction::Spell {
            caster: hero,
            spell_id: "sacred-flame".into(),
            targets: vec![enemy],
            slot_level: 0,
        })
        .unwrap();
    assert_eq!(
        engine.combatants[&hero].casting.as_ref().unwrap().slots[&1].remaining,
        1
    );
}

#[test]
fn invalid_range_or_unprepared_spell_spends_nothing() {
    let (mut engine, hero, enemy) = fixture();
    engine.combatants.get_mut(&enemy).unwrap().position = Position { x: 30, y: 0 };
    for spell in ["magic-missile", "fire-bolt"] {
        assert!(engine
            .resolve(CombatAction::Spell {
                caster: hero,
                spell_id: spell.into(),
                targets: vec![enemy],
                slot_level: 1
            })
            .is_err());
    }
    assert_eq!(
        engine.combatants[&hero].casting.as_ref().unwrap().slots[&1].remaining,
        2
    );
    assert!(engine.combatants[&hero].budget.action);
}

#[test]
fn magic_missile_hits_without_attack_and_supports_distributed_darts() {
    let (mut engine, hero, enemy) = fixture();
    engine.combatants.get_mut(&enemy).unwrap().ac = 100;
    let events = engine
        .resolve(CombatAction::Spell {
            caster: hero,
            spell_id: "magic-missile".into(),
            targets: vec![enemy],
            slot_level: 1,
        })
        .unwrap();
    assert_eq!(events.rolls.len(), 1);
    assert_eq!(events.rolls[0].detail.rolls.len(), 1);
    assert_eq!(
        events.damage.iter().map(|d| d.raw_amount).sum::<i32>(),
        events.rolls[0].detail.total * 3
    );
}

#[test]
fn legacy_bound_cast_cannot_consume_an_action_without_an_effect() {
    let (mut engine, hero, _) = fixture();
    assert!(engine
        .resolve(CombatAction::Cast { combatant: hero })
        .is_err());
    assert!(engine.combatants[&hero].budget.action);
}

#[test]
fn a_successful_save_deals_no_damage_and_uses_no_slot() {
    let (mut engine, hero, enemy) = fixture();
    engine.combatants.get_mut(&enemy).unwrap().dex_mod = 100;
    let events = engine
        .resolve(CombatAction::Spell {
            caster: hero,
            spell_id: "sacred-flame".into(),
            targets: vec![enemy],
            slot_level: 0,
        })
        .unwrap();
    assert!(events.damage.is_empty());
    assert_eq!(events.rolls.len(), 1);
    assert_eq!(engine.combatants[&enemy].current_hp, 100);
    assert_eq!(
        engine.combatants[&hero].casting.as_ref().unwrap().slots[&1].remaining,
        2
    );
}

#[test]
fn eldritch_blast_scales_beams_instead_of_damage_dice() {
    let (mut engine, hero, enemy) = fixture();
    let casting = engine
        .combatants
        .get_mut(&hero)
        .unwrap()
        .casting
        .as_mut()
        .unwrap();
    casting.level = 5;
    casting.prepared = vec!["eldritch-blast".into()];
    engine.combatants.get_mut(&enemy).unwrap().position = Position { x: 2, y: 0 };
    engine.combatants.get_mut(&enemy).unwrap().ac = 1;
    let events = engine
        .resolve(CombatAction::Spell {
            caster: hero,
            spell_id: "eldritch-blast".into(),
            targets: vec![enemy],
            slot_level: 0,
        })
        .unwrap();
    assert_eq!(
        events
            .rolls
            .iter()
            .filter(|roll| roll.purpose == "eldritch-blast attack")
            .count(),
        2
    );
    for damage in &events.damage {
        assert!(damage.raw_amount <= if damage.was_critical { 20 } else { 10 });
    }
    assert!(events.spell_slots_consumed.is_empty());
}

#[test]
fn depleted_slots_fail_before_healing_or_using_the_action() {
    let (mut engine, hero, _) = fixture();
    engine
        .combatants
        .get_mut(&hero)
        .unwrap()
        .casting
        .as_mut()
        .unwrap()
        .slots
        .get_mut(&1)
        .unwrap()
        .remaining = 0;
    assert!(engine
        .resolve(CombatAction::Spell {
            caster: hero,
            spell_id: "cure-wounds".into(),
            targets: vec![hero],
            slot_level: 1
        })
        .is_err());
    assert!(engine.combatants[&hero].budget.action);
    assert_eq!(engine.combatants[&hero].current_hp, 2);
}

#[test]
fn the_next_turn_allows_another_slot_and_healing_caps_at_max_hp() {
    let (mut engine, hero, enemy) = fixture();
    engine
        .resolve(CombatAction::Spell {
            caster: hero,
            spell_id: "healing-word".into(),
            targets: vec![hero],
            slot_level: 1,
        })
        .unwrap();
    engine
        .resolve(CombatAction::EndTurn { combatant: hero })
        .unwrap();
    engine
        .resolve(CombatAction::EndTurn { combatant: enemy })
        .unwrap();
    engine.combatants.get_mut(&hero).unwrap().current_hp = 19;
    let events = engine
        .resolve(CombatAction::Spell {
            caster: hero,
            spell_id: "cure-wounds".into(),
            targets: vec![hero],
            slot_level: 1,
        })
        .unwrap();
    assert_eq!(engine.combatants[&hero].current_hp, 20);
    assert_eq!(events.healing[0].amount, 1);
    assert_eq!(
        engine.combatants[&hero].casting.as_ref().unwrap().slots[&1].remaining,
        0
    );
}

#[test]
fn a_blinded_caster_can_make_a_ranged_spell_attack_with_disadvantage() {
    use app_domain::combat::conditions::Condition;
    let (mut engine, hero, enemy) = fixture();
    let actor = engine.combatants.get_mut(&hero).unwrap();
    actor.casting.as_mut().unwrap().prepared = vec!["fire-bolt".into()];
    actor.add_condition(Condition::Blinded);
    let events = engine
        .resolve(CombatAction::Spell {
            caster: hero,
            spell_id: "fire-bolt".into(),
            targets: vec![enemy],
            slot_level: 0,
        })
        .unwrap();
    assert_eq!(events.rolls[0].detail.rolls.len(), 2);
}

#[test]
fn a_hit_against_an_unconscious_target_within_five_feet_is_critical() {
    use app_domain::combat::conditions::Condition;
    let (mut engine, hero, enemy) = fixture();
    engine
        .combatants
        .get_mut(&hero)
        .unwrap()
        .casting
        .as_mut()
        .unwrap()
        .prepared = vec!["fire-bolt".into()];
    let victim = engine.combatants.get_mut(&enemy).unwrap();
    victim.ac = 1;
    victim.add_condition(Condition::Unconscious);
    let events = engine
        .resolve(CombatAction::Spell {
            caster: hero,
            spell_id: "fire-bolt".into(),
            targets: vec![enemy],
            slot_level: 0,
        })
        .unwrap();
    assert_eq!(events.damage.len(), 1);
    assert!(events.damage[0].was_critical);
}

#[test]
fn an_unconscious_target_automatically_fails_a_dexterity_spell_save() {
    use app_domain::combat::conditions::Condition;
    let (mut engine, hero, enemy) = fixture();
    let victim = engine.combatants.get_mut(&enemy).unwrap();
    victim.dex_mod = 100;
    victim.add_condition(Condition::Unconscious);
    let events = engine
        .resolve(CombatAction::Spell {
            caster: hero,
            spell_id: "sacred-flame".into(),
            targets: vec![enemy],
            slot_level: 0,
        })
        .unwrap();
    assert_eq!(events.damage.len(), 1);
    assert!(events.rolls[0].detail.rolls.is_empty());
}

#[test]
fn hostile_save_spell_damages_a_stable_hero_without_a_false_death_event() {
    use app_domain::combat::conditions::Condition;
    let (mut engine, caster, target) = fixture();
    engine.combatants.get_mut(&caster).unwrap().character_id = None;
    let victim = engine.combatants.get_mut(&target).unwrap();
    victim.character_id = Some(CharacterId(Uuid::new_v4()));
    victim.current_hp = 0;
    victim.death_saves.stabilize();
    victim.add_condition(Condition::Unconscious);
    let events = engine
        .resolve(CombatAction::Spell {
            caster,
            spell_id: "sacred-flame".into(),
            targets: vec![target],
            slot_level: 0,
        })
        .unwrap();
    assert_eq!(events.damage.len(), 1);
    assert!(events.deaths.is_empty());
    let victim = &engine.combatants[&target];
    assert!(!victim.death_saves.stable);
    assert_eq!(victim.death_saves.failures, 1);
    assert!(!victim.is_dead);
}

#[test]
fn each_magic_missile_dart_causes_a_separate_failure_at_zero_hp() {
    use app_domain::combat::conditions::Condition;
    let (mut engine, caster, target) = fixture();
    engine.combatants.get_mut(&caster).unwrap().character_id = None;
    let victim = engine.combatants.get_mut(&target).unwrap();
    victim.character_id = Some(CharacterId(Uuid::new_v4()));
    victim.current_hp = 0;
    victim.death_saves.stabilize();
    victim.add_condition(Condition::Unconscious);
    let events = engine
        .resolve(CombatAction::Spell {
            caster,
            spell_id: "magic-missile".into(),
            targets: vec![target],
            slot_level: 1,
        })
        .unwrap();
    assert_eq!(events.damage.len(), 3);
    assert_eq!(events.deaths, vec![target]);
    assert_eq!(engine.combatants[&target].death_saves.failures, 3);
    assert!(engine.combatants[&target].is_dead);
}

#[test]
fn missile_darts_do_not_combine_into_false_massive_damage() {
    let (mut engine, caster, target) = fixture();
    engine.combatants.get_mut(&caster).unwrap().character_id = None;
    let victim = engine.combatants.get_mut(&target).unwrap();
    victim.character_id = Some(CharacterId(Uuid::new_v4()));
    victim.max_hp = 7;
    victim.current_hp = 1;
    let events = engine
        .resolve(CombatAction::Spell {
            caster,
            spell_id: "magic-missile".into(),
            targets: vec![target],
            slot_level: 1,
        })
        .unwrap();
    assert_eq!(events.damage.len(), 3);
    assert!(events.deaths.is_empty());
    assert_eq!(engine.combatants[&target].death_saves.failures, 2);
    assert!(!engine.combatants[&target].is_dead);
}
