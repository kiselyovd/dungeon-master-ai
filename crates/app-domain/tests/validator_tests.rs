use app_domain::combat::validator::{validate_tool_call, ToolCallError};
use serde_json::json;

// ---- Existing validators (regression) ----

#[test]
fn roll_dice_valid() {
    let v = validate_tool_call("roll_dice", json!({ "dice": "2d6" })).unwrap();
    assert_eq!(v.tool_name, "roll_dice");
}

#[test]
fn apply_damage_rejects_negative() {
    let err = validate_tool_call(
        "apply_damage",
        json!({ "token_id": "abc", "amount": -5, "type": "fire" }),
    ).unwrap_err();
    assert!(matches!(err, ToolCallError::ValidationFailed(_)));
}

// ---- New M3 validators ----

#[test]
fn set_scene_valid_exploration() {
    let v = validate_tool_call("set_scene", json!({ "title": "Tavern", "mode": "exploration" }))
        .unwrap();
    assert_eq!(v.tool_name, "set_scene");
}

#[test]
fn set_scene_rejects_missing_mode() {
    let err = validate_tool_call("set_scene", json!({ "title": "Tavern" })).unwrap_err();
    assert!(matches!(err, ToolCallError::InvalidArgs(_)));
}

#[test]
fn set_scene_rejects_invalid_mode() {
    let err = validate_tool_call(
        "set_scene",
        json!({ "title": "Tavern", "mode": "dungeon_crawl" }),
    ).unwrap_err();
    assert!(matches!(err, ToolCallError::ValidationFailed(_)));
}

#[test]
fn cast_spell_valid() {
    let v = validate_tool_call(
        "cast_spell",
        json!({ "caster_id": "p1", "spell": "magic_missile", "targets": ["e1"] }),
    ).unwrap();
    assert_eq!(v.tool_name, "cast_spell");
}

#[test]
fn cast_spell_rejects_empty_targets() {
    let err = validate_tool_call(
        "cast_spell",
        json!({ "caster_id": "p1", "spell": "magic_missile", "targets": [] }),
    ).unwrap_err();
    assert!(matches!(err, ToolCallError::ValidationFailed(_)));
}

#[test]
fn remember_npc_valid() {
    let v = validate_tool_call(
        "remember_npc",
        json!({ "name": "Mira", "fact": "She saved the party in session 2" }),
    ).unwrap();
    assert_eq!(v.tool_name, "remember_npc");
}

#[test]
fn remember_npc_rejects_empty_fact() {
    let err = validate_tool_call(
        "remember_npc",
        json!({ "name": "Mira", "fact": "" }),
    ).unwrap_err();
    assert!(matches!(err, ToolCallError::ValidationFailed(_)));
}

#[test]
fn recall_npc_valid() {
    let v = validate_tool_call("recall_npc", json!({ "name": "Mira" })).unwrap();
    assert_eq!(v.tool_name, "recall_npc");
}

#[test]
fn journal_append_valid() {
    let v = validate_tool_call(
        "journal_append",
        json!({ "entry_html": "<p>The party entered the dungeon.</p>" }),
    ).unwrap();
    assert_eq!(v.tool_name, "journal_append");
}

#[test]
fn journal_append_rejects_empty_html() {
    let err = validate_tool_call("journal_append", json!({ "entry_html": "" })).unwrap_err();
    assert!(matches!(err, ToolCallError::ValidationFailed(_)));
}

#[test]
fn quick_save_valid_no_label() {
    let v = validate_tool_call("quick_save", json!({})).unwrap();
    assert_eq!(v.tool_name, "quick_save");
}

#[test]
fn query_rules_valid() {
    let v = validate_tool_call("query_rules", json!({ "question": "How does grappling work?" }))
        .unwrap();
    assert_eq!(v.tool_name, "query_rules");
}

#[test]
fn generate_image_valid() {
    let v = validate_tool_call(
        "generate_image",
        json!({ "prompt": "Dark tavern with hooded figures", "style": "dark_fantasy" }),
    ).unwrap();
    assert_eq!(v.tool_name, "generate_image");
}

#[test]
fn generate_image_rejects_empty_prompt() {
    let err = validate_tool_call("generate_image", json!({ "prompt": "" })).unwrap_err();
    assert!(matches!(err, ToolCallError::ValidationFailed(_)));
}

#[test]
fn unknown_tool_errors() {
    let err = validate_tool_call("fly_dragon", json!({})).unwrap_err();
    assert!(matches!(err, ToolCallError::UnknownTool(_)));
}
