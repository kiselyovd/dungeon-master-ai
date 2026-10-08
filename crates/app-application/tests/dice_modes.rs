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
