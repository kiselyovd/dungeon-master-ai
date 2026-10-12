use app_application::{
    campaigns::{CreateCampaign, CreateCampaignCommand},
    models::{campaign_state::CommitCampaignCharacter, session::CampaignSeed},
    ports::repositories::CampaignStateRepository,
};
use app_llm::ToolCall;
use app_server::agent::tool_executor::execute_tool;
use serde_json::json;
use std::{collections::BTreeMap, sync::Arc};
use uuid::Uuid;

#[tokio::test]
async fn actual_dm_check_reads_saved_hero_and_rejects_foreign_identity_and_model_modifier() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    adapter_sqlite::pool::migrate(&pool).await.unwrap();
    let store = Arc::new(adapter_sqlite::SqliteStore::new(pool));
    let campaign = CreateCampaign::new(store.clone())
        .execute(CreateCampaignCommand {
            request_id: None,
            title: "Checks".into(),
            premise: "A ferry".into(),
            seed: CampaignSeed::Collaborative {
                answers: BTreeMap::new(),
            },
        })
        .await
        .unwrap();
    let character = Uuid::new_v4();
    store.commit_character(CommitCampaignCharacter {campaign_id:campaign.id,session_id:campaign.last_session_id,character_id:character,expected_revision:campaign.revision,causation_id:"hero".into(),character:json!({"name":"Boris","heroClass":"fighter","race":"human","level":1,"hp":12,"hpMax":12,"ac":10,"abilities":{"str":15,"dex":14,"con":13,"int":12,"wis":10,"cha":8},"inventory":[{"id":"chain-mail","name":"Chain Mail","count":1}],"defense":{"schema_version":1,"armor_id":"chain-mail","shield":false},"skillProfs":{"stealth":true}}) }).await.unwrap();
    let base = json!({"dice":"1d20","check":{"character_id":character,"skill":"stealth","dc":15}});
    let invoke = |args| async {
        execute_tool(
            &ToolCall {
                id: "check-1".into(),
                name: "roll_dice".into(),
                args,
            },
            &store,
            None,
            None,
            None,
            "",
            campaign.id.0,
            campaign.last_session_id.0,
        )
        .await
    };
    let (result, error) = invoke(base.clone()).await;
    assert!(!error, "{result}");
    assert_eq!(result["modifier"], 4);
    assert_eq!(result["armor_disadvantage"], true);
    assert_eq!(result["rolls"].as_array().unwrap().len(), 2);
    let mut args = base.clone();
    args["modifier"] = json!(99);
    assert!(invoke(args).await.1);
    let mut args = base;
    args["check"]["character_id"] = json!(Uuid::new_v4());
    assert!(invoke(args).await.1);
    let after = store.load_current(campaign.id).await.unwrap();
    assert_eq!(after.campaign.revision.0, 2);
    assert_eq!(after.projection.character["hp"], 12);
}
