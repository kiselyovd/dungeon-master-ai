use adapter_sqlite::{pool, SqliteStore};
use app_application::models::campaign::{CampaignSave, JournalEntry, NpcRecord, SceneRecord};
use app_application::models::chat::{ChatMessage, ToolCall, ToolResult};
use app_application::models::combat::{
    CombatProjection, CombatSnapshot, COMBAT_PROJECTION_VERSION,
};
use app_application::models::identity::{CampaignId, Revision, SessionId};
use app_application::models::session::{
    CampaignDetail, CampaignEvent, CampaignEventKind, CampaignSeed, CampaignSession,
    InitialCampaign,
};
use app_application::models::snapshot::{
    CreateSnapshotRecord, RestoreSnapshotRecord, SNAPSHOT_SCHEMA_VERSION,
};
use app_application::ports::repositories::{
    CampaignRepository, CombatRepository, JournalRepository, MessageRepository, NpcRepository,
    SaveRepository, SceneRepository, SnapshotRepository, SrdRepository,
};
use app_domain::combat::combatant::Combatant;
use app_domain::combat::initiative::InitiativeEntry;
use app_domain::combat::types::CombatantId;
use serde_json::json;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

async fn store() -> SqliteStore {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&pool).await.unwrap();
    SqliteStore::new(pool)
}

async fn campaign(store: &SqliteStore, revision: u64) -> (CampaignId, SessionId) {
    let campaign_id = CampaignId(Uuid::new_v4());
    let session_id = SessionId(Uuid::new_v4());
    let now = 1_700_000_000_000;
    let initial = InitialCampaign {
        detail: CampaignDetail {
            id: campaign_id,
            schema_version: 1,
            ruleset_id: "srd-5.2.1".into(),
            title: "Moonlit Gate".into(),
            premise: "A sealed road".into(),
            seed: CampaignSeed::Collaborative {
                answers: Default::default(),
            },
            last_session_id: session_id,
            revision: Revision(1),
            created_at_epoch_ms: now,
            updated_at_epoch_ms: now,
        },
        session: CampaignSession {
            id: session_id,
            campaign_id,
            revision: Revision(1),
            created_at_epoch_ms: now,
            updated_at_epoch_ms: now,
        },
        event: CampaignEvent {
            event_id: Uuid::new_v4(),
            campaign_id,
            revision: Revision(1),
            kind: CampaignEventKind::new("campaign_created"),
            payload: json!({"seed": {"kind": "collaborative", "answers": {}}}),
            causation_id: Some("campaign-create".into()),
            created_at_epoch_ms: now,
        },
    };
    CampaignRepository::create(store, initial).await.unwrap();
    if revision != 1 {
        sqlx::query("UPDATE sessions SET revision = ?1 WHERE id = ?2")
            .bind(revision as i64)
            .bind(session_id.0.to_string())
            .execute(store.pool())
            .await
            .unwrap();
    }
    (campaign_id, session_id)
}

fn create_snapshot_record(
    campaign_id: CampaignId,
    session_id: SessionId,
    revision: Revision,
    causation_id: &str,
) -> CreateSnapshotRecord {
    CreateSnapshotRecord {
        id: Uuid::new_v4(),
        campaign_id,
        session_id,
        expected_revision: revision,
        schema_version: SNAPSHOT_SCHEMA_VERSION,
        ruleset_id: "srd-5.2.1".into(),
        title: "At the gate".into(),
        kind: "manual".into(),
        summary: "Before entering".into(),
        tag: "exploration".into(),
        causation_id: causation_id.into(),
        character: json!({"name": "Mira", "hp": 12}),
    }
}

fn combat_projection(encounter_id: Uuid, combatant_id: CombatantId) -> CombatProjection {
    CombatProjection {
        authored: None,

        adventure: None,

        campaign_scope: None,
        schema_version: COMBAT_PROJECTION_VERSION,
        encounter_id,
        revision: 0,
        snapshot: CombatSnapshot {
            active: true,
            round: 1,
            current_combatant: Some(combatant_id),
            initiative: vec![InitiativeEntry {
                id: combatant_id,
                roll: 10,
                dex_tiebreak: 1,
            }],
            combatants: vec![Combatant::new(combatant_id, "Hero".into(), 10, 10, 12)],
        },
        events: vec![],
    }
}

