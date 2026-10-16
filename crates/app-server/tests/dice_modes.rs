use app_llm::ToolCall;
use app_server::agent::tool_executor::execute_tool;
use serde_json::json;

#[tokio::test]
async fn actual_typed_dm_tool_executes_flags_and_rejects_invalid_damage_modes() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    app_server::db::init_db(&pool).await.unwrap();
    let store = adapter_sqlite::SqliteStore::new(pool);
    for (flag, higher) in [("advantage", true), ("disadvantage", false)] {
        let call = ToolCall {
            id: "dice-mode".into(),
            name: "roll_dice".into(),
            args: json!({"dice":"1d20","modifier":-2,flag:true}),
        };
        let (result, error) = execute_tool(
            &call,
            &store,
            None,
            None,
            None,
            "",
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
        )
        .await;
        assert!(!error, "{result}");
        let rolls = result["rolls"].as_array().unwrap();
        assert_eq!(rolls.len(), 2);
        let a = rolls[0].as_i64().unwrap();
        let b = rolls[1].as_i64().unwrap();
        assert_eq!(
            result["total"],
            if higher { a.max(b) } else { a.min(b) } - 2
        );
    }
    let call = ToolCall {
        id: "invalid-mode".into(),
        name: "roll_dice".into(),
        args: json!({"dice":"2d6","advantage":true}),
    };
    let (result, error) = execute_tool(
        &call,
        &store,
        None,
        None,
        None,
        "",
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
    )
    .await;
    assert!(error);
    assert!(result["error"].as_str().unwrap().contains("single d20"));
}
