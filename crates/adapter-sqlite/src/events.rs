use crate::campaigns::{
    column, insert_event, parse_epoch, parse_revision, parse_uuid, revision_i64,
};
use crate::messages::repository_error;
use crate::SqliteStore;
use app_application::models::identity::{CampaignId, Revision};
use app_application::models::session::{CampaignEvent, CampaignEventKind};
use app_application::ports::repositories::{EventRepository, RepositoryError};
use async_trait::async_trait;

#[async_trait]
impl EventRepository for SqliteStore {
    #[tracing::instrument(skip_all, fields(
        campaign_id = %campaign_id.0,
        expected_revision = expected.0,
        event_count = events.len(),
        repository = "campaign_events"
    ))]
    async fn append(
        &self,
        campaign_id: CampaignId,
        expected: Revision,
        events: Vec<CampaignEvent>,
    ) -> Result<Revision, RepositoryError> {
        validate_events(campaign_id, expected, &events)?;
        let mut transaction = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| repository_error("begin_campaign_event_append"))?;
        let row = sqlx::query(
            "SELECT id, revision FROM sessions WHERE campaign_id = ?1 \
             ORDER BY CAST(created_at AS INTEGER) DESC, id DESC LIMIT 1",
        )
        .bind(campaign_id.0.to_string())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| repository_error("read_campaign_revision"))?
        .ok_or(RepositoryError::NotFound)?;
        let session_id: String = column(&row, "id")?;
        let actual = parse_revision(column(&row, "revision")?)?;
        if actual != expected {
            return Err(RepositoryError::Conflict { expected, actual });
        }
        if events.is_empty() {
            transaction
                .commit()
                .await
                .map_err(|_| repository_error("commit_empty_campaign_event_append"))?;
            return Ok(actual);
        }
        for event in &events {
            insert_event(&mut transaction, event).await?;
        }
        let next = events.last().map(|event| event.revision).unwrap_or(actual);
        let updated_at = events
            .last()
            .map(|event| event.created_at_epoch_ms)
            .unwrap_or_default()
            .to_string();
        sqlx::query("UPDATE sessions SET revision = ?1, updated_at = ?2 WHERE id = ?3")
            .bind(revision_i64(next)?)
            .bind(&updated_at)
            .bind(session_id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| repository_error("update_campaign_session_revision"))?;
        sqlx::query("UPDATE campaigns SET updated_at = ?1, last_played = ?1 WHERE id = ?2")
            .bind(updated_at)
            .bind(campaign_id.0.to_string())
            .execute(&mut *transaction)
            .await
            .map_err(|_| repository_error("update_campaign_revision"))?;
        transaction
            .commit()
            .await
            .map_err(|_| repository_error("commit_campaign_event_append"))?;
        tracing::info!(
            revision = next.0,
            event_count = events.len(),
            "campaign events committed"
        );
        Ok(next)
    }

    async fn since(
        &self,
        campaign_id: CampaignId,
        after: Revision,
    ) -> Result<Vec<CampaignEvent>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT event_id, revision, kind, payload_json, causation_id, created_at \
             FROM campaign_events WHERE campaign_id = ?1 AND revision > ?2 \
             ORDER BY revision ASC",
        )
        .bind(campaign_id.0.to_string())
        .bind(revision_i64(after)?)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| repository_error("list_campaign_events"))?;
        rows.into_iter()
            .map(|row| {
                Ok(CampaignEvent {
                    event_id: parse_uuid(column(&row, "event_id")?, "decode_campaign_event_id")?,
                    campaign_id,
                    revision: parse_revision(column(&row, "revision")?)?,
                    kind: CampaignEventKind::new(column::<String>(&row, "kind")?),
                    payload: serde_json::from_str(&column::<String>(&row, "payload_json")?)
                        .map_err(|_| repository_error("decode_campaign_event_payload"))?,
                    causation_id: column(&row, "causation_id")?,
                    created_at_epoch_ms: parse_epoch(column(&row, "created_at")?)?,
                })
            })
            .collect()
    }
}

fn validate_events(
    campaign_id: CampaignId,
    expected: Revision,
    events: &[CampaignEvent],
) -> Result<(), RepositoryError> {
    for (index, event) in events.iter().enumerate() {
        let offset = u64::try_from(index)
            .map_err(|_| repository_error("validate_campaign_event_revision"))?
            .saturating_add(1);
        if event.campaign_id != campaign_id
            || event.revision != Revision(expected.0.saturating_add(offset))
            || event.kind.as_str().is_empty()
        {
            return Err(repository_error("validate_campaign_event"));
        }
    }
    Ok(())
}
