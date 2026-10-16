use app_application::agent::tool_decoder::decode_tool_call;
use serde_json::json;

#[test]
fn canonical_hero_intentions_are_available_without_model_owned_stats() {
    for args in [
        json!({"kind":"end_turn"}),
        json!({"kind":"move","x":3,"y":4}),
        json!({"kind":"spell","spell_id":"magic-missile","targets":["aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"],"slot_level":1}),
    ] {
        assert!(decode_tool_call("combat_action", &args).is_ok());
    }
    assert!(decode_tool_call("rest_action", &json!({"kind":"short","hit_dice":1})).is_ok());
}

#[test]
fn model_cannot_select_another_actor_or_supply_damage_or_resource_stock() {
    for args in [
        json!({"kind":"end_turn","combatant_id":"another"}),
        json!({"kind":"move","x":3,"y":4,"damage":99}),
        json!({"kind":"spell","spell_id":"magic-missile","targets":[],"slot_level":1,"spell_slots":99}),
    ] {
        assert!(decode_tool_call("combat_action", &args).is_err());
    }
    assert!(decode_tool_call("rest_action", &json!({"kind":"long","hp":999})).is_err());
}
