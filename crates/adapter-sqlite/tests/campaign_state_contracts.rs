use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_sqlite::{pool, SqliteStore};
use app_application::campaigns::{CreateCampaign, CreateCampaignCommand};
use app_application::models::campaign_state::CommitCampaignCharacter;
use app_application::models::session::CampaignSeed;
use app_application::models::snapshot::{CreateSnapshotRecord, RestoreSnapshotRecord};
use app_application::ports::repositories::{
    CampaignStateRepository, MessageRepository, SnapshotRepository,
};
use serde_json::json;
use sqlx::SqlitePool;
use uuid::Uuid;

async fn store() -> Arc<SqliteStore> {
    let db = SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&db).await.unwrap();
    Arc::new(SqliteStore::new(db))
}

async fn campaign(
    store: Arc<SqliteStore>,
    title: &str,
) -> app_application::models::session::CampaignDetail {
    CreateCampaign::new(store)
        .execute(CreateCampaignCommand {
            title: title.into(),
            premise: "A road".into(),
            seed: CampaignSeed::Collaborative {
                answers: BTreeMap::from([("genre".into(), vec!["fantasy".into()])]),
            },
        })
        .await
        .unwrap()
}

fn hero(name: &str) -> serde_json::Value {
    json!({"name": name, "heroClass": "fighter", "hp": 12, "hpMax": 12, "ac": 14,
        "level": 1, "abilities": {"str": 15, "dex": 14, "con": 13, "int": 12, "wis": 10, "cha": 8},
        "inventory": [], "backstory": "An old ferry debt", "spells": {"cantrips": [], "level1": []}})
}

#[tokio::test]
async fn current_campaign_characters_are_durable_isolated_and_retry_safe() {
    let store = store().await;
    let a = campaign(store.clone(), "A").await;
    let b = campaign(store.clone(), "B").await;
    let empty = store.load_current(b.id).await.unwrap();
    assert!(empty.character_id.is_none());
    assert_eq!(empty.projection.character, json!({}));
    let command = CommitCampaignCharacter {
        campaign_id: a.id,
        session_id: a.last_session_id,
        character_id: Uuid::new_v4(),
        expected_revision: a.revision,
        causation_id: "hero-commit".into(),
        character: hero("Mira"),
    };
    let committed = store.commit_character(command.clone()).await.unwrap();
    let retry = store.commit_character(command).await.unwrap();
    assert_eq!(retry, committed);
    assert_eq!(
        store.load_current(a.id).await.unwrap().projection.character["name"],
        "Mira"
    );
    assert_eq!(
        store.load_current(b.id).await.unwrap().projection.character,
        json!({})
    );
    assert_eq!(
        committed.projection.character["backstory"],
        "An old ferry debt"
    );
}

