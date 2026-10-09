use adapter_sqlite::{human_commands::SoloHumanCommandLedger, SqliteStore};
use app_application::{
    agent::{
        human_commands::*,
        operations::{AgentOperation, AgentOperationRepository},
    },
    campaigns::{CreateCampaign, CreateCampaignCommand},
    models::{
        agent::AgentTurnRequest, campaign_state::CommitCampaignCharacter, session::CampaignSeed,
    },
    ports::repositories::CampaignStateRepository,
};
use app_llm::MockProvider;
use app_server::test_support::TestServer;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;
#[tokio::test]
async fn pending_http_read_recovers_original_request_on_a_new_server_without_mutation() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    app_server::db::init_db(&pool).await.unwrap();
    let store = SqliteStore::new(pool.clone());
    let campaign = CreateCampaign::new(Arc::new(store.clone()))
        .execute(CreateCampaignCommand {
            title: "Read pending".into(),
            premise: "Quay".into(),
            seed: CampaignSeed::Bundled {
                adventure_id: "bell-beneath-water".into(),
            },
        })
        .await
        .unwrap();
    let actor = Uuid::new_v4();
    store.commit_character(CommitCampaignCharacter {campaign_id:campaign.id,session_id:campaign.last_session_id,character_id:actor,expected_revision:campaign.revision,causation_id:"hero".into(),character:json!({"name":"Mira","heroClass":"fighter","level":1,"hp":12,"hpMax":12,"ac":14,"abilities":{"str":15,"dex":14,"con":13,"int":12,"wis":10,"cha":8},"inventory":[]})}).await.unwrap();
    let operation = AgentOperation {
        request_id: Uuid::new_v4(),
        model: Some("saved-model".into()),
        request: AgentTurnRequest {
            campaign_id: campaign.id.0,
            session_id: campaign.last_session_id.0,
            player_message: "Иду на причал".into(),
            history: vec![],
            images: vec![],
            board: Some("original-board".into()),
        },
    };
    let owner = Uuid::new_v4();
    store.claim(operation.clone(), owner).await.unwrap();
    let ledger = SoloHumanCommandLedger::new(store.clone(), operation.request_id, owner);
    ledger
        .save_verified_manifest(HumanCommandManifest {
            version: HUMAN_COMMAND_VERSION,
            scope: CommandScope {
                request_id: operation.request_id,
                campaign_id: operation.request.campaign_id,
                session_id: operation.request.session_id,
                actor_id: actor,
                authority: CommandAuthority::Solo {
                    restore_generation: 0,
                },
            },
            commands: vec![HumanCommand {
                repetition: None,
                id: 1,
                source: CommandSpan::from_quote(
                    &operation.request.player_message,
                    "Иду на причал",
                    0,
                )
                .unwrap(),
                goal: HumanCommandGoal::NeedsResolution {
                    description: "Место назначения".into(),
                },
                condition: None,
            }],
            constraints: vec![],
        })
        .await
        .unwrap();
    let question = ledger
        .record_clarification(1, "Какой причал?".into())
        .await
        .unwrap();
    store
        .finish(operation.request_id, owner, false)
        .await
        .unwrap();
    let before = store
        .load_current(campaign.id)
        .await
        .unwrap()
        .campaign
        .revision;
    let server = TestServer::start_with(Arc::new(MockProvider::new(vec![])), pool).await;
    let url = server.url(&format!(
        "/agent/pending?campaign_id={}&session_id={}",
        campaign.id.0, campaign.last_session_id.0
    ));
    let result: reqwest::Response = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let pending: PendingHumanCommandClarification = result.json().await.unwrap();
    assert_eq!(pending.operation, operation);
    assert_eq!(pending.clarification, Some(question));
    assert_eq!(
        store
            .load_current(campaign.id)
            .await
            .unwrap()
            .campaign
            .revision,
        before
    );
    let other = reqwest::Client::new()
        .get(server.url(&format!(
            "/agent/pending?campaign_id={}&session_id={}",
            campaign.id.0,
            Uuid::new_v4()
        )))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert!(other.is_null());
    let rooms = app_application::rooms::RoomService::new(
        Arc::new(store.clone()),
        Arc::new(store.clone()),
        Arc::new(adapter_secrets::InMemorySecretsStore::default()),
        Arc::new(app_application::rooms::SystemRoomClock),
    );
    rooms
        .create_local_room(app_application::models::room::CreateLocalRoom {
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            expected_revision: before,
            causation_id: "room".into(),
        })
        .await
        .unwrap();
    let room_owned = reqwest::Client::new()
        .get(server.url(&format!(
            "/agent/pending?campaign_id={}&session_id={}",
            campaign.id.0, campaign.last_session_id.0
        )))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert!(
        room_owned.is_null(),
        "room state is available only through authenticated room services"
    );
}
