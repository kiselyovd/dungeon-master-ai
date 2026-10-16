use crate::campaigns::{column, parse_epoch, parse_revision, parse_uuid};
use crate::messages::repository_error;
use crate::SqliteStore;
use app_application::models::identity::{CampaignId, SessionId};
use app_application::models::session::CampaignSession;
use app_application::ports::repositories::{RepositoryError, SessionRepository};
use async_trait::async_trait;

#[async_trait]
impl SessionRepository for SqliteStore {
    async fn get(&self, session_id: SessionId) -> Result<Option<CampaignSession>, RepositoryError> {
        let row = sqlx::query(
            "SELECT id, campaign_id, revision, created_at, updated_at \
             FROM sessions WHERE id = ?1",
        )
        .bind(session_id.0.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| repository_error("get_campaign_session"))?;
        row.map(map_session).transpose()
    }

    async fn latest_for_campaign(
        &self,
        campaign_id: CampaignId,
    ) -> Result<Option<CampaignSession>, RepositoryError> {
        let row = sqlx::query(
            "SELECT id, campaign_id, revision, created_at, updated_at \
             FROM sessions WHERE campaign_id = ?1 \
             ORDER BY CAST(created_at AS INTEGER) DESC, id DESC LIMIT 1",
        )
        .bind(campaign_id.0.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| repository_error("latest_campaign_session"))?;
        row.map(map_session).transpose()
    }
}

fn map_session(row: sqlx::sqlite::SqliteRow) -> Result<CampaignSession, RepositoryError> {
    Ok(CampaignSession {
        id: SessionId(parse_uuid(column(&row, "id")?, "decode_session_id")?),
        campaign_id: CampaignId(parse_uuid(
            column(&row, "campaign_id")?,
            "decode_session_campaign_id",
        )?),
        revision: parse_revision(column(&row, "revision")?)?,
        created_at_epoch_ms: parse_epoch(column(&row, "created_at")?)?,
        updated_at_epoch_ms: parse_epoch(column(&row, "updated_at")?)?,
    })
}