#[tokio::test]
async fn migrations_are_embedded_idempotent_and_include_authoritative_projection_schema() {
    let store = store().await;
    pool::migrate(store.pool()).await.unwrap();
    let migration_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(migration_count, 20);
    let scoped:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='trigger' AND name='room_agent_message_published')")
        .fetch_one(store.pool()).await.unwrap();
    assert!(scoped);
    let broad:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='trigger' AND name='room_message_created')")
        .fetch_one(store.pool()).await.unwrap();
    assert!(!broad);

    let tables = sqlx::query_scalar::<_, String>(
        "SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name",
    )
    .fetch_all(store.pool())
    .await
    .unwrap();
    for required in [
        "campaign_parties",
        "party_characters",
        "party_operation_receipts",
        "coop_rooms",
        "room_creation_receipts",
        "room_join_receipts",
        "room_departure_receipts",
        "room_return_receipts",
        "room_character_proposals",
        "room_character_approvals",
        "room_changes",
        "shared_messages",
        "room_restore_guard",
        "room_world_receipts",
        "room_recovery_receipts",
        "room_dm_turns",
        "room_dm_events",
        "room_agent_tools",
        "room_agent_messages",
        "campaigns",
        "sessions",
        "campaign_events",
        "campaign_snapshots",
        "campaign_adventures",
        "adventure_operations",
        "adventuring_states",
        "rest_operations",
        "solo_recovery_receipts",
        "agent_turn_operations",
        "agent_turn_events",
        "agent_turn_messages",
        "agent_tool_steps",
        "messages",
        "saves",
        "scenes",
        "combat",
        "combat_tokens",
        "combat_action_receipts",
        "journal_entries",
        "npc_memory",
        "srd_chunks",
    ] {
        assert!(
            tables.iter().any(|table| table == required),
            "missing {required}"
        );
    }

    let campaign_columns = sqlx::query("PRAGMA table_info(campaigns)")
        .fetch_all(store.pool())
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get::<String, _>("name"))
        .collect::<Vec<_>>();
    for required in ["schema_version", "ruleset_id", "title", "premise"] {
        assert!(
            campaign_columns.iter().any(|column| column == required),
            "missing campaigns.{required}"
        );
    }

    let invalid_ruleset = sqlx::query(
        "INSERT INTO campaigns \
         (id, schema_version, ruleset_id, title, premise, created_at, updated_at) \
         VALUES (?1, 1, 'srd-5.1', 'Wrong ruleset', '', ?2, ?2)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind("2026-08-13T00:00:00Z")
    .execute(store.pool())
    .await;
    assert!(invalid_ruleset.is_err());

    let combat_columns = sqlx::query("PRAGMA table_info(combat_encounters)")
        .fetch_all(store.pool())
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get::<String, _>("name"))
        .collect::<Vec<_>>();
    assert!(combat_columns.contains(&"revision".to_string()));
    assert!(combat_columns.contains(&"projection".to_string()));
}

#[tokio::test]
async fn messages_round_trip_every_stable_role_shape() {
    let store = store().await;
    let session_id = Uuid::new_v4();
    let messages = vec![
        ChatMessage::System {
            content: "system".into(),
        },
        ChatMessage::user_text("user"),
        ChatMessage::Assistant {
            content: "assistant".into(),
        },
        ChatMessage::AssistantWithToolCalls {
            content: Some("calling".into()),
            tool_calls: vec![ToolCall {
                id: "call-1".into(),
                name: "roll_dice".into(),
                args: json!({"dice":"1d20"}),
            }],
        },
        ChatMessage::ToolResult(ToolResult {
            tool_call_id: "call-1".into(),
            content: "{\"total\":12}".into(),
            is_error: false,
        }),
    ];
    for message in &messages {
        store.append(session_id, message.clone()).await.unwrap();
    }
    let restored = MessageRepository::list(&store, session_id).await.unwrap();
    assert_eq!(
        restored
            .into_iter()
            .map(|row| row.message)
            .collect::<Vec<_>>(),
        messages
    );
}

