use adapter_sqlite::SqliteStore;
use app_application::{
    campaigns::{CreateCampaign, CreateCampaignCommand},
    models::{campaign_state::CommitCampaignCharacter, session::CampaignSeed},
    ports::repositories::{CampaignStateRepository, CombatRepository},
};
use app_server::test_support::TestServer;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

async fn campaign(
    store: Arc<SqliteStore>,
    title: &str,
) -> app_application::models::session::CampaignDetail {
    CreateCampaign::new(store)
        .execute(CreateCampaignCommand {
            title: title.into(),
            premise: "Bridge".into(),
            seed: CampaignSeed::Collaborative {
                answers: std::collections::BTreeMap::from([(
                    "world".into(),
                    vec!["Bridge".into()],
                )]),
            },
        })
        .await
        .unwrap()
}
async fn hero(
    store: &SqliteStore,
    campaign: &app_application::models::session::CampaignDetail,
) -> Uuid {
    let id = Uuid::new_v4();
    store.commit_character(CommitCampaignCharacter{campaign_id:campaign.id,session_id:campaign.last_session_id,character_id:id,expected_revision:campaign.revision,causation_id:"hero".into(),character:json!({"name":"Mira","heroClass":"fighter","level":1,"hp":7,"hpMax":12,"ac":16,"abilities":{"str":16,"dex":14,"con":14,"int":10,"wis":12,"cha":8},"inventory":[]})}).await.unwrap();
    id
}

#[tokio::test]
async fn combat_start_rejects_another_campaigns_session_before_mutation() {
    let server = TestServer::start().await;
    let store = Arc::new(SqliteStore::new(server.state.db().clone()));
    let a = campaign(store.clone(), "A").await;
    let b = campaign(store.clone(), "B").await;
    hero(&store, &b).await;
    let response = reqwest::Client::new()
        .post(server.url("/combat/start"))
        .json(
            &json!({"campaign_id":a.id.0,"session_id":b.last_session_id.0,"initiative_entries":[]}),
        )
        .send()
        .await
        .unwrap();
    assert!(
        response.status().is_client_error(),
        "mismatched identity must not mutate B under A's lane"
    );
    assert!(store
        .load_current(b.id)
        .await
        .unwrap()
        .projection
        .combat
        .is_none());
}

#[tokio::test]
async fn model_start_uses_an_explicit_npc_contract_without_a_second_hero() {
    use app_llm::ToolCall;
    use app_server::agent::tool_executor::execute_tool;
    let server = TestServer::start().await;
    let store = Arc::new(SqliteStore::new(server.state.db().clone()));
    let campaign = campaign(store.clone(), "Role contract").await;
    let character = hero(&store, &campaign).await;
    let ambiguous = ToolCall {
        id: "old-start".into(),
        name: "start_combat".into(),
        args: json!({"initiative_entries":[{"name":"Mira"},{"name":"Goblin"}]}),
    };
    let (_, error) = execute_tool(
        &ambiguous,
        &store,
        None,
        None,
        None,
        "mock",
        campaign.id.0,
        campaign.last_session_id.0,
    )
    .await;
    assert!(
        error,
        "an ambiguous legacy list must not create another owned hero"
    );
    assert!(store
        .load_current(campaign.id)
        .await
        .unwrap()
        .projection
        .combat
        .is_none());
    let call = ToolCall {
        id: "npc-start".into(),
        name: "start_combat".into(),
        args: json!({"initiative_entries":[{"role":"npc","name":"Goblin"}]}),
    };
    let (outcome, error) = execute_tool(
        &call,
        &store,
        None,
        None,
        None,
        "mock",
        campaign.id.0,
        campaign.last_session_id.0,
    )
    .await;
    assert!(!error, "{outcome}");
    let id = Uuid::parse_str(outcome["encounter_id"].as_str().unwrap()).unwrap();
    let stored = store.get(id).await.unwrap().unwrap();
    assert_eq!(stored.snapshot.combatants.len(), 2);
    assert_eq!(
        stored
            .snapshot
            .combatants
            .iter()
            .filter(|actor| actor.character_id.is_some_and(|id| id.0 == character))
            .count(),
        1
    );
    assert_eq!(outcome["projection"], serde_json::to_value(stored).unwrap());
}
