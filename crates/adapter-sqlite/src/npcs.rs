use app_application::models::campaign::NpcRecord;
use app_application::ports::repositories::{NpcRepository, RepositoryError};
use async_trait::async_trait;
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

use crate::messages::repository_error;
use crate::SqliteStore;

#[async_trait]
impl NpcRepository for SqliteStore {
    async fn list(&self, campaign_id: Uuid) -> Result<Vec<NpcRecord>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT id, name, facts, updated_at FROM npc_memory \
             WHERE campaign_id = ?1 ORDER BY name",
        )
        .bind(campaign_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|_| repository_error("list_npcs"))?;
        rows.into_iter()
            .map(|row| {
                let id: String = row
                    .try_get("id")
                    .map_err(|_| repository_error("decode_npc"))?;
                let facts: String = row
                    .try_get("facts")
                    .map_err(|_| repository_error("decode_npc"))?;
                let updated_at: String = row
                    .try_get("updated_at")
                    .map_err(|_| repository_error("decode_npc"))?;
                let description = serde_json::from_str::<Vec<serde_json::Value>>(&facts)
                    .ok()
                    .and_then(|facts| {
                        facts
                            .last()
                            .and_then(|fact| fact.get("text"))
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_string)
                    })
                    .unwrap_or_default();
                Ok(NpcRecord {
                    id: Uuid::parse_str(&id).map_err(|_| repository_error("decode_npc_id"))?,
                    campaign_id,
                    name: row
                        .try_get("name")
                        .map_err(|_| repository_error("decode_npc"))?,
                    description,
                    updated_at_epoch_ms: chrono::DateTime::parse_from_rfc3339(&updated_at)
                        .map(|value| value.timestamp_millis())
                        .unwrap_or_default(),
                })
            })
            .collect()
    }

    async fn put(&self, npc: NpcRecord) -> Result<(), RepositoryError> {
        let updated_at = chrono::DateTime::from_timestamp_millis(npc.updated_at_epoch_ms)
            .unwrap_or_else(chrono::Utc::now)
            .to_rfc3339();
        let facts = serde_json::to_string(&vec![json!({
            "text": npc.description,
            "created_at": updated_at,
        })])
        .map_err(|_| repository_error("encode_npc"))?;
        sqlx::query(
            "INSERT OR REPLACE INTO npc_memory \
             (id, campaign_id, name, role, disposition, trust, facts, updated_at) \
             VALUES (?1, ?2, ?3, '', 'unknown', 0, ?4, ?5)",
        )
        .bind(npc.id.to_string())
        .bind(npc.campaign_id.to_string())
        .bind(npc.name)
        .bind(facts)
        .bind(updated_at)
        .execute(&self.pool)
        .await
        .map_err(|_| repository_error("put_npc"))?;
        Ok(())
    }
}
