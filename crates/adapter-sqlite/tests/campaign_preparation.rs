use adapter_sqlite::{pool, SqliteStore};
use app_application::{
    campaigns::{CreateCampaign, CreateCampaignCommand},
    models::{
        campaign_state::CommitCampaignCharacter,
        identity::{CampaignId, Revision},
        session::CampaignSeed,
    },
    ports::repositories::{CampaignRepository, CampaignStateRepository},
};
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

async fn store() -> Arc<SqliteStore> {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&pool).await.unwrap();
    Arc::new(SqliteStore::new(pool))
}
fn command(id: Uuid) -> CreateCampaignCommand {
    CreateCampaignCommand {
        request_id: Some(id),
        title: " Rain bridge ".into(),
        premise: " A bridge at dawn. ".into(),
        seed: CampaignSeed::Collaborative {
            answers: Default::default(),
        },
    }
}
#[tokio::test]
async fn concurrent_creation_and_lost_ack_keep_one_campaign_session_event_and_opening() {
    let store = store().await;
    let id = Uuid::new_v4();
    let create = CreateCampaign::new(store.clone());
    let (first, second) = tokio::join!(create.execute(command(id)), create.execute(command(id)));
    assert_eq!(first.unwrap(), second.unwrap());
    assert_eq!(
        create.execute(command(id)).await.unwrap().id,
        CampaignId(id)
    );
    let state = store.load_current(CampaignId(id)).await.unwrap();
    assert_eq!(state.campaign.revision, Revision(1));
    assert!(state.character_id.is_none());
    assert_eq!(state.projection.scene.unwrap()["title"], "Rain bridge");
    for (table, query) in [
        ("campaigns", "SELECT count(*) FROM campaigns"),
        ("sessions", "SELECT count(*) FROM sessions"),
        ("campaign_events", "SELECT count(*) FROM campaign_events"),
        ("scenes", "SELECT count(*) FROM scenes"),
    ] {
        let count: i64 = sqlx::query_scalar(query)
            .fetch_one(store.pool())
            .await
            .unwrap();
        assert_eq!(count, 1, "{table}");
    }
    let mut changed = command(id);
    changed.premise = "Changed request".into();
    assert!(create.execute(changed).await.is_err());
    assert_eq!(
        store.get(CampaignId(id)).await.unwrap().unwrap().premise,
        "A bridge at dawn."
    );
}
#[tokio::test]
async fn failed_opening_initialization_rolls_back_the_whole_campaign() {
    let store = store().await;
    sqlx::query("CREATE TRIGGER reject_opening BEFORE INSERT ON scenes BEGIN SELECT RAISE(ABORT, 'fixture'); END").execute(store.pool()).await.unwrap();
    let id = Uuid::new_v4();
    assert!(CreateCampaign::new(store.clone())
        .execute(command(id))
        .await
        .is_err());
    assert!(store.get(CampaignId(id)).await.unwrap().is_none());
    assert!(store.list().await.unwrap().is_empty());
}
#[tokio::test]
async fn every_production_prepared_hero_commits_and_retries_without_duplicate_effects() {
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/prepared_heroes.json")).unwrap();
    for (class, character) in fixtures.as_object().unwrap() {
        let store = store().await;
        let campaign = CreateCampaign::new(store.clone())
            .execute(command(Uuid::new_v4()))
            .await
            .unwrap();
        let request = CommitCampaignCharacter {
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            character_id: Uuid::new_v4(),
            expected_revision: Revision(1),
            causation_id: Uuid::new_v4().to_string(),
            character: character.clone(),
        };
        let state = store.commit_character(request.clone()).await.unwrap();
        assert_eq!(state.campaign.revision, Revision(2));
        assert_eq!(state.projection.character["heroClass"], class.as_str());
        assert_eq!(state.projection.character["ac"], character["ac"]);
        assert_eq!(state.projection.character["spells"], character["spells"]);
        assert_eq!(store.commit_character(request).await.unwrap(), state);
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM character_commits")
            .fetch_one(store.pool())
            .await
            .unwrap();
        assert_eq!(count, 1);
    }
}
