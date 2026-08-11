use app_application::models::campaign::JournalEntry;
use app_application::ports::repositories::{JournalRepository, RepositoryError};
use async_trait::async_trait;
use sqlx::Row;
use uuid::Uuid;

use crate::messages::repository_error;
use crate::SqliteStore;

#[async_trait]
impl JournalRepository for SqliteStore {
    async fn list(&self, campaign_id: Uuid) -> Result<Vec<JournalEntry>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT id, chapter, entry_html, created_at FROM journal_entries \
             WHERE campaign_id = ?1 ORDER BY created_at ASC",
        )
        .bind(campaign_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|_| repository_error("list_journal"))?;
        rows.into_iter()
            .map(|row| {
                let id: String = row
                    .try_get("id")
                    .map_err(|_| repository_error("decode_journal"))?;
                let created_at: String = row
                    .try_get("created_at")
                    .map_err(|_| repository_error("decode_journal"))?;
                Ok(JournalEntry {
                    id: Uuid::parse_str(&id).map_err(|_| repository_error("decode_journal_id"))?,
                    campaign_id,
                    title: row
                        .try_get::<Option<String>, _>("chapter")
                        .map_err(|_| repository_error("decode_journal"))?
                        .unwrap_or_default(),
                    body: row
                        .try_get("entry_html")
                        .map_err(|_| repository_error("decode_journal"))?,
                    updated_at_epoch_ms: chrono::DateTime::parse_from_rfc3339(&created_at)
                        .map(|value| value.timestamp_millis())
                        .unwrap_or_default(),
                })
            })
            .collect()
    }

    async fn put(&self, entry: JournalEntry) -> Result<(), RepositoryError> {
        let created_at = chrono::DateTime::from_timestamp_millis(entry.updated_at_epoch_ms)
            .unwrap_or_else(chrono::Utc::now)
            .to_rfc3339();
        sqlx::query(
            "INSERT OR REPLACE INTO journal_entries \
             (id, campaign_id, chapter, entry_html, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(entry.id.to_string())
        .bind(entry.campaign_id.to_string())
        .bind(entry.title)
        .bind(entry.body)
        .bind(created_at)
        .execute(&self.pool)
        .await
        .map_err(|_| repository_error("put_journal"))?;
        Ok(())
    }
}
