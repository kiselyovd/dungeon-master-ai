use adapter_sqlite::{pool, SqliteStore};
use app_application::agent::operations::{
    AgentOperation, AgentOperationRepository, OperationClaim,
};
use app_application::models::agent::{AgentEvent, AgentTurnRequest};
use app_application::models::chat::ChatMessage;
use uuid::Uuid;

#[tokio::test]
async fn resume_of_a_request_that_never_arrived_claims_the_same_identity() {
    let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&db).await.unwrap();
    let store = SqliteStore::new(db);
    let owner = Uuid::new_v4();
    let input = AgentOperation {
        request_id: Uuid::new_v4(),
        model: None,
        request: AgentTurnRequest {
            campaign_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            player_message: "Look".into(),
            history: vec![],
            images: vec![],
            board: None,
        },
    };
    assert_eq!(
        store.resume(input.clone(), owner).await.unwrap(),
        OperationClaim::Started
    );
    assert_eq!(
        store.claim(input, owner).await.unwrap(),
        OperationClaim::Running
    );
}

#[tokio::test]
async fn resume_repairs_a_confirmed_result_missing_from_the_delivery_log() {
    use app_application::agent::tool_dispatch::ToolExecution;
    use app_application::models::chat::{ToolCall, ToolResult};
    let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&db).await.unwrap();
    let store = SqliteStore::new(db);
    let owner = Uuid::new_v4();
    let input = AgentOperation {
        request_id: Uuid::new_v4(),
        model: None,
        request: AgentTurnRequest {
            campaign_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            player_message: "Look".into(),
            history: vec![],
            images: vec![],
            board: None,
        },
    };
    store.claim(input.clone(), owner).await.unwrap();
    let args = serde_json::json!({"action":{"kind":"move","location":"quay"}});
    let result = serde_json::json!({"revision":3,"inventory":{"gold":10}});
    for message in [
        ChatMessage::user_text("Look"),
        ChatMessage::AssistantWithToolCalls {
            content: None,
            tool_calls: vec![ToolCall {
                id: "call-1".into(),
                name: "adventure_action".into(),
                args: args.clone(),
            }],
        },
        ChatMessage::ToolResult(ToolResult {
            tool_call_id: "call-1".into(),
            content: result.to_string(),
            is_error: false,
        }),
    ] {
        store
            .append_turn_message(input.request_id, input.request.session_id, message)
            .await
            .unwrap();
    }
    store
        .claim_tool_step(
            input.request_id,
            "call-1",
            serde_json::json!({"tool_name":"adventure_action","args":args}),
        )
        .await
        .unwrap();
    store
        .finish_tool_step(
            input.request_id,
            "call-1",
            &ToolExecution {
                result: result.clone(),
                is_error: false,
                handled_by: "engine".into(),
                media: vec![],
            },
        )
        .await
        .unwrap();
    store.finish(input.request_id, owner, false).await.unwrap();
    let OperationClaim::Resuming { events, .. } = store.resume(input.clone(), owner).await.unwrap()
    else {
        panic!("expected resume")
    };
    assert!(
        matches!(events.first(),Some(AgentEvent::ToolCallStart{id,tool_name,..}) if id==&format!("{}:call-1",input.request_id) && tool_name=="adventure_action")
    );
    assert!(events.iter().any(|event| matches!(event, AgentEvent::ToolCallResult{id,result:actual,..} if id == &format!("{}:call-1",input.request_id) && actual == &result)));
}

#[tokio::test]
async fn interrupted_resume_retains_safe_messages_and_discards_unfinished_text() {
    let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&db).await.unwrap();
    let store = SqliteStore::new(db);
    let owner = Uuid::new_v4();
    let input = AgentOperation {
        request_id: Uuid::new_v4(),
        request: AgentTurnRequest {
            campaign_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            player_message: "Look".into(),
            history: vec![],
            images: vec![],
            board: None,
        },
        model: None,
    };
    store.claim(input.clone(), owner).await.unwrap();
    let message = ChatMessage::user_text("Look");
    store
        .append_turn_message(input.request_id, input.request.session_id, message.clone())
        .await
        .unwrap();
    store
        .append_event(
            input.request_id,
            owner,
            0,
            AgentEvent::TextDelta {
                text: "Unfinished text".into(),
            },
        )
        .await
        .unwrap();
    store.finish(input.request_id, owner, false).await.unwrap();
    let resumed = store.resume(input.clone(), Uuid::new_v4()).await.unwrap();
    assert_eq!(
        resumed,
        OperationClaim::Resuming {
            messages: vec![message],
            events: vec![]
        }
    );
    assert_eq!(
        store.claim(input, owner).await.unwrap(),
        OperationClaim::Running
    );
}

#[tokio::test]
async fn a_restored_timeline_rejects_an_old_interrupted_turn() {
    let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&db).await.unwrap();
    let store = std::sync::Arc::new(SqliteStore::new(db));
    let campaign = app_application::campaigns::CreateCampaign::new(store.clone())
        .execute(app_application::campaigns::CreateCampaignCommand {
            title: "Bell".into(),
            premise: "Ferry".into(),
            seed: app_application::models::session::CampaignSeed::Bundled {
                adventure_id: "bell-beneath-water".into(),
            },
        })
        .await
        .unwrap();
    let owner = Uuid::new_v4();
    let input = AgentOperation {
        request_id: Uuid::new_v4(),
        request: AgentTurnRequest {
            campaign_id: campaign.id.0,
            session_id: campaign.last_session_id.0,
            player_message: "Move".into(),
            history: vec![],
            images: vec![],
            board: None,
        },
        model: None,
    };
    store.claim(input.clone(), owner).await.unwrap();
    store.finish(input.request_id, owner, false).await.unwrap();
    sqlx::query("UPDATE sessions SET restore_generation=restore_generation+1 WHERE id=?1")
        .bind(campaign.last_session_id.0.to_string())
        .execute(store.pool())
        .await
        .unwrap();
    assert!(store.resume(input, Uuid::new_v4()).await.is_err());
}
