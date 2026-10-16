use adapter_sqlite::{pool, SqliteStore};
use app_application::combat::commands::{CombatActionCommand, ResolveCombatCommand};
use app_application::combat::resolve::ResolveCombatAction;
use app_application::models::combat::{
    CombatProjection, CombatSnapshot, COMBAT_PROJECTION_VERSION,
};
use app_application::ports::repositories::CombatRepository;
use app_domain::combat::combatant::Combatant;
use app_domain::combat::initiative::InitiativeEntry;
use app_domain::combat::types::{CombatantId, Position};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

async fn fixture() -> (Arc<SqliteStore>, Uuid, CombatantId, CombatantId) {
    let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    fixture_with_pool(db).await
}
async fn fixture_with_pool(
    db: sqlx::SqlitePool,
) -> (Arc<SqliteStore>, Uuid, CombatantId, CombatantId) {
    pool::migrate(&db).await.unwrap();
    let store = Arc::new(SqliteStore::new(db));
    let encounter = Uuid::new_v4();
    let first = CombatantId::new();
    let second = CombatantId::new();
    let mut hero = Combatant::new(first, "Hero".into(), 20, 20, 14);
    hero.position = Position { x: 8, y: 8 };
    let mut foe = Combatant::new(second, "Foe".into(), 10, 10, 12);
    foe.position = Position { x: 9, y: 8 };
    store
        .create(
            Uuid::new_v4(),
            CombatProjection {
                authored: None,

                adventure: None,

                campaign_scope: None,

                appearances: Default::default(),
                schema_version: COMBAT_PROJECTION_VERSION,
                encounter_id: encounter,
                revision: 0,
                snapshot: CombatSnapshot {
                    active: true,
                    round: 3,
                    current_combatant: Some(first),
                    initiative: vec![
                        InitiativeEntry {
                            id: first,
                            roll: 20,
                            dex_tiebreak: 2,
                        },
                        InitiativeEntry {
                            id: second,
                            roll: 10,
                            dex_tiebreak: 1,
                        },
                    ],
                    combatants: vec![hero, foe],
                },
                events: vec![],
            },
        )
        .await
        .unwrap();
    (store, encounter, first, second)
}

