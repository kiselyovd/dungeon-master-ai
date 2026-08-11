use app_application::models::campaign::CampaignSave;
use app_application::ports::repositories::{RepositoryError, SaveRepository};
use async_trait::async_trait;
use sqlx::Row;
use uuid::Uuid;

use crate::messages::repository_error;
use crate::SqliteStore;

#[async_trait]
impl SaveRepository for SqliteStore {
    async fn put(&self, save: CampaignSave) -> Result<(), RepositoryError> {
        let payload = serde_json::to_string(&save).map_err(|_| repository_error("encode_save"))?;
        sqlx::query(
            "INSERT OR REPLACE INTO snapshots \
             (id, session_id, turn_number, created_at, game_state, kind, title, summary, tag) \
             VALUES (?1, ?2, 0, ?3, ?4, 'manual', ?5, '', '')",
        )
        .bind(save.id.to_string())
        .bind(save.session_id.to_string())
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(payload)
        .bind(&save.name)
        .execute(&self.pool)
        .await
        .map_err(|_| repository_error("put_save"))?;
        Ok(())
    }

    async fn get(&self, save_id: Uuid) -> Result<Option<CampaignSave>, RepositoryError> {
        let row = sqlx::query("SELECT game_state FROM snapshots WHERE id = ?1")
            .bind(save_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| repository_error("get_save"))?;
        row.map(|row| {
            let payload: String = row
                .try_get("game_state")
                .map_err(|_| repository_error("decode_save"))?;
            serde_json::from_str(&payload).map_err(|_| repository_error("decode_save"))
        })
        .transpose()
    }
}
