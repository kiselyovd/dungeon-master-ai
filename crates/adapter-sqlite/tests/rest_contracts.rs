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
            request_id: None,
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

async fn recovery_fixture(
    stable: bool,
) -> (
    Arc<SqliteStore>,
    app_application::models::solo_recovery::SoloRecoveryCommand,
) {
    let (store, rest) = fixture().await;
    let mut state = store.load_current(rest.campaign_id).await.unwrap();
    let mut actor =
        app_application::recovery::actor(&state.projection.character, rest.character_id).unwrap();
    actor.current_hp = 0;
    if stable {
        actor.death_saves.stabilize();
    }
    app_application::vitality::store_actor(&mut state.projection.character, &actor).unwrap();
    sqlx::query("UPDATE campaign_characters SET character_json=?1 WHERE campaign_id=?2")
        .bind(state.projection.character.to_string())
        .bind(rest.campaign_id.0.to_string())
        .execute(store.pool())
        .await
        .unwrap();
    (
        store,
        app_application::models::solo_recovery::SoloRecoveryCommand {
            campaign_id: rest.campaign_id,
            session_id: rest.session_id,
            character_id: rest.character_id,
            expected_revision: rest.expected_revision,
            causation_id: "recover".into(),
            intent: if stable {
                app_application::models::room_recovery::RecoveryIntent::Wait {
                    target_character_id: rest.character_id,
                }
            } else {
                app_application::models::room_recovery::RecoveryIntent::DeathSave {}
            },
        },
    )
}
#[tokio::test]
async fn solo_stable_wait_replays_the_deadline_and_continues_to_normal_rest() {
    use app_application::ports::repositories::SoloRecoveryRepository;
    let (store, command) = recovery_fixture(true).await;
    let before = store.recovery_status(command.campaign_id).await.unwrap();
    assert_eq!(before.hp, 0);
    assert_eq!(before.recovery.actions.len(), 1);
    let result = store.recover_solo(command.clone(), 42).await.unwrap();
    assert_eq!(result.status.hp, 1);
    assert!(result.receipt.recovered);
    assert!([3600, 7200, 10800, 14400].contains(&result.status.elapsed_seconds));
    let retry = store.recover_solo(command.clone(), 99).await.unwrap();
    assert!(retry.receipt.duplicate);
    assert_eq!(
        serde_json::to_value(retry.status).unwrap(),
        serde_json::to_value(&result.status).unwrap()
    );
    let rested = store
        .rest(
            RestCommand {
                campaign_id: command.campaign_id,
                session_id: command.session_id,
                character_id: command.character_id,
                expected_revision: app_application::models::identity::Revision(
                    result.status.revision,
                ),
                causation_id: "after-recovery".into(),
                intent: RestIntent::Long,
            },
            42,
        )
        .await
        .unwrap();
    assert_eq!(rested.status.hp, 20);
    let mut changed = command;
    changed.intent = app_application::models::room_recovery::RecoveryIntent::DeathSave {};
    assert!(store.recover_solo(changed, 42).await.is_err());
}
#[tokio::test]
async fn solo_death_save_is_one_server_roll_and_exact_retry_keeps_it() {
    use app_application::ports::repositories::SoloRecoveryRepository;
    let (store, command) = recovery_fixture(false).await;
    let result = store.recover_solo(command.clone(), 42).await.unwrap();
    assert_eq!(result.receipt.death_saves.len(), 1);
    assert_eq!(result.status.elapsed_seconds, 6);
    let retry = store.recover_solo(command.clone(), 99).await.unwrap();
    assert!(retry.receipt.duplicate);
    assert_eq!(
        serde_json::to_value(retry.receipt.death_saves).unwrap(),
        serde_json::to_value(result.receipt.death_saves).unwrap()
    );
    assert_eq!(
        store
            .recovery_status(command.campaign_id)
            .await
            .unwrap()
            .revision,
        result.status.revision
    );
}
#[tokio::test]
async fn solo_recovery_final_receipt_failure_rolls_back_timer_hp_and_clock() {
    use app_application::ports::repositories::SoloRecoveryRepository;
    let (store, command) = recovery_fixture(true).await;
    let before = store.load_current(command.campaign_id).await.unwrap();
    sqlx::query("CREATE TRIGGER fail_solo_recovery BEFORE INSERT ON solo_recovery_receipts BEGIN SELECT RAISE(ABORT,'test'); END").execute(store.pool()).await.unwrap();
    assert!(store.recover_solo(command.clone(), 42).await.is_err());
    let after = store.load_current(command.campaign_id).await.unwrap();
    assert_eq!(after.campaign.revision, before.campaign.revision);
    assert_eq!(after.projection.character, before.projection.character);
    assert_eq!(after.projection.adventuring, before.projection.adventuring);
}
#[tokio::test]
async fn solo_recovery_rejects_foreign_identity_and_self_aid_without_mutation() {
    use app_application::{
        models::room_recovery::RecoveryIntent, ports::repositories::SoloRecoveryRepository,
    };
    let (store, command) = recovery_fixture(false).await;
    let before = store.load_current(command.campaign_id).await.unwrap();
    for mode in 0..4 {
        let mut bad = command.clone();
        match mode {
            0 => bad.character_id = Uuid::new_v4(),
            1 => bad.session_id = app_application::models::identity::SessionId(Uuid::new_v4()),
            2 => {
                bad.intent = RecoveryIntent::Aid {
                    target_character_id: bad.character_id,
                }
            }
            _ => {
                bad.intent = RecoveryIntent::Wait {
                    target_character_id: Uuid::new_v4(),
                }
            }
        }
        assert!(store.recover_solo(bad, 42).await.is_err());
    }
    let after = store.load_current(command.campaign_id).await.unwrap();
    assert_eq!(after.campaign.revision, before.campaign.revision);
    assert_eq!(after.projection.character, before.projection.character);
}
#[tokio::test]
async fn solo_recovery_restore_invalidates_the_old_receipt_and_turn() {
    use app_application::ports::repositories::SoloRecoveryRepository;
    let (store, mut command) = recovery_fixture(true).await;
    let state = store.load_current(command.campaign_id).await.unwrap();
    let saved = CreateSnapshot::new(store.clone())
        .execute(CreateSnapshotCommand {
            id: Uuid::new_v4(),
            campaign_id: command.campaign_id,
            session_id: command.session_id,
            expected_revision: state.campaign.revision,
            title: "Before recovery".into(),
            kind: "manual".into(),
            summary: "".into(),
            tag: "recovery".into(),
            causation_id: "save-recovery".into(),
            character: state.projection.character,
        })
        .await
        .unwrap();
    command.expected_revision = saved.snapshot.revision;
    let recovered = store.recover_solo(command.clone(), 42).await.unwrap();
    store
        .restore_snapshot(RestoreSnapshotRecord {
            save_id: saved.save_id,
            campaign_id: command.campaign_id,
            session_id: command.session_id,
            expected_revision: app_application::models::identity::Revision(
                recovered.status.revision,
            ),
            causation_id: "restore-recovery".into(),
        })
        .await
        .unwrap();
    assert!(store.recover_solo(command.clone(), 99).await.is_err());
    assert_eq!(
        store.recovery_status(command.campaign_id).await.unwrap().hp,
        0
    );
}

