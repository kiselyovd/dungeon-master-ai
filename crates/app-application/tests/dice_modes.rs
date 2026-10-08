use app_application::agent::tool_decoder::decode_tool_call;
use serde_json::json;

#[test]
fn dice_modes_are_typed_and_limited_to_single_d20_tests() {
    for dice in ["d20", "1d20", "1D20"] {
        let command =
            decode_tool_call("roll_dice", &json!({"dice":dice,"disadvantage":true})).unwrap();
        assert_eq!(command.to_args()["disadvantage"], true);
    }
    for dice in ["2d20", "2d6", "bad", "1d6"] {
        assert!(decode_tool_call("roll_dice", &json!({"dice":dice,"advantage":true})).is_err());
    }
    assert!(decode_tool_call("roll_dice", &json!({"dice":"2d6"})).is_ok());
}

#[test]
fn canonical_checks_reject_unknown_skills_invented_outcomes_and_model_modifiers() {
    let base = json!({"dice":"1d20","check":{"character_id":uuid::Uuid::new_v4(),"skill":"stealth","dc":15}});
    assert!(decode_tool_call("roll_dice", &base).is_ok());
    for patch in [
        json!({"skill":"unknown"}),
        json!({"dc":41}),
        json!({"dc":-1}),
        json!({"total":20}),
        json!({"character_id":uuid::Uuid::nil()}),
    ] {
        let mut args = base.clone();
        for (key, value) in patch.as_object().unwrap() {
            args["check"][key] = value.clone();
        }
        assert!(decode_tool_call("roll_dice", &args).is_err(), "{args}");
    }
    let mut args = base;
    args["modifier"] = json!(99);
    assert!(decode_tool_call("roll_dice", &args).is_err());
}
