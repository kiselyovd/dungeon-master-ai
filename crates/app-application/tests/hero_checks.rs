use app_application::hero_checks::{roll_skill, HeroSkillCheck, Skill};
use app_domain::rng::SeededRng;
use serde_json::json;
use uuid::Uuid;

fn hero() -> serde_json::Value {
    json!({"name":"Boris","heroClass":"fighter","race":"human","level":5,"hp":20,"hpMax":20,"ac":1,"speedFt":30,"abilities":{"str":15,"dex":14,"con":13,"int":12,"wis":10,"cha":8},"inventory":[{"id":"chain-mail","name":"Chain Mail","count":1}],"defense":{"schema_version":1,"armor_id":"chain-mail","shield":false},"skillProfs":{"stealth":true,"athletics":true},"proficiencyBonus":99})
}
fn check(skill: Skill) -> HeroSkillCheck {
    HeroSkillCheck {
        character_id: Uuid::new_v4(),
        skill,
        dc: Some(40),
    }
}

#[test]
fn stealth_derives_modifier_and_worn_armor_penalty_from_canonical_sheet() {
    let result = roll_skill(
        &hero(),
        &check(Skill::Stealth),
        false,
        false,
        &mut SeededRng::from_seed(42),
    )
    .unwrap();
    assert_eq!(result["modifier"], 5);
    assert_eq!(result["armor_disadvantage"], true);
    let rolls = result["rolls"].as_array().unwrap();
    assert_eq!(rolls.len(), 2);
    assert_eq!(
        result["total"],
        rolls.iter().map(|v| v.as_i64().unwrap()).min().unwrap() + 5
    );
    assert_eq!(result["success"], false);
    let result = roll_skill(
        &hero(),
        &check(Skill::Athletics),
        false,
        false,
        &mut SeededRng::from_seed(42),
    )
    .unwrap();
    assert_eq!(result["modifier"], 5);
    assert_eq!(result["rolls"].as_array().unwrap().len(), 1);
}
#[test]
fn carried_armor_does_not_impose_penalty_and_advantage_cancels_worn_penalty() {
    let result = roll_skill(
        &hero(),
        &check(Skill::Stealth),
        true,
        false,
        &mut SeededRng::from_seed(42),
    )
    .unwrap();
    assert_eq!(result["armor_disadvantage"], true);
    assert_eq!(result["rolls"].as_array().unwrap().len(), 1);
    let mut sheet = hero();
    sheet["defense"]["armor_id"] = json!(null);
    let result = roll_skill(
        &sheet,
        &check(Skill::Stealth),
        false,
        false,
        &mut SeededRng::from_seed(42),
    )
    .unwrap();
    assert_eq!(result["armor_disadvantage"], false);
    assert_eq!(result["rolls"].as_array().unwrap().len(), 1);
    sheet["hp"] = json!(0);
    assert_eq!(
        roll_skill(
            &sheet,
            &check(Skill::Stealth),
            false,
            false,
            &mut SeededRng::from_seed(42)
        )
        .unwrap_err(),
        "hero_check_incapacitated"
    );
}
#[test]
fn natural_twenty_does_not_invent_automatic_skill_success() {
    for seed in 0..1000 {
        let result = roll_skill(
            &hero(),
            &check(Skill::History),
            false,
            false,
            &mut SeededRng::from_seed(seed),
        )
        .unwrap();
        if result["rolls"][0] == 20 {
            assert_eq!(result["success"], false);
            return;
        }
    }
    panic!("no natural twenty fixture found");
}

#[test]
fn skill_check_context_never_serializes_portrait_biography_or_unbounded_name() {
    let mut sheet = hero();
    sheet["portraitUrl"] = json!("x".repeat(8 * 1024 * 1024));
    sheet["backstory"] = json!("private prose".repeat(1000));
    sheet["name"] = json!("Б".repeat(500));
    let identity = app_application::hero_checks::context_identity(Uuid::new_v4(), &sheet);
    assert_eq!(identity["name"].as_str().unwrap().chars().count(), 120);
    assert!(identity.get("portraitUrl").is_none());
    assert!(identity.get("backstory").is_none());
    assert!(identity.to_string().len() < 400);
}

#[test]
fn exact_budget_legacy_checks_follow_the_same_read_compatibility_as_combat() {
    use app_application::models::campaign_state::{
        within_character_budget, MAX_CHARACTER_METADATA_BYTES,
    };
    let mut sheet = hero();
    sheet.as_object_mut().unwrap().remove("defense");
    sheet["backstory"] = json!("");
    let size = serde_json::to_vec(&sheet).unwrap().len();
    sheet["backstory"] = json!("x".repeat(MAX_CHARACTER_METADATA_BYTES - size));
    assert!(within_character_budget(&sheet));
    let before = sheet.clone();
    let result = roll_skill(
        &sheet,
        &check(Skill::Stealth),
        false,
        false,
        &mut SeededRng::from_seed(42),
    )
    .unwrap();
    assert_eq!(result["armor_disadvantage"], false);
    assert_eq!(result["rolls"].as_array().unwrap().len(), 1);
    assert_eq!(sheet, before);
}

#[test]
fn poisoned_condition_is_read_from_server_vitality_for_every_skill() {
    use app_domain::combat::conditions::Condition;
    let id = Uuid::new_v4();
    let mut sheet = hero();
    let mut actor = app_application::recovery::actor(&sheet, id).unwrap();
    actor.conditions.insert(Condition::Poisoned);
    app_application::vitality::store_actor(&mut sheet, &actor).unwrap();
    let check = HeroSkillCheck {
        character_id: id,
        skill: Skill::History,
        dc: None,
    };
    let result = roll_skill(&sheet, &check, false, false, &mut SeededRng::from_seed(42)).unwrap();
    assert_eq!(result["poisoned_disadvantage"], true);
    assert_eq!(result["armor_disadvantage"], false);
    assert_eq!(result["rolls"].as_array().unwrap().len(), 2);
}