#[tokio::test]
async fn campaign_repositories_round_trip_without_cross_entity_partial_state() {
    let store = store().await;
    let campaign_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let scene = SceneRecord {
        id: Uuid::new_v4(),
        campaign_id,
        name: "Crypt".into(),
        description: "Cold stone".into(),
    };
    store.set_current(scene.clone()).await.unwrap();
    assert_eq!(
        store.current(campaign_id).await.unwrap(),
        Some(scene.clone())
    );

    let journal = JournalEntry {
        id: Uuid::new_v4(),
        campaign_id,
        title: "Chapter 1".into(),
        body: "Entered the crypt".into(),
        updated_at_epoch_ms: 1_700_000_000_000,
    };
    JournalRepository::put(&store, journal.clone())
        .await
        .unwrap();
    assert_eq!(
        JournalRepository::list(&store, campaign_id).await.unwrap(),
        vec![journal]
    );

    let npc = NpcRecord {
        id: Uuid::new_v4(),
        campaign_id,
        name: "Mara".into(),
        description: "Keeper".into(),
        updated_at_epoch_ms: 1_700_000_000_000,
    };
    NpcRepository::put(&store, npc.clone()).await.unwrap();
    assert_eq!(
        NpcRepository::list(&store, campaign_id).await.unwrap(),
        vec![npc]
    );

    sqlx::query("INSERT INTO srd_chunks (id, source_key, text_en) VALUES (?1, ?2, ?3)")
        .bind(Uuid::new_v4().to_string())
        .bind("rule.grapple")
        .bind("Grapple uses Athletics")
        .execute(store.pool())
        .await
        .unwrap();
    assert_eq!(store.search("Athletics", 3).await.unwrap().len(), 1);

    let save = CampaignSave {
        id: Uuid::new_v4(),
        campaign_id,
        session_id,
        name: "Before crypt".into(),
        messages: vec![],
        combat: None,
        scene: Some(scene),
    };
    SaveRepository::put(&store, save.clone()).await.unwrap();
    let restored = SaveRepository::get(&store, save.id).await.unwrap().unwrap();
    assert_eq!(restored.id, save.id);
    assert_eq!(restored.name, save.name);
}

