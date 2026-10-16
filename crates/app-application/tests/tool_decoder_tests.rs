use app_application::agent::tool_decoder::{decode_tool_call, ToolDecodeError};
use serde_json::{json, Value};

#[test]
fn decodes_every_supported_tool_into_a_typed_variant() {
    let cases: [(&str, Value); 18] = [
        ("roll_dice", json!({"dice": "2d6", "modifier": 1})),
        (
            "apply_damage",
            json!({"token_id": "enemy-1", "amount": 5, "type": "fire"}),
        ),
        ("apply_healing", json!({"token_id": "hero-1", "amount": 5})),
        (
            "start_combat",
            json!({"initiative_entries": [{"name": "Hero"}]}),
        ),
        ("end_combat", json!({})),
        (
            "add_token",
            json!({"id": "enemy-1", "name": "Goblin", "x": 1, "y": 2, "hp": 7, "max_hp": 7, "ac": 13}),
        ),
        ("update_token", json!({"id": "enemy-1", "hp": 2})),
        ("remove_token", json!({"id": "enemy-1"})),
        (
            "set_scene",
            json!({"title": "Tavern", "mode": "exploration"}),
        ),
        (
            "cast_spell",
            json!({"caster_id": "hero-1", "spell": "magic_missile", "targets": ["enemy-1"], "slot_level": 1}),
        ),
        (
            "remember_npc",
            json!({"name": "Mira", "fact": "Saved the party"}),
        ),
        ("recall_npc", json!({"name": "Mira"})),
        (
            "journal_append",
            json!({"entry_html": "<p>We entered.</p>"}),
        ),
        ("quick_save", json!({})),
        ("query_rules", json!({"question": "How does cover work?"})),
        ("generate_map", json!({"prompt": "ruined hall"})),
        (
            "generate_illustration",
            json!({"prompt": "hooded figure", "style": "portrait"}),
        ),
        (
            "generate_video",
            json!({"prompt": "dragon swoops", "seconds": 4.0, "frame_count": 97}),
        ),
    ];

    for (name, args) in cases {
        let command = decode_tool_call(name, &args).unwrap_or_else(|error| {
            panic!("{name} should decode, got {error}");
        });
        assert_eq!(command.tool_name(), name);
    }
}

#[test]
fn rejects_unknown_tools_and_non_object_payloads() {
    assert!(matches!(
        decode_tool_call("fly_dragon", &json!({})),
        Err(ToolDecodeError::UnknownTool(_))
    ));
    assert!(matches!(
        decode_tool_call("roll_dice", &json!(["2d6"])),
        Err(ToolDecodeError::InvalidArgs(_))
    ));
}

#[test]
fn validates_ids_and_numeric_bounds() {
    for (name, args) in [
        (
            "apply_damage",
            json!({"token_id": "", "amount": 1, "type": "fire"}),
        ),
        (
            "apply_damage",
            json!({"token_id": "enemy", "amount": -1, "type": "fire"}),
        ),
        (
            "cast_spell",
            json!({"caster_id": "hero", "spell": "shield", "targets": ["enemy"], "slot_level": 10}),
        ),
        ("generate_video", json!({"prompt": "fog", "seconds": 2.0})),
        ("generate_video", json!({"prompt": "fog", "frame_count": 0})),
    ] {
        assert!(
            matches!(
                decode_tool_call(name, &args),
                Err(ToolDecodeError::ValidationFailed(_))
            ),
            "{name} should reject {args}"
        );
    }
}

#[test]
fn ignores_unknown_fields_after_typed_decoding() {
    let command = decode_tool_call(
        "roll_dice",
        &json!({"dice": "1d20", "future_extension": true}),
    )
    .unwrap();
    assert_eq!(command.tool_name(), "roll_dice");
    assert!(command.to_args().get("future_extension").is_none());
}

#[test]
fn preserves_empty_start_combat_entries_compatibility() {
    let command = decode_tool_call("start_combat", &json!({"initiative_entries": []})).unwrap();
    assert_eq!(command.tool_name(), "start_combat");
}

#[test]
fn preserves_legacy_validation_failures_at_the_application_boundary() {
    let validation_failures = [
        ("set_scene", json!({"title": "Tavern", "mode": "dungeon"})),
        (
            "cast_spell",
            json!({"caster_id": "hero", "spell": "shield", "targets": []}),
        ),
        ("remember_npc", json!({"name": "Mira", "fact": ""})),
        ("journal_append", json!({"entry_html": "  "})),
        ("generate_map", json!({"prompt": ""})),
        ("generate_illustration", json!({"prompt": "  "})),
        ("apply_healing", json!({"token_id": "hero", "amount": -1})),
    ];
    for (name, args) in validation_failures {
        assert!(matches!(
            decode_tool_call(name, &args),
            Err(ToolDecodeError::ValidationFailed(_))
        ));
    }

    assert!(matches!(
        decode_tool_call("set_scene", &json!({"title": "Tavern"})),
        Err(ToolDecodeError::InvalidArgs(_))
    ));
}
