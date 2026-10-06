use adapter_sqlite::{pool, SqliteStore};
use app_application::campaigns::{CreateCampaign, CreateCampaignCommand};
use app_application::models::adventure::{Action, AdventureIntent, Location};
use app_application::models::campaign_state::CommitCampaignCharacter;
use app_application::models::session::CampaignSeed;
use app_application::ports::repositories::{AdventureRepository, CampaignStateRepository};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn dm_intention_replay_returns_current_state_without_repeating_or_rewinding() {
    let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&db).await.unwrap();
    let store = Arc::new(SqliteStore::new(db));
    let campaign = CreateCampaign::new(store.clone())
        .execute(CreateCampaignCommand {
            title: "Bell".into(),
            premise: "Ferry".into(),
            seed: CampaignSeed::Bundled {
                adventure_id: "bell-beneath-water".into(),
            },
        })
        .await
        .unwrap();
    let hero = Uuid::new_v4();
    store.commit_character(CommitCampaignCharacter{campaign_id:campaign.id,session_id:campaign.last_session_id,character_id:hero,expected_revision:campaign.revision,causation_id:"hero".into(),character:json!({"name":"Mira","heroClass":"fighter","level":1,"hp":12,"hpMax":12,"ac":14,"abilities":{"str":15,"dex":14,"con":13,"int":12,"wis":10,"cha":8},"inventory":[]})}).await.unwrap();
    let first = AdventureIntent {
        campaign_id: campaign.id,
        session_id: campaign.last_session_id,
        causation_id: "turn-1:call-1".into(),
        action: Action::Move(Location::Quay),
    };
    let outcome = store.act_as_dm(first.clone()).await.unwrap();
    assert_eq!(outcome.character_id, Some(hero));
    assert_eq!(
        outcome.projection.adventure.unwrap().state.location,
        Location::Quay
    );
    let next = AdventureIntent {
        causation_id: "turn-2:call-1".into(),
        action: Action::Move(Location::Archive),
        ..first.clone()
    };
    let advanced = store.act_as_dm(next).await.unwrap();
    let replayed = store.act_as_dm(first.clone()).await.unwrap();
    assert_eq!(replayed, advanced);
    let altered = AdventureIntent {
        action: Action::Move(Location::Town),
        ..first
    };
    assert!(store.act_as_dm(altered).await.is_err());
}
