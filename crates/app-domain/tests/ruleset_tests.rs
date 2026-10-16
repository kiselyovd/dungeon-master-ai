#[test]
fn production_ruleset_is_srd_521_only() {
    assert_eq!(app_domain::ruleset::RULESET_ID, "srd-5.2.1");
    assert_eq!(app_domain::ruleset::GAME_SCHEMA_VERSION, 1);
}
