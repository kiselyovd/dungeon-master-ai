use app_server::db::{
    init_db, journal_insert, journal_list, snapshot_insert, snapshot_load_latest,
};
use sqlx::SqlitePool;
use uuid::Uuid;

async fn in_memory_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("in-memory sqlite");
    init_db(&pool).await.expect("migrate");
    pool
}

#[tokio::test]
async fn migrations_run_without_error() {
    let _pool = in_memory_pool().await;
    // If we reach here, migrations ran.
}

#[tokio::test]
async fn snapshot_round_trip() {
    let pool = in_memory_pool().await;
    let session_id = Uuid::new_v4();
    let game_state = serde_json::json!({
        "schema_version": 1,
        "state": { "round": 1, "test": true }
    });

    let snap_id = snapshot_insert(&pool, session_id, 1, &game_state, None)
        .await
        .expect("insert snapshot");

    let loaded = snapshot_load_latest(&pool, session_id)
        .await
        .expect("load snapshot")
        .expect("should exist");

    assert_eq!(loaded.id, snap_id);
    assert_eq!(loaded.turn_number, 1);
    assert_eq!(loaded.game_state["state"]["test"], true);
}

#[tokio::test]
async fn snapshot_load_latest_returns_none_when_empty() {
    let pool = in_memory_pool().await;
    let session_id = Uuid::new_v4();
    let result = snapshot_load_latest(&pool, session_id)
        .await
        .expect("query ok");
    assert!(result.is_none());
}

// ---- Journal ----

#[tokio::test]
async fn journal_insert_and_list() {
    let pool = in_memory_pool().await;
    let campaign_id = Uuid::new_v4();
    let entry_id = journal_insert(
        &pool,
        campaign_id,
        "<p>The party entered the dungeon.</p>",
        Some("Chapter 1"),
    )
    .await
    .unwrap();
    let entries = journal_list(&pool, campaign_id).await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, entry_id);
    assert_eq!(entries[0].chapter.as_deref(), Some("Chapter 1"));
}

#[tokio::test]
async fn journal_list_empty_for_new_campaign() {
    let pool = in_memory_pool().await;
    let entries = journal_list(&pool, Uuid::new_v4()).await.unwrap();
    assert!(entries.is_empty());
}

#[tokio::test]
async fn journal_entries_ordered_by_creation() {
    let pool = in_memory_pool().await;
    let campaign_id = Uuid::new_v4();
    journal_insert(&pool, campaign_id, "<p>First.</p>", None)
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    journal_insert(&pool, campaign_id, "<p>Second.</p>", None)
        .await
        .unwrap();
    let entries = journal_list(&pool, campaign_id).await.unwrap();
    assert_eq!(entries.len(), 2);
    assert!(entries[0].entry_html.contains("First"));
    assert!(entries[1].entry_html.contains("Second"));
}