#[tokio::test]
async fn recovery_and_rest_receipts_have_independent_nonce_spaces() {
    use app_application::ports::repositories::SoloRecoveryRepository;
    let (store, mut command) = recovery_fixture(true).await;
    command.causation_id = "same-nonce".into();
    let result = store.recover_solo(command.clone(), 42).await.unwrap();
    let rested = store
        .rest(
            RestCommand {
                campaign_id: command.campaign_id,
                session_id: command.session_id,
                character_id: command.character_id,
                expected_revision: app_application::models::identity::Revision(
                    result.status.revision,
                ),
                causation_id: command.causation_id.clone(),
                intent: RestIntent::Long,
            },
            42,
        )
        .await
        .unwrap();
    assert_eq!(rested.status.hp, 20);
    assert!(
        store
            .recover_solo(command, 99)
            .await
            .unwrap()
            .receipt
            .duplicate
    );
}

#[tokio::test]
async fn solo_dead_and_active_combat_states_have_no_recovery_actions() {
    use app_application::ports::repositories::SoloRecoveryRepository;
    for dead in [false, true] {
        let (store, command) = recovery_fixture(false).await;
        if dead {
            let state = store.load_current(command.campaign_id).await.unwrap();
            let mut sheet = state.projection.character;
            let mut actor = app_application::recovery::actor(&sheet, command.character_id).unwrap();
            actor.is_dead = true;
            actor.death_saves.failures = 3;
            app_application::vitality::store_actor(&mut sheet, &actor).unwrap();
            sqlx::query("UPDATE campaign_characters SET character_json=?1 WHERE campaign_id=?2")
                .bind(sheet.to_string())
                .bind(command.campaign_id.0.to_string())
                .execute(store.pool())
                .await
                .unwrap();
        } else {
            sqlx::query("INSERT INTO combat_encounters(id,session_id,round,active_turn,started_at,initiative,revision,projection,ended_at) VALUES(?1,?2,1,NULL,?3,'[]',0,NULL,NULL)").bind(Uuid::new_v4().to_string()).bind(command.session_id.0.to_string()).bind("2026-10-08T00:00:00Z").execute(store.pool()).await.unwrap();
        }
        assert!(store
            .recovery_status(command.campaign_id)
            .await
            .unwrap()
            .recovery
            .actions
            .is_empty());
        assert!(store.recover_solo(command.clone(), 42).await.is_err());
        assert_eq!(
            store
                .load_current(command.campaign_id)
                .await
                .unwrap()
                .campaign
                .revision,
            command.expected_revision
        );
    }
}
