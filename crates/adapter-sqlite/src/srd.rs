use app_application::models::campaign::SrdReference;
use app_application::ports::repositories::{RepositoryError, SrdRepository};
use async_trait::async_trait;
use sqlx::Row;

use crate::messages::repository_error;
use crate::SqliteStore;

#[async_trait]
impl SrdRepository for SqliteStore {
    async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SrdReference>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT source_key, text_en FROM srd_chunks WHERE text_en LIKE ?1 LIMIT ?2",
        )
        .bind(format!("%{query}%"))
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| repository_error("search_srd"))?;
        rows.into_iter()
            .map(|row| {
                let source_id: String = row
                    .try_get("source_key")
                    .map_err(|_| repository_error("decode_srd"))?;
                let body: String = row
                    .try_get("text_en")
                    .map_err(|_| repository_error("decode_srd"))?;
                Ok(SrdReference {
                    title: source_id.clone(),
                    source_id,
                    body,
                    score: 0.0,
                })
            })
            .collect()
    }
}