#[tokio::test]
async fn file_backed_receipt_survives_pool_shutdown_and_reopening() {
    let path = std::env::temp_dir().join(format!("dmai-combat-{}.sqlite", Uuid::new_v4()));
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(&path)
        .create_if_missing(true);
    let db = sqlx::SqlitePool::connect_with(options.clone())
        .await
        .unwrap();
    let (store, encounter, actor, _) = fixture_with_pool(db).await;
    let command = movement(encounter, actor, "durable-move", 0);
    let first = ResolveCombatAction::new(store.clone())
        .execute(command.clone())
        .await
        .unwrap();
    store.pool().close().await;
    let reopened = Arc::new(SqliteStore::new(
        sqlx::SqlitePool::connect_with(options).await.unwrap(),
    ));
    let replay = ResolveCombatAction::new(reopened.clone())
        .execute(command)
        .await
        .unwrap();
    assert!(replay.duplicate);
    assert_eq!(
        serde_json::to_value(first.projection).unwrap(),
        serde_json::to_value(replay.projection).unwrap()
    );
    reopened.pool().close().await;
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn receipt_insert_failure_rolls_back_projection_and_token_movement() {
    let (store, encounter, actor, _) = fixture().await;
    sqlx::query("CREATE TRIGGER fail_combat_receipt BEFORE INSERT ON combat_action_receipts BEGIN SELECT RAISE(ABORT,'forced receipt failure'); END").execute(store.pool()).await.unwrap();
    assert!(ResolveCombatAction::new(store.clone())
        .execute(movement(encounter, actor, "fails", 0))
        .await
        .is_err());
    let projection = store.get(encounter).await.unwrap().unwrap();
    assert_eq!(projection.revision, 0);
    assert_eq!(
        projection.snapshot.combatants[0].position,
        Position { x: 8, y: 8 }
    );
    let row = sqlx::query("SELECT pos_x,pos_y FROM combat_tokens WHERE id=?1")
        .bind(actor.0.to_string())
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(
        (row.get::<i32, _>("pos_x"), row.get::<i32, _>("pos_y")),
        (8, 8)
    );
}

#[tokio::test]
async fn restore_advances_combat_revision_and_rejects_pre_restore_intentions() {
    use app_application::campaigns::{
        CreateCampaign, CreateCampaignCommand, CreateSnapshot, CreateSnapshotCommand,
        RestoreSnapshot, RestoreSnapshotCommand,
    };
    use app_application::models::session::CampaignSeed;
    let (store, encounter, actor, _) = fixture().await;
    let campaign = CreateCampaign::new(store.clone())
        .execute(CreateCampaignCommand {
            request_id: None,
            title: "Bell".into(),
            premise: "Ferry".into(),
            seed: CampaignSeed::Bundled {
                adventure_id: "bell-beneath-water".into(),
            },
        })
        .await
        .unwrap();
    sqlx::query("UPDATE combat_encounters SET session_id=?1 WHERE id=?2")
        .bind(campaign.last_session_id.0.to_string())
        .bind(encounter.to_string())
        .execute(store.pool())
        .await
        .unwrap();
    let save = CreateSnapshot::new(store.clone())
        .execute(CreateSnapshotCommand {
            id: Uuid::new_v4(),
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            expected_revision: campaign.revision,
            title: "Before move".into(),
            kind: "manual".into(),
            summary: "".into(),
            tag: "combat".into(),
            causation_id: "combat-save".into(),
            character: serde_json::json!({}),
        })
        .await
        .unwrap();
    let command = movement(encounter, actor, "old-move", 0);
    ResolveCombatAction::new(store.clone())
        .execute(command.clone())
        .await
        .unwrap();
    RestoreSnapshot::new(store.clone())
        .execute(RestoreSnapshotCommand {
            save_id: save.save_id,
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            expected_revision: save.snapshot.revision,
            causation_id: "combat-restore".into(),
        })
        .await
        .unwrap();
    let restored = store.get(encounter).await.unwrap().unwrap();
    assert_eq!(
        restored.snapshot.combatants[0].position,
        Position { x: 8, y: 8 }
    );
    assert!(
        restored.revision > 1,
        "restored encounter must invalidate old expected revisions"
    );
    assert!(ResolveCombatAction::new(store.clone())
        .execute(command)
        .await
        .is_err());
    assert_eq!(
        store
            .get(encounter)
            .await
            .unwrap()
            .unwrap()
            .snapshot
            .combatants[0]
            .position,
        Position { x: 8, y: 8 }
    );
}
#[tokio::test]
async fn restore_without_combat_or_with_another_encounter_invalidates_old_receipts() {
    use app_application::campaigns::{
        CreateCampaign, CreateCampaignCommand, CreateSnapshot, CreateSnapshotCommand,
        RestoreSnapshot, RestoreSnapshotCommand,
    };
    use app_application::models::session::CampaignSeed;
    for with_other_encounter in [false, true] {
        let (store, encounter, actor, _) = fixture().await;
        let campaign = CreateCampaign::new(store.clone())
            .execute(CreateCampaignCommand {
                request_id: None,
                title: "Bell".into(),
                premise: "Ferry".into(),
                seed: CampaignSeed::Bundled {
                    adventure_id: "bell-beneath-water".into(),
                },
            })
            .await
            .unwrap();
        if with_other_encounter {
            let mut other = store.get(encounter).await.unwrap().unwrap();
            other.encounter_id = Uuid::new_v4();
            let ids = [CombatantId::new(), CombatantId::new()];
            for (index, combatant) in other.snapshot.combatants.iter_mut().enumerate() {
                combatant.id = ids[index];
                other.snapshot.initiative[index].id = ids[index];
            }
            other.snapshot.current_combatant = Some(ids[0]);
            store
                .create(campaign.last_session_id.0, other)
                .await
                .unwrap();
        }
        let save = CreateSnapshot::new(store.clone())
            .execute(CreateSnapshotCommand {
                id: Uuid::new_v4(),
                campaign_id: campaign.id,
                session_id: campaign.last_session_id,
                expected_revision: campaign.revision,
                title: "Earlier timeline".into(),
                kind: "manual".into(),
                summary: "".into(),
                tag: "combat".into(),
                causation_id: "save".into(),
                character: serde_json::json!({}),
            })
            .await
            .unwrap();
        sqlx::query("UPDATE combat_encounters SET session_id=?1 WHERE id=?2")
            .bind(campaign.last_session_id.0.to_string())
            .bind(encounter.to_string())
            .execute(store.pool())
            .await
            .unwrap();
        let command = movement(encounter, actor, "old-timeline-move", 0);
        ResolveCombatAction::new(store.clone())
            .execute(command.clone())
            .await
            .unwrap();
        RestoreSnapshot::new(store.clone())
            .execute(RestoreSnapshotCommand {
                save_id: save.save_id,
                campaign_id: campaign.id,
                session_id: campaign.last_session_id,
                expected_revision: save.snapshot.revision,
                causation_id: "restore".into(),
            })
            .await
            .unwrap();
        assert!(ResolveCombatAction::new(store.clone()).execute(command).await.is_err(),"old receipt must be invalidated even if the restored save has no combat or another encounter");
    }
}

fn movement(encounter: Uuid, actor: CombatantId, id: &str, revision: u64) -> ResolveCombatCommand {
    ResolveCombatCommand {
        request_id: id.into(),
        encounter_id: encounter,
        expected_revision: revision,
        rng_seed: 42,
        action: CombatActionCommand::Move {
            combatant: actor,
            to: Position { x: 8, y: 9 },
        },
    }
}

#[tokio::test]
async fn replay_survives_new_use_case_and_changed_intention_is_rejected() {
    let (store, encounter, actor, _) = fixture().await;
    let command = movement(encounter, actor, "move-1", 0);
    let first = ResolveCombatAction::new(store.clone())
        .execute(command.clone())
        .await
        .unwrap();
    let reopened = ResolveCombatAction::new(Arc::new(SqliteStore::new(store.pool().clone())));
    let replay = reopened
        .execute(command.clone())
        .await
        .expect("receipt survives resolver recreation");
    assert!(replay.duplicate);
    assert_eq!(
        serde_json::to_value(first.projection).unwrap(),
        serde_json::to_value(replay.projection).unwrap()
    );
    let changed = ResolveCombatCommand {
        action: CombatActionCommand::Move {
            combatant: actor,
            to: Position { x: 8, y: 10 },
        },
        ..command
    };
    assert!(
        reopened.execute(changed).await.is_err(),
        "same ID must not acknowledge another intention"
    );
    assert_eq!(store.get(encounter).await.unwrap().unwrap().revision, 1);
}

#[tokio::test]
async fn round_and_token_coordinates_survive_each_committed_action() {
    let (store, encounter, actor, _) = fixture().await;
    let initial = sqlx::query("SELECT pos_x,pos_y FROM combat_tokens WHERE id=?1")
        .bind(actor.0.to_string())
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(
        (
            initial.get::<i32, _>("pos_x"),
            initial.get::<i32, _>("pos_y")
        ),
        (8, 8)
    );
    let result = ResolveCombatAction::new(store.clone())
        .execute(movement(encounter, actor, "move-1", 0))
        .await
        .unwrap();
    assert_eq!(
        result.projection.snapshot.round, 3,
        "reconstructing initiative must preserve round"
    );
    assert_eq!(result.projection.snapshot.current_combatant, Some(actor));
    let moved = sqlx::query("SELECT pos_x,pos_y FROM combat_tokens WHERE id=?1")
        .bind(actor.0.to_string())
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(
        (moved.get::<i32, _>("pos_x"), moved.get::<i32, _>("pos_y")),
        (8, 9)
    );
}

#[tokio::test]
async fn concurrent_duplicates_have_one_effect_and_one_receipt() {
    let (store, encounter, actor, _) = fixture().await;
    let command = movement(encounter, actor, "same", 0);
    let a = ResolveCombatAction::new(store.clone());
    let b = ResolveCombatAction::new(store.clone());
    let (a, b) = tokio::join!(a.execute(command.clone()), b.execute(command));
    let (a, b) = (a.unwrap(), b.unwrap());
    assert_ne!(a.duplicate, b.duplicate);
    assert_eq!(
        serde_json::to_value(a.projection).unwrap(),
        serde_json::to_value(b.projection).unwrap()
    );
    assert_eq!(store.get(encounter).await.unwrap().unwrap().revision, 1);
}

#[tokio::test]
async fn finishing_a_round_preserves_the_next_round_for_following_actions() {
    let (store, encounter, first, second) = fixture().await;
    let service = ResolveCombatAction::new(store.clone());
    for (revision, actor) in [(0, first), (1, second)] {
        let result = service
            .execute(ResolveCombatCommand {
                request_id: format!("end-{revision}"),
                encounter_id: encounter,
                expected_revision: revision,
                rng_seed: 0,
                action: CombatActionCommand::EndTurn { combatant: actor },
            })
            .await
            .unwrap();
        assert_eq!(
            result.projection.snapshot.round,
            if revision == 0 { 3 } else { 4 }
        );
    }
    let result = service
        .execute(movement(encounter, first, "next-move", 2))
        .await
        .unwrap();
    assert_eq!(result.projection.snapshot.round, 4);
}