#[tokio::test]
async fn combat_compare_and_set_is_atomic_and_rejects_stale_writers() {
    let store = store().await;
    let encounter_id = Uuid::new_v4();
    let combatant_id = CombatantId(Uuid::new_v4());
    let projection = combat_projection(encounter_id, combatant_id);
    CombatRepository::create(&store, Uuid::new_v4(), projection.clone())
        .await
        .unwrap();

    let mut next = projection.clone();
    next.revision = 1;
    next.snapshot.combatants[0].current_hp = 7;
    store.compare_and_set(0, next).await.unwrap();
    assert_eq!(
        CombatRepository::get(&store, encounter_id)
            .await
            .unwrap()
            .unwrap()
            .revision,
        1
    );

    let mut stale = projection;
    stale.revision = 1;
    assert!(store.compare_and_set(0, stale).await.is_err());
    assert_eq!(
        CombatRepository::get(&store, encounter_id)
            .await
            .unwrap()
            .unwrap()
            .snapshot
            .combatants[0]
            .current_hp,
        7
    );

    let ended = store.end(encounter_id).await.unwrap().unwrap();
    assert!(!ended.snapshot.active);
    assert_eq!(ended.revision, 2);
    assert!(CombatRepository::get(&store, encounter_id)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn combat_create_rolls_back_every_row_when_a_token_insert_fails() {
    let store = store().await;
    let encounter_id = Uuid::new_v4();
    let combatant_id = CombatantId(Uuid::new_v4());
    let mut projection = combat_projection(encounter_id, combatant_id);
    projection
        .snapshot
        .combatants
        .push(Combatant::new(combatant_id, "Duplicate".into(), 5, 5, 10));

    assert!(CombatRepository::create(&store, Uuid::new_v4(), projection)
        .await
        .is_err());
    assert!(CombatRepository::get(&store, encounter_id)
        .await
        .unwrap()
        .is_none());
    let token_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM combat_tokens WHERE encounter_id = ?1")
            .bind(encounter_id.to_string())
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert_eq!(token_count, 0);
}

#[tokio::test]
async fn concurrent_combat_writers_allow_exactly_one_revision_commit() {
    let store = store().await;
    let encounter_id = Uuid::new_v4();
    let combatant_id = CombatantId(Uuid::new_v4());
    let projection = combat_projection(encounter_id, combatant_id);
    CombatRepository::create(&store, Uuid::new_v4(), projection.clone())
        .await
        .unwrap();
    let mut first = projection.clone();
    first.revision = 1;
    first.snapshot.combatants[0].current_hp = 8;
    let mut second = projection;
    second.revision = 1;
    second.snapshot.combatants[0].current_hp = 6;

    let (first_result, second_result) = tokio::join!(
        store.compare_and_set(0, first),
        store.compare_and_set(0, second)
    );
    assert_eq!(
        usize::from(first_result.is_ok()) + usize::from(second_result.is_ok()),
        1
    );
    let stored = CombatRepository::get(&store, encounter_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.revision, 1);
    assert!([6, 8].contains(&stored.snapshot.combatants[0].current_hp));
}

#[tokio::test]
async fn distinct_saves_capture_changed_state_and_retry_remains_idempotent() {
    let store = store().await;
    let (campaign_id, session_id) = campaign(&store, 1).await;
    let first = store
        .create_snapshot(create_snapshot_record(
            campaign_id,
            session_id,
            Revision(1),
            "fresh-1",
        ))
        .await
        .unwrap();
    MessageRepository::append(&store, session_id.0, ChatMessage::user_text("A new turn"))
        .await
        .unwrap();
    let mut next =
        create_snapshot_record(campaign_id, session_id, first.snapshot.revision, "fresh-2");
    next.character = json!({"name": "Mira", "hp": 4, "inventory": ["rope"]});
    let second = store.create_snapshot(next.clone()).await.unwrap();
    let retry = store.create_snapshot(next).await.unwrap();
    assert_ne!(first.snapshot.id, second.snapshot.id);
    assert_eq!(second.snapshot.projection.character["hp"], 4);
    assert_eq!(second.snapshot.projection.messages.len(), 1);
    assert_eq!(retry.snapshot.id, second.snapshot.id);
    assert!(second.snapshot.revision.0 > first.snapshot.revision.0);
}

#[tokio::test]
async fn save_retry_cannot_return_another_campaigns_snapshot() {
    let store = store().await;
    let (campaign_a, session_a) = campaign(&store, 1).await;
    let (campaign_b, session_b) = campaign(&store, 1).await;
    store
        .create_snapshot(create_snapshot_record(
            campaign_a,
            session_a,
            Revision(1),
            "shared-request",
        ))
        .await
        .unwrap();
    let result = store
        .create_snapshot(create_snapshot_record(
            campaign_b,
            session_b,
            Revision(1),
            "shared-request",
        ))
        .await;
    assert!(
        result.is_err(),
        "retry must not expose another campaign's state"
    );
}

#[tokio::test]
async fn campaign_snapshot_is_complete_revision_bound_and_idempotent() {
    let store = store().await;
    let (campaign_id, session_id) = campaign(&store, 4).await;
    MessageRepository::append(
        &store,
        session_id.0,
        ChatMessage::user_text("Open the gate"),
    )
    .await
    .unwrap();
    let record = create_snapshot_record(campaign_id, session_id, Revision(4), "save-request-1");
    let first = store.create_snapshot(record.clone()).await.unwrap();
    let retried = store.create_snapshot(record).await.unwrap();
    assert_eq!(retried.save_id, first.save_id);
    assert_eq!(retried.snapshot.id, first.snapshot.id);
    assert_eq!(first.snapshot.revision, Revision(5));
    assert_eq!(first.snapshot.ruleset_id, "srd-5.2.1");
    assert_eq!(first.snapshot.projection.messages.len(), 1);
    assert_eq!(first.snapshot.projection.character["name"], "Mira");
    let save_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM saves")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(save_count, 1);

    let mut stale = create_snapshot_record(campaign_id, session_id, Revision(3), "stale-save");
    stale.id = Uuid::new_v4();
    assert!(matches!(
        store.create_snapshot(stale).await,
        Err(app_application::ports::repositories::RepositoryError::Conflict { .. })
    ));
}

#[tokio::test]
async fn restore_rolls_back_every_projection_when_active_pointer_update_fails() {
    let store = store().await;
    let (campaign_id, session_id) = campaign(&store, 4).await;
    MessageRepository::append(
        &store,
        session_id.0,
        ChatMessage::user_text("Saved message"),
    )
    .await
    .unwrap();
    let save = create_snapshot_record(campaign_id, session_id, Revision(4), "save-request-2");
    let save_id = save.id;
    let created = store.create_snapshot(save).await.unwrap();
    sqlx::query("DELETE FROM messages WHERE session_id = ?1")
        .bind(session_id.0.to_string())
        .execute(store.pool())
        .await
        .unwrap();
    MessageRepository::append(
        &store,
        session_id.0,
        ChatMessage::user_text("Current message"),
    )
    .await
    .unwrap();
    sqlx::query(
        "CREATE TRIGGER fail_snapshot_activation BEFORE UPDATE OF active_snapshot_id ON sessions \
         BEGIN SELECT RAISE(ABORT, 'forced activation failure'); END",
    )
    .execute(store.pool())
    .await
    .unwrap();

    let result = store
        .restore_snapshot(RestoreSnapshotRecord {
            save_id,
            campaign_id,
            session_id,
            expected_revision: created.snapshot.revision,
            causation_id: "restore-request-1".into(),
        })
        .await;
    assert!(result.is_err());
    let messages = MessageRepository::list(&store, session_id.0).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].message,
        ChatMessage::user_text("Current message")
    );
    let active: Option<String> =
        sqlx::query_scalar("SELECT active_snapshot_id FROM sessions WHERE id = ?1")
            .bind(session_id.0.to_string())
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert!(active.is_none());
    let restore_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM snapshot_restores")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(restore_count, 0);
}
