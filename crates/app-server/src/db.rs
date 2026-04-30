use serde::{Deserialize, Serialize};
use sqlx::Row;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotRow {
    pub id: Uuid,
    pub session_id: Uuid,
    pub turn_number: i64,
    pub game_state: serde_json::Value,
}

/// Run SQLx migrations from the embedded `migrations/` directory.
pub async fn init_db(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

/// Insert a new snapshot row. Returns the new snapshot UUID.
pub async fn snapshot_insert(
    pool: &SqlitePool,
    session_id: Uuid,
    turn_number: i32,
    game_state: &serde_json::Value,
    player_action: Option<&serde_json::Value>,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::new_v4();
    let now = chrono::Utc::now().to_rfc3339();
    let state_json = serde_json::to_string(game_state).expect("serialize state");
    let action_json = player_action.map(|a| serde_json::to_string(a).expect("serialize action"));

    sqlx::query(
        r#"INSERT INTO snapshots
           (id, session_id, turn_number, created_at, game_state, player_action)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
    )
    .bind(id.to_string())
    .bind(session_id.to_string())
    .bind(turn_number)
    .bind(now)
    .bind(state_json)
    .bind(action_json)
    .execute(pool)
    .await?;

    Ok(id)
}

/// Load the most recent snapshot for a given session, or None if no snapshots exist.
pub async fn snapshot_load_latest(
    pool: &SqlitePool,
    session_id: Uuid,
) -> Result<Option<SnapshotRow>, sqlx::Error> {
    let row = sqlx::query(
        r#"SELECT id, session_id, turn_number, game_state
           FROM snapshots
           WHERE session_id = ?1
           ORDER BY turn_number DESC
           LIMIT 1"#,
    )
    .bind(session_id.to_string())
    .fetch_optional(pool)
    .await?;

    if let Some(r) = row {
        let id_str: String = r.try_get("id")?;
        let session_str: String = r.try_get("session_id")?;
        let turn_number: i64 = r.try_get("turn_number")?;
        let game_state_str: String = r.try_get("game_state")?;
        let id = Uuid::parse_str(&id_str).map_err(|e| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            )))
        })?;
        let session_id_parsed = Uuid::parse_str(&session_str).map_err(|e| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            )))
        })?;
        let state: serde_json::Value = serde_json::from_str(&game_state_str).map_err(|e| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            )))
        })?;
        Ok(Some(SnapshotRow {
            id,
            session_id: session_id_parsed,
            turn_number,
            game_state: state,
        }))
    } else {
        Ok(None)
    }
}
