use adapter_sqlite::{pool, SqliteStore};
use app_application::{
    campaigns::{CreateCampaign, CreateCampaignCommand, CreateSnapshot, CreateSnapshotCommand},
    models::{
        campaign_state::CommitCampaignCharacter,
        rest::{RestCommand, RestIntent},
        session::CampaignSeed,
        snapshot::RestoreSnapshotRecord,
    },
    ports::repositories::{CampaignStateRepository, RestRepository, SnapshotRepository},
};
use serde_json::json;
use std::{collections::BTreeMap, sync::Arc};
use uuid::Uuid;

async fn fixture() -> (Arc<SqliteStore>, RestCommand) {
    fixture_with_seed(CampaignSeed::Collaborative {
        answers: BTreeMap::from([("world".into(), vec!["Town".into()])]),
    })
    .await
}
async fn fixture_with_seed(seed: CampaignSeed) -> (Arc<SqliteStore>, RestCommand) {
    let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&db).await.unwrap();
    let store = Arc::new(SqliteStore::new(db));
    let campaign = CreateCampaign::new(store.clone())
        .execute(CreateCampaignCommand {
            title: "Rest".into(),
            premise: "Town".into(),
            seed,
        })
        .await
        .unwrap();
    let character_id = Uuid::new_v4();
    let state=store.commit_character(CommitCampaignCharacter{campaign_id:campaign.id,session_id:campaign.last_session_id,character_id,expected_revision:campaign.revision,causation_id:"hero".into(),character:json!({"name":"Mira","heroClass":"cleric","level":1,"hp":2,"hpMax":20,"ac":16,"abilities":{"str":12,"dex":14,"con":14,"int":10,"wis":16,"cha":8},"inventory":[{"id":"arrows-20","name":"Arrows","count":1}],"spells":{"cantrips":["sacred-flame"],"level1":["cure-wounds"]},"combatResources":{"arrows":19,"spellSlots":{"1":0},"hitDiceRemaining":1}})}).await.unwrap();
    (
        store,
        RestCommand {
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            character_id,
            expected_revision: state.campaign.revision,
            causation_id: "rest".into(),
            intent: RestIntent::Short { hit_dice: 1 },
        },
    )
}
#[tokio::test]
async fn exact_rest_retry_preserves_rolls_time_and_resources() {
    let (store, command) = fixture().await;
    let first = store.rest(command.clone(), 42).await.unwrap();
    let retry = store.rest(command.clone(), 99).await.unwrap();
    assert_eq!(
        serde_json::to_value(first).unwrap(),
        serde_json::to_value(retry).unwrap()
    );
    let current = store.load_current(command.campaign_id).await.unwrap();
    assert_eq!(
        current.projection.adventuring.unwrap().elapsed_seconds,
        3600
    );
    assert_eq!(
        current.projection.character["combatResources"]["hitDiceRemaining"],
        0
    );
    assert_eq!(
        current.projection.character["combatResources"]["arrows"],
        19
    );
    let mut changed = command;
    changed.intent = RestIntent::Long;
    assert!(store.rest(changed, 42).await.is_err());
}
#[tokio::test]
async fn rest_receipt_failure_rolls_back_sheet_clock_and_revision() {
    let (store, command) = fixture().await;
    sqlx::query("CREATE TRIGGER reject_rest BEFORE INSERT ON rest_operations BEGIN SELECT RAISE(FAIL,'fixture'); END").execute(store.pool()).await.unwrap();
    assert!(store.rest(command.clone(), 42).await.is_err());
    let current = store.load_current(command.campaign_id).await.unwrap();
    assert_eq!(current.campaign.revision, command.expected_revision);
    assert_eq!(current.projection.character["hp"], 2);
    assert!(current.projection.adventuring.is_none());
}
#[tokio::test]
async fn restore_rewinds_the_clock_and_resources_and_invalidates_old_rest_receipts() {
    let (store, mut command) = fixture().await;
    let current = store.load_current(command.campaign_id).await.unwrap();
    let saved = CreateSnapshot::new(store.clone())
        .execute(CreateSnapshotCommand {
            id: Uuid::new_v4(),
            campaign_id: command.campaign_id,
            session_id: command.session_id,
            expected_revision: current.campaign.revision,
            title: "Before rest".into(),
            kind: "manual".into(),
            summary: "".into(),
            tag: "exploration".into(),
            causation_id: "save".into(),
            character: current.projection.character,
        })
        .await
        .unwrap();
    command.expected_revision = saved.snapshot.revision;
    command.intent = RestIntent::Long;
    let rested = store.rest(command.clone(), 42).await.unwrap();
    assert_eq!(rested.status.hp, 20);
    assert_eq!(rested.status.elapsed_seconds, 28800);
    assert_eq!(rested.status.slots[&1].remaining, 2);
    store
        .restore_snapshot(RestoreSnapshotRecord {
            save_id: saved.save_id,
            campaign_id: command.campaign_id,
            session_id: command.session_id,
            expected_revision: app_application::models::identity::Revision(rested.status.revision),
            causation_id: "restore".into(),
        })
        .await
        .unwrap();
    let status = store.status(command.campaign_id).await.unwrap();
    assert_eq!(status.elapsed_seconds, 0);
    assert_eq!(status.hp, 2);
    assert_eq!(status.slots[&1].remaining, 0);
    assert_eq!(status.long_rest_ready_in_seconds, 0);
    assert!(store.rest(command, 42).await.is_err());
}