#[tokio::test]
async fn restore_updates_current_character_instead_of_selecting_newest_save() {
    let store = store().await;
    let campaign = campaign(store.clone(), "A").await;
    let committed = store
        .commit_character(CommitCampaignCharacter {
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            character_id: Uuid::new_v4(),
            expected_revision: campaign.revision,
            causation_id: "hero".into(),
            character: hero("Mira"),
        })
        .await
        .unwrap();
    let record = CreateSnapshotRecord {
        id: Uuid::new_v4(),
        campaign_id: campaign.id,
        session_id: campaign.last_session_id,
        expected_revision: committed.campaign.revision,
        schema_version: 1,
        ruleset_id: "srd-5.2.1".into(),
        title: "Before".into(),
        kind: "manual".into(),
        summary: String::new(),
        tag: "exploration".into(),
        causation_id: "save-before".into(),
        character: hero("Mira"),
    };
    let first = store.create_snapshot(record.clone()).await.unwrap();
    let changed = store
        .commit_character(CommitCampaignCharacter {
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            character_id: committed.character_id.unwrap(),
            expected_revision: first.snapshot.revision,
            causation_id: "change-hero".into(),
            character: hero("Changed"),
        })
        .await
        .unwrap();
    let mut second_record = record;
    second_record.id = Uuid::new_v4();
    second_record.expected_revision = changed.campaign.revision;
    second_record.causation_id = "save-after".into();
    second_record.character = hero("Changed");
    let second = store.create_snapshot(second_record).await.unwrap();
    assert_eq!(
        store
            .load_current(campaign.id)
            .await
            .unwrap()
            .projection
            .character["name"],
        "Changed"
    );
    store
        .restore_snapshot(RestoreSnapshotRecord {
            save_id: first.save_id,
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            expected_revision: second.snapshot.revision,
            causation_id: "restore-before".into(),
        })
        .await
        .unwrap();
    assert_eq!(
        store
            .load_current(campaign.id)
            .await
            .unwrap()
            .projection
            .character["name"],
        "Mira"
    );
    assert!(
        MessageRepository::list(store.as_ref(), campaign.last_session_id.0)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn failed_character_commit_rolls_back_hero_revision_and_receipt() {
    let store = store().await;
    let campaign = campaign(store.clone(), "A").await;
    sqlx::query("CREATE TRIGGER fail_character BEFORE INSERT ON campaign_characters BEGIN SELECT RAISE(ABORT, 'failure'); END")
        .execute(store.pool()).await.unwrap();
    let result = store
        .commit_character(CommitCampaignCharacter {
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            character_id: Uuid::new_v4(),
            expected_revision: campaign.revision,
            causation_id: "failed".into(),
            character: hero("Mira"),
        })
        .await;
    assert!(result.is_err());
    let state = store.load_current(campaign.id).await.unwrap();
    assert_eq!(state.campaign.revision, campaign.revision);
    assert!(state.character_id.is_none());
}

#[tokio::test]
async fn migration_backfills_the_explicitly_restored_hero_without_rewriting_history() {
    let db = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::raw_sql(include_str!("../migrations/0001_initial.sql"))
        .execute(&db)
        .await
        .unwrap();
    let store = Arc::new(SqliteStore::new(db));
    let campaign = campaign(store.clone(), "Old campaign").await;
    let first_id = Uuid::new_v4();
    let latest_id = Uuid::new_v4();
    for (id, revision, name) in [
        (first_id, 1_i64, "Restored Mira"),
        (latest_id, 2, "Later Hero"),
    ] {
        sqlx::query("INSERT INTO campaign_snapshots (id, campaign_id, session_id, revision, schema_version, ruleset_id, projection_json, created_at) VALUES (?1, ?2, ?3, ?4, 1, 'srd-5.2.1', ?5, '2026-10-06')")
            .bind(id.to_string()).bind(campaign.id.0.to_string()).bind(campaign.last_session_id.0.to_string()).bind(revision)
            .bind(json!({"character": hero(name), "messages": [], "combat": null, "scene": null, "journal": [], "npcs": []}).to_string())
            .execute(store.pool()).await.unwrap();
    }
    sqlx::query("UPDATE sessions SET active_snapshot_id = ?1 WHERE id = ?2")
        .bind(first_id.to_string())
        .bind(campaign.last_session_id.0.to_string())
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../migrations/0002_campaign_characters.sql"))
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../migrations/0003_campaign_adventures.sql"))
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../migrations/0004_agent_operations.sql"))
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../migrations/0005_agent_checkpoints.sql"))
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../migrations/0006_combat_action_receipts.sql"
    ))
    .execute(store.pool())
    .await
    .unwrap();
    sqlx::raw_sql(include_str!("../migrations/0007_adventuring_resources.sql"))
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../migrations/0008_party_roster.sql"))
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../migrations/0009_room_capabilities.sql"))
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../migrations/0010_room_character_proposals.sql"
    ))
    .execute(store.pool())
    .await
    .unwrap();
    sqlx::raw_sql(include_str!("../migrations/0011_room_updates.sql"))
        .execute(store.pool())
        .await
        .unwrap();
    assert_eq!(
        store
            .load_current(campaign.id)
            .await
            .unwrap()
            .projection
            .character["name"],
        "Restored Mira"
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM campaign_snapshots")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn server_side_checkpoint_uses_the_durable_hero_when_no_sheet_is_supplied() {
    let store = store().await;
    let campaign = campaign(store.clone(), "A").await;
    let state = store
        .commit_character(CommitCampaignCharacter {
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            character_id: Uuid::new_v4(),
            expected_revision: campaign.revision,
            causation_id: "hero".into(),
            character: hero("Mira"),
        })
        .await
        .unwrap();
    let saved = store
        .create_snapshot(CreateSnapshotRecord {
            id: Uuid::new_v4(),
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            expected_revision: state.campaign.revision,
            schema_version: 1,
            ruleset_id: "srd-5.2.1".into(),
            title: "Server checkpoint".into(),
            kind: "auto".into(),
            summary: String::new(),
            tag: "exploration".into(),
            causation_id: "checkpoint".into(),
            character: json!({}),
        })
        .await
        .unwrap();
    assert_eq!(saved.snapshot.projection.character["name"], "Mira");
}

