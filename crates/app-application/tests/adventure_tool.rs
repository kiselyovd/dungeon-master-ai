use app_application::agent::tool_decoder::decode_tool_call;
use app_application::agent::tools::AgentToolCatalog;
use app_application::models::chat::Tool;
use serde_json::json;

#[test]
fn adventure_tool_accepts_only_typed_intentions_not_model_owned_identity() {
    let command = decode_tool_call(
        "adventure_action",
        &json!({"action":{"kind":"move","location":"quay"}}),
    )
    .unwrap();
    assert_eq!(command.tool_name(), "adventure_action");
    assert_eq!(
        command.to_args(),
        json!({"action":{"kind":"move","location":"quay"}})
    );
    for args in [
        json!({"action":{"kind":"grant_gold"}}),
        json!({"action":{"kind":"move","location":"moon"}}),
        json!({"action":{"kind":"finish"},"character_id":"invented"}),
    ] {
        assert!(decode_tool_call("adventure_action", &args).is_err());
    }
}

#[test]
fn authored_catalog_withholds_tools_that_rewrite_canon() {
    let names = [
        "adventure_action",
        "set_scene",
        "remember_npc",
        "start_combat",
        "roll_dice",
        "quick_save",
    ];
    let catalog = AgentToolCatalog {
        exploration: names
            .iter()
            .map(|name| Tool {
                name: (*name).into(),
                description: String::new(),
                parameters: json!({}),
            })
            .collect(),
        combat: vec![],
    };
    let authored = catalog.for_context(false, true);
    assert_eq!(
        authored.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
        vec!["adventure_action", "roll_dice", "quick_save"]
    );
    assert!(!catalog
        .for_context(false, false)
        .iter()
        .any(|t| t.name == "adventure_action"));
}
