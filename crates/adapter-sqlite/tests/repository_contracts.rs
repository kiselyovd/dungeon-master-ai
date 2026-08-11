use adapter_sqlite::{pool, SqliteStore};
use app_application::models::campaign::{CampaignSave, JournalEntry, NpcRecord, SceneRecord};
use app_application::models::chat::{ChatMessage, ToolCall, ToolResult};
use app_application::models::combat::{
    CombatProjection, CombatSnapshot, COMBAT_PROJECTION_VERSION,
};
use app_application::ports::repositories::{
    CombatRepository, JournalRepository, MessageRepository, NpcRepository, SaveRepository,
    SceneRepository, SrdRepository,
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

fn combat_projection(encounter_id: Uuid, combatant_id: CombatantId) -> CombatProjection {
    CombatProjection {
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
    assert_eq!(migration_count, 7);
    let columns = sqlx::query("PRAGMA table_info(combat_encounters)")
        .fetch_all(store.pool())
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get::<String, _>("name"))
        .collect::<Vec<_>>();
    assert!(columns.contains(&"revision".to_string()));
    assert!(columns.contains(&"projection".to_string()));
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
    store
        .create(Uuid::new_v4(), projection.clone())
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

    assert!(store.create(Uuid::new_v4(), projection).await.is_err());
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
    store
        .create(Uuid::new_v4(), projection.clone())
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