#[tokio::test]
async fn a_sheet_update_cannot_reset_host_owned_rest_resources() {
    let (store, command) = fixture().await;
    let old = store.load_current(command.campaign_id).await.unwrap();
    store.rest(command.clone(), 42).await.unwrap();
    let current = store.load_current(command.campaign_id).await.unwrap();
    let mut supplied = old.projection.character;
    supplied["name"] = json!("Changed");
    supplied["combatResources"]["spellSlots"]["1"] = json!(2);
    let committed = store
        .commit_character(CommitCampaignCharacter {
            campaign_id: command.campaign_id,
            session_id: command.session_id,
            character_id: command.character_id,
            expected_revision: current.campaign.revision,
            causation_id: "edit".into(),
            character: supplied,
        })
        .await
        .unwrap();
    assert_eq!(
        committed.projection.character["combatResources"]["hitDiceRemaining"],
        0
    );
    assert_eq!(
        committed.projection.character["combatResources"]["spellSlots"]["1"],
        0
    );
}

#[tokio::test]
async fn a_legacy_encounter_without_a_projection_still_blocks_rest() {
    let (store, command) = fixture().await;
    sqlx::query("INSERT INTO combat_encounters(id,session_id,round,active_turn,started_at,initiative,revision,projection,ended_at) VALUES(?1,?2,1,NULL,?3,'[]',0,NULL,NULL)").bind(Uuid::new_v4().to_string()).bind(command.session_id.0.to_string()).bind("2026-10-07T00:00:00Z").execute(store.pool()).await.unwrap();
    assert!(
        store
            .status(command.campaign_id)
            .await
            .unwrap()
            .combat_active
    );
    assert!(store.rest(command, 42).await.is_err());
}

#[tokio::test]
async fn concurrent_exact_retries_share_one_roll_and_one_hour() {
    let (store, command) = fixture().await;
    let (first, second) = tokio::join!(
        store.rest(command.clone(), 42),
        store.rest(command.clone(), 43)
    );
    assert_eq!(
        serde_json::to_value(first.unwrap()).unwrap(),
        serde_json::to_value(second.unwrap()).unwrap()
    );
    assert_eq!(
        store
            .status(command.campaign_id)
            .await
            .unwrap()
            .elapsed_seconds,
        3600
    );
}

#[tokio::test]
async fn authored_travel_advances_time_once_and_unsafe_locations_reject_rest() {
    use app_application::{
        models::adventure::{Action, AdventureCommand, Location},
        ports::repositories::AdventureRepository,
    };
    let (store, mut command) = fixture_with_seed(CampaignSeed::Bundled {
        adventure_id: "bell-beneath-water".into(),
    })
    .await;
    let rested = store.rest(command.clone(), 42).await.unwrap();
    let movement = AdventureCommand {
        campaign_id: command.campaign_id,
        session_id: command.session_id,
        character_id: command.character_id,
        expected_revision: app_application::models::identity::Revision(rested.status.revision),
        causation_id: "go-quay".into(),
        action: Action::Move(Location::Quay),
    };
    store.act(movement.clone()).await.unwrap();
    store.act(movement).await.unwrap();
    let status = store.status(command.campaign_id).await.unwrap();
    assert_eq!(status.elapsed_seconds, 3900);
    assert!(!status.safe);
    command.intent = RestIntent::Long;
    command.expected_revision = app_application::models::identity::Revision(status.revision);
    command.causation_id = "unsafe-rest".into();
    assert!(store.rest(command.clone(), 42).await.is_err());
    assert_eq!(
        store
            .status(command.campaign_id)
            .await
            .unwrap()
            .elapsed_seconds,
        3900
    );
}
