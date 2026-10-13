use adapter_sqlite::{pool, SqliteStore};
use app_application::campaigns::{CreateCampaign, CreateCampaignCommand};
use app_application::models::adventure::AdventureCommand;
use app_application::models::adventure::{Action, Fact, Location};
use app_application::models::campaign_state::CommitCampaignCharacter;
use app_application::models::session::CampaignSeed;
use app_application::models::snapshot::{CreateSnapshotRecord, RestoreSnapshotRecord};
use app_application::ports::repositories::{
    AdventureRepository, CampaignStateRepository, RestRepository, SnapshotRepository,
};
use serde_json::json;
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn adventure_is_authoritative_retry_safe_and_restored_with_its_reward() {
    rescue_contract(Action::RescueTikhon).await;
    rescue_contract(Action::EscortTikhonHome).await;
}
async fn rescue_contract(rescue_action: Action) {
    let db = SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&db).await.unwrap();
    let store = Arc::new(SqliteStore::new(db));
    let campaign = CreateCampaign::new(store.clone())
        .execute(CreateCampaignCommand {
            request_id: None,
            title: "Bell".into(),
            premise: "Missing ferry".into(),
            seed: CampaignSeed::Bundled {
                adventure_id: "bell-beneath-water".into(),
            },
        })
        .await
        .unwrap();
    let hero = Uuid::new_v4();
    let mut current=store.commit_character(CommitCampaignCharacter{
        campaign_id:campaign.id, session_id:campaign.last_session_id, character_id:hero,
        expected_revision:campaign.revision, causation_id:"hero".into(),
        character:json!({"name":"Mira","heroClass":"fighter","level":1,"hp":12,"hpMax":12,"ac":14,
        "abilities":{"str":15,"dex":14,"con":13,"int":12,"wis":10,"cha":8},"inventory":[]})
    }).await.unwrap();
    let command = AdventureCommand {
        campaign_id: campaign.id,
        session_id: campaign.last_session_id,
        character_id: hero,
        expected_revision: current.campaign.revision,
        causation_id: "move-quay".into(),
        action: Action::Move(Location::Quay),
    };
    current = store.act(command.clone()).await.unwrap();
    assert_eq!(
        store.act(command.clone()).await.unwrap().projection,
        current.projection
    );
    let mut changed = command.clone();
    changed.action = Action::TalkAgata;
    assert!(store.act(changed).await.is_err());
    let mut stale = command;
    stale.causation_id = "stale".into();
    assert!(store.act(stale).await.is_err());
    for (index, action) in [Action::Move(Location::Archive), Action::LocateTikhon]
        .into_iter()
        .enumerate()
    {
        current = store
            .act(AdventureCommand {
                campaign_id: campaign.id,
                session_id: campaign.last_session_id,
                character_id: hero,
                expected_revision: current.campaign.revision,
                causation_id: format!("step-{index}"),
                action,
            })
            .await
            .unwrap();
    }
    let checkpoint = store
        .create_snapshot(CreateSnapshotRecord {
            id: Uuid::new_v4(),
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            expected_revision: current.campaign.revision,
            schema_version: 1,
            ruleset_id: "srd-5.2.1".into(),
            title: "Before rescue".into(),
            kind: "manual".into(),
            summary: String::new(),
            tag: String::new(),
            causation_id: "before-rescue".into(),
            character: json!({}),
        })
        .await
        .unwrap();
    let discovered_portraits = current
        .projection
        .adventure
        .as_ref()
        .unwrap()
        .public_npcs
        .clone();
    let saved_scene_asset = current
        .projection
        .adventure
        .as_ref()
        .unwrap()
        .scene_asset_id
        .clone();
    assert_eq!(saved_scene_asset.as_deref(), Some("scene.bell.archive.v2"));
    assert_eq!(
        current
            .projection
            .adventure
            .as_ref()
            .unwrap()
            .map_skin_asset_id
            .as_deref(),
        Some("map.bell_archive.v2")
    );
    assert!(discovered_portraits.iter().any(|npc| npc.id == "tikhon"));
    assert!(!discovered_portraits.iter().any(|npc| npc.id == "savely"));
    let rescue = AdventureCommand {
        campaign_id: campaign.id,
        session_id: campaign.last_session_id,
        character_id: hero,
        expected_revision: checkpoint.snapshot.revision,
        causation_id: "rescue".into(),
        action: rescue_action,
    };
    let clock_before = store.status(campaign.id).await.unwrap().elapsed_seconds;
    current = store.act(rescue.clone()).await.unwrap();
    let clock_after = store.status(campaign.id).await.unwrap().elapsed_seconds;
    assert_eq!(clock_after, clock_before + rescue_action.elapsed_seconds());
    let adventure = current.projection.adventure.as_ref().unwrap();
    assert_eq!(
        adventure.state.location,
        if rescue_action == Action::EscortTikhonHome {
            Location::Town
        } else {
            Location::Archive
        }
    );
    assert_eq!(
        adventure.map.is_none(),
        rescue_action == Action::EscortTikhonHome
    );
    assert_eq!(current.projection.character["inventory"][0]["count"], 10);
    assert!(current
        .projection
        .adventure
        .as_ref()
        .unwrap()
        .state
        .facts
        .contains(&Fact::TikhonRescued));
    assert_eq!(
        store.act(rescue).await.unwrap().projection.character["inventory"][0]["count"],
        10
    );
    assert_eq!(
        store.status(campaign.id).await.unwrap().elapsed_seconds,
        clock_after
    );
    let mut wrong = AdventureCommand {
        campaign_id: campaign.id,
        session_id: campaign.last_session_id,
        character_id: Uuid::new_v4(),
        expected_revision: current.campaign.revision,
        causation_id: "wrong-owner".into(),
        action: Action::RecoverLedger,
    };
    assert!(store.act(wrong.clone()).await.is_err());
    wrong.character_id = hero;
    wrong.session_id = app_application::models::identity::SessionId(Uuid::new_v4());
    assert!(store.act(wrong).await.is_err());
    store
        .restore_snapshot(RestoreSnapshotRecord {
            save_id: checkpoint.save_id,
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            expected_revision: current.campaign.revision,
            causation_id: "rewind".into(),
        })
        .await
        .unwrap();
    let restored = store.load_current(campaign.id).await.unwrap();
    assert_eq!(
        restored
            .projection
            .adventure
            .as_ref()
            .unwrap()
            .map_skin_asset_id
            .as_deref(),
        Some("map.bell_archive.v2")
    );
    assert_eq!(
        restored
            .projection
            .adventure
            .as_ref()
            .unwrap()
            .scene_asset_id,
        saved_scene_asset
    );
    assert_eq!(
        restored.projection.adventure.as_ref().unwrap().public_npcs,
        discovered_portraits
    );
    assert_eq!(restored.projection.character["inventory"], json!([]));
    assert!(!restored
        .projection
        .adventure
        .unwrap()
        .state
        .facts
        .contains(&Fact::TikhonRescued));
    let stale_before_restore = AdventureCommand {
        campaign_id: campaign.id,
        session_id: campaign.last_session_id,
        character_id: hero,
        expected_revision: current.campaign.revision,
        causation_id: "delayed-before-restore".into(),
        action: Action::Move(Location::Belfry),
    };
    assert!(
        store.act(stale_before_restore).await.is_err(),
        "restore must invalidate already pending writes"
    );
    let before_failure = store.load_current(campaign.id).await.unwrap();
    sqlx::query("CREATE TRIGGER fail_adventure_receipt BEFORE INSERT ON adventure_operations BEGIN SELECT RAISE(ABORT, 'forced receipt failure'); END")
        .execute(store.pool()).await.unwrap();
    let failed = store
        .act(AdventureCommand {
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            character_id: hero,
            expected_revision: before_failure.campaign.revision,
            causation_id: "rollback-rescue".into(),
            action: rescue_action,
        })
        .await;
    assert!(failed.is_err());
    let after_failure = store.load_current(campaign.id).await.unwrap();
    assert_eq!(after_failure.projection, before_failure.projection);
    assert_eq!(
        after_failure.campaign.revision,
        before_failure.campaign.revision
    );
    let receipts: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM adventure_operations WHERE causation_id='rollback-rescue'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(receipts, 0);
}
