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

// ---- Journal ----

pub async fn journal_insert(
    pool: &SqlitePool,
    campaign_id: Uuid,
    entry_html: &str,
    chapter: Option<&str>,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::new_v4();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        r#"INSERT INTO journal_entries (id, campaign_id, chapter, entry_html, created_at)
           VALUES (?1, ?2, ?3, ?4, ?5)"#,
    )
    .bind(id.to_string())
    .bind(campaign_id.to_string())
    .bind(chapter)
    .bind(entry_html)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub chapter: Option<String>,
    pub entry_html: String,
    pub created_at: String,
}

pub async fn journal_list(
    pool: &SqlitePool,
    campaign_id: Uuid,
) -> Result<Vec<JournalEntry>, sqlx::Error> {
    let rows = sqlx::query(
        r#"SELECT id, campaign_id, chapter, entry_html, created_at
           FROM journal_entries
           WHERE campaign_id = ?1
           ORDER BY created_at ASC"#,
    )
    .bind(campaign_id.to_string())
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|r| {
            Ok(JournalEntry {
                id: Uuid::parse_str(r.try_get::<String, _>("id")?.as_str()).map_err(|e| {
                    sqlx::Error::Decode(Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e.to_string(),
                    )))
                })?,
                campaign_id: Uuid::parse_str(r.try_get::<String, _>("campaign_id")?.as_str())
                    .map_err(|e| {
                        sqlx::Error::Decode(Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            e.to_string(),
                        )))
                    })?,
                chapter: r.try_get("chapter")?,
                entry_html: r.try_get("entry_html")?,
                created_at: r.try_get("created_at")?,
            })
        })
        .collect()
}

// ---- NPC Memory ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcFact {
    pub text: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcMemoryRow {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub name: String,
    pub role: String,
    pub disposition: String,
    pub trust: i32,
    pub facts: Vec<NpcFact>,
    pub updated_at: String,
}

pub async fn npc_upsert_fact(
    pool: &SqlitePool,
    campaign_id: Uuid,
    name: &str,
    fact: &str,
    disposition: &str,
    role: &str,
) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    // Fetch existing row if any.
    let existing =
        sqlx::query("SELECT id, facts FROM npc_memory WHERE campaign_id = ?1 AND name = ?2")
            .bind(campaign_id.to_string())
            .bind(name)
            .fetch_optional(pool)
            .await?;

    if let Some(row) = existing {
        let id: String = row.try_get("id")?;
        let facts_json: String = row.try_get("facts")?;
        let mut facts: Vec<NpcFact> = serde_json::from_str(&facts_json).unwrap_or_default();
        facts.push(NpcFact {
            text: fact.to_string(),
            created_at: now.clone(),
        });
        let new_facts = serde_json::to_string(&facts).unwrap_or_default();
        sqlx::query(
            "UPDATE npc_memory SET facts = ?1, disposition = ?2, role = ?3, updated_at = ?4 WHERE id = ?5"
        )
        .bind(new_facts)
        .bind(disposition)
        .bind(role)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    } else {
        let id = Uuid::new_v4();
        let initial_fact = NpcFact {
            text: fact.to_string(),
            created_at: now.clone(),
        };
        let facts_json = serde_json::to_string(&[initial_fact]).unwrap_or_default();
        sqlx::query(
            r#"INSERT INTO npc_memory (id, campaign_id, name, role, disposition, trust, facts, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7)"#,
        )
        .bind(id.to_string())
        .bind(campaign_id.to_string())
        .bind(name)
        .bind(role)
        .bind(disposition)
        .bind(facts_json)
        .bind(now)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn npc_get(
    pool: &SqlitePool,
    campaign_id: Uuid,
    name: &str,
) -> Result<Option<NpcMemoryRow>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id, campaign_id, name, role, disposition, trust, facts, updated_at FROM npc_memory WHERE campaign_id = ?1 AND name = ?2"
    )
    .bind(campaign_id.to_string())
    .bind(name)
    .fetch_optional(pool)
    .await?;

    if let Some(r) = row {
        Ok(Some(parse_npc_row(&r)?))
    } else {
        Ok(None)
    }
}

pub async fn npc_get_all(
    pool: &SqlitePool,
    campaign_id: Uuid,
) -> Result<Vec<NpcMemoryRow>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, campaign_id, name, role, disposition, trust, facts, updated_at FROM npc_memory WHERE campaign_id = ?1 ORDER BY name"
    )
    .bind(campaign_id.to_string())
    .fetch_all(pool)
    .await?;

    rows.iter().map(parse_npc_row).collect()
}

fn parse_npc_row(r: &sqlx::sqlite::SqliteRow) -> Result<NpcMemoryRow, sqlx::Error> {
    let facts_json: String = r.try_get("facts")?;
    let facts: Vec<NpcFact> = serde_json::from_str(&facts_json).unwrap_or_default();
    Ok(NpcMemoryRow {
        id: Uuid::parse_str(r.try_get::<String, _>("id")?.as_str()).map_err(|e| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            )))
        })?,
        campaign_id: Uuid::parse_str(r.try_get::<String, _>("campaign_id")?.as_str()).map_err(
            |e| {
                sqlx::Error::Decode(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    e.to_string(),
                )))
            },
        )?,
        name: r.try_get("name")?,
        role: r.try_get("role")?,
        disposition: r.try_get("disposition")?,
        trust: r.try_get("trust")?,
        facts,
        updated_at: r.try_get("updated_at")?,
    })
}

// ---- SRD chunks (embedding cache) ----

pub async fn srd_chunks_upsert(
    pool: &SqlitePool,
    source_key: &str,
    text_en: &str,
    embedding_bytes: &[u8],
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT OR REPLACE INTO srd_chunks (id, source_key, text_en, embedding)
           VALUES (?1, ?2, ?3, ?4)"#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(source_key)
    .bind(text_en)
    .bind(embedding_bytes)
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Debug)]
pub struct SrdChunkRow {
    pub source_key: String,
    pub text_en: String,
    pub embedding: Vec<u8>,
}

pub async fn srd_chunks_load_all(pool: &SqlitePool) -> Result<Vec<SrdChunkRow>, sqlx::Error> {
    let rows =
        sqlx::query("SELECT source_key, text_en, embedding FROM srd_chunks WHERE embedding IS NOT NULL")
            .fetch_all(pool)
            .await?;

    rows.into_iter()
        .map(|r| {
            Ok(SrdChunkRow {
                source_key: r.try_get("source_key")?,
                text_en: r.try_get("text_en")?,
                embedding: r.try_get("embedding")?,
            })
        })
        .collect()
}
