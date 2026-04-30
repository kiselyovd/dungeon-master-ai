use app_server::db::{init_db, snapshot_insert, snapshot_load_latest};
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
