use app_application::models::campaign::SceneRecord;
use app_application::ports::repositories::{RepositoryError, SceneRepository};
use async_trait::async_trait;
use sqlx::Row;
use uuid::Uuid;

use crate::messages::repository_error;
use crate::SqliteStore;

#[async_trait]
impl SceneRepository for SqliteStore {
    async fn current(&self, campaign_id: Uuid) -> Result<Option<SceneRecord>, RepositoryError> {
        let row = sqlx::query(
            "SELECT id, title, subtitle FROM scenes WHERE campaign_id = ?1 \
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(campaign_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| repository_error("current_scene"))?;
        row.map(|row| {
            let id: String = row
                .try_get("id")
                .map_err(|_| repository_error("decode_scene"))?;
            Ok(SceneRecord {
                id: Uuid::parse_str(&id).map_err(|_| repository_error("decode_scene_id"))?,
                campaign_id,
                name: row
                    .try_get("title")
                    .map_err(|_| repository_error("decode_scene"))?,
                description: row
                    .try_get::<Option<String>, _>("subtitle")
                    .map_err(|_| repository_error("decode_scene"))?
                    .unwrap_or_default(),
            })
        })
        .transpose()
    }

    async fn set_current(&self, scene: SceneRecord) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT OR REPLACE INTO scenes \
             (id, campaign_id, title, subtitle, mode, image_prompt, created_at) \
             VALUES (?1, ?2, ?3, ?4, 'exploration', NULL, ?5)",
        )
        .bind(scene.id.to_string())
        .bind(scene.campaign_id.to_string())
        .bind(scene.name)
        .bind(scene.description)
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|_| repository_error("set_scene"))?;
        Ok(())
    }
}
