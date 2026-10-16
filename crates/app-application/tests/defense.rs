use app_application::defense::normalize;
use serde_json::json;
fn sheet() -> serde_json::Value {
    json!({"name":"Hero","heroClass":"fighter","race":"human","level":1,"hp":11,"hpMax":11,"ac":12,"speedFt":30,"abilities":{"str":12,"dex":14,"con":13,"int":12,"wis":10,"cha":8},"inventory":[{"id":"chain-mail","name":"Chain Mail","count":1},{"id":"shield","name":"Shield","count":1}]})
}
#[test]
fn known_legacy_armor_projects_correctly_and_repeated_projection_does_not_reduce_speed_again() {
    let mut value = sheet();
    normalize(&mut value).unwrap();
    assert_eq!(value["ac"], 18);
    assert_eq!(value["speedFt"], 20);
    let once = value.clone();
    normalize(&mut value).unwrap();
    assert_eq!(value, once);
}
#[test]
fn explicit_carrying_and_shield_choices_override_cached_numbers() {
    let mut value = sheet();
    value["defense"] = json!({"schema_version":1,"armor_id":null,"shield":false});
    value["ac"] = json!(99);
    normalize(&mut value).unwrap();
    assert_eq!(value["ac"], 12);
    assert_eq!(value["speedFt"], 30);
}
#[test]
fn invalid_declarations_are_rejected_and_unmodeled_legacy_sheets_remain_compatible() {
    let mut value = sheet();
    value["defense"] = json!({"schema_version":1,"armor_id":"unknown","shield":false});
    let before = value.clone();
    assert!(normalize(&mut value).is_err());
    assert_eq!(value, before);
    let mut value = sheet();
    value["inventory"] = json!([]);
    value["ac"] = json!(20);
    let before = value.clone();
    assert!(normalize(&mut value).unwrap().is_none());
    assert_eq!(value, before);
}

#[test]
fn implicit_legacy_upgrade_at_the_budget_boundary_preserves_readability() {
    use app_application::{
        defense::normalize_for_read,
        models::campaign_state::{within_character_budget, MAX_CHARACTER_METADATA_BYTES},
    };
    let mut value = sheet();
    value["backstory"] = json!("");
    let size = serde_json::to_vec(&value).unwrap().len();
    value["backstory"] = json!("x".repeat(MAX_CHARACTER_METADATA_BYTES - size));
    assert!(within_character_budget(&value));
    let before = value.clone();
    assert!(normalize_for_read(&mut value).unwrap().is_none());
    assert_eq!(value, before);
    normalize(&mut value).unwrap();
    assert!(!within_character_budget(&value));
}