#[tokio::test]
async fn migration_skips_incomplete_sheets_and_selects_a_complete_older_candidate() {
    let db = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::raw_sql(include_str!("../migrations/0001_initial.sql"))
        .execute(&db)
        .await
        .unwrap();
    let store = Arc::new(SqliteStore::new(db));
    let a = campaign(store.clone(), "A").await;
    let b = campaign(store.clone(), "B").await;
    for (campaign, revision, character) in [
        (&a, 1_i64, hero("Complete Mira")),
        (
            &a,
            2,
            json!({"name": "Incomplete later", "heroClass": "fighter"}),
        ),
        (
            &b,
            1,
            json!({"name": "Only incomplete", "heroClass": "fighter"}),
        ),
    ] {
        sqlx::query("INSERT INTO campaign_snapshots (id, campaign_id, session_id, revision, schema_version, ruleset_id, projection_json, created_at) VALUES (?1, ?2, ?3, ?4, 1, 'srd-5.2.1', ?5, '2026-10-06')")
            .bind(Uuid::new_v4().to_string()).bind(campaign.id.0.to_string()).bind(campaign.last_session_id.0.to_string()).bind(revision)
            .bind(json!({"character": character, "messages": [], "combat": null, "scene": null, "journal": [], "npcs": []}).to_string())
            .execute(store.pool()).await.unwrap();
    }
    sqlx::raw_sql(include_str!("../migrations/0002_campaign_characters.sql"))
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../migrations/0003_campaign_adventures.sql"))
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../migrations/0004_agent_operations.sql"))
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../migrations/0005_agent_checkpoints.sql"))
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../migrations/0006_combat_action_receipts.sql"
    ))
    .execute(store.pool())
    .await
    .unwrap();
    sqlx::raw_sql(include_str!("../migrations/0007_adventuring_resources.sql"))
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../migrations/0008_party_roster.sql"))
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../migrations/0009_room_capabilities.sql"))
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../migrations/0010_room_character_proposals.sql"
    ))
    .execute(store.pool())
    .await
    .unwrap();
    sqlx::raw_sql(include_str!("../migrations/0011_room_updates.sql"))
        .execute(store.pool())
        .await
        .unwrap();
    assert_eq!(
        store.load_current(a.id).await.unwrap().projection.character["name"],
        "Complete Mira"
    );
    assert!(store
        .load_current(b.id)
        .await
        .unwrap()
        .character_id
        .is_none());
    let recovered = store
        .commit_character(CommitCampaignCharacter {
            campaign_id: b.id,
            session_id: b.last_session_id,
            character_id: Uuid::new_v4(),
            expected_revision: b.revision,
            causation_id: "recover".into(),
            character: hero("Recovered"),
        })
        .await
        .unwrap();
    assert_eq!(recovered.projection.character["name"], "Recovered");
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM campaign_snapshots")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(count, 3);
}
