use app_application::models::identity::{CampaignId, Revision, SessionId};
use app_application::models::session::{
    CampaignDetail, CampaignSeed, CampaignSummary, InitialCampaign,
};
use app_application::ports::repositories::{CampaignRepository, RepositoryError};
use async_trait::async_trait;
use sqlx::{Decode, Row, Sqlite, Type};

use crate::messages::repository_error;
use crate::SqliteStore;

#[async_trait]
impl CampaignRepository for SqliteStore {
    #[tracing::instrument(skip_all, fields(
        campaign_id = %campaign.detail.id.0,
        session_id = %campaign.session.id.0,
        revision = campaign.detail.revision.0,
        repository = "campaigns"
    ))]
    async fn create(&self, campaign: InitialCampaign) -> Result<CampaignDetail, RepositoryError> {
        validate_initial(&campaign)?;
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| repository_error("begin_campaign_create"))?;
        sqlx::query(
            "INSERT INTO campaigns \
             (id, schema_version, ruleset_id, title, premise, created_at, updated_at, \
              name, language, last_played) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?4, 'ru', ?7)",
        )
        .bind(campaign.detail.id.0.to_string())
        .bind(i64::from(campaign.detail.schema_version))
        .bind(&campaign.detail.ruleset_id)
        .bind(&campaign.detail.title)
        .bind(&campaign.detail.premise)
        .bind(campaign.detail.created_at_epoch_ms.to_string())
        .bind(campaign.detail.updated_at_epoch_ms.to_string())
        .execute(&mut *transaction)
        .await
        .map_err(|_| repository_error("insert_campaign"))?;
        sqlx::query(
            "INSERT INTO sessions \
             (id, campaign_id, revision, created_at, updated_at, number, started_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?4)",
        )
        .bind(campaign.session.id.0.to_string())
        .bind(campaign.session.campaign_id.0.to_string())
        .bind(revision_i64(campaign.session.revision)?)
        .bind(campaign.session.created_at_epoch_ms.to_string())
        .bind(campaign.session.updated_at_epoch_ms.to_string())
        .execute(&mut *transaction)
        .await
        .map_err(|_| repository_error("insert_campaign_session"))?;
        insert_event(&mut transaction, &campaign.event).await?;
        transaction
            .commit()
            .await
            .map_err(|_| repository_error("commit_campaign_create"))?;
        Ok(campaign.detail)
    }

    #[tracing::instrument(skip_all, fields(repository = "campaigns"))]
    async fn list(&self) -> Result<Vec<CampaignSummary>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT c.id, c.schema_version, c.ruleset_id, c.title, c.premise, \
                    c.updated_at, s.id AS session_id, s.revision \
             FROM campaigns c \
             JOIN sessions s ON s.id = ( \
                 SELECT latest.id FROM sessions latest \
                 WHERE latest.campaign_id = c.id \
                 ORDER BY CAST(latest.created_at AS INTEGER) DESC, latest.id DESC LIMIT 1 \
             ) \
             ORDER BY CAST(c.updated_at AS INTEGER) DESC, c.id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| repository_error("list_campaigns"))?;
        rows.into_iter().map(|row| map_summary(&row)).collect()
    }

    #[tracing::instrument(skip_all, fields(campaign_id = %campaign_id.0, repository = "campaigns"))]
    async fn get(
        &self,
        campaign_id: CampaignId,
    ) -> Result<Option<CampaignDetail>, RepositoryError> {
        let row = sqlx::query(
            "SELECT c.id, c.schema_version, c.ruleset_id, c.title, c.premise, \
                    c.created_at, c.updated_at, s.id AS session_id, s.revision, \
                    e.payload_json \
             FROM campaigns c \
             JOIN sessions s ON s.id = ( \
                 SELECT latest.id FROM sessions latest \
                 WHERE latest.campaign_id = c.id \
                 ORDER BY CAST(latest.created_at AS INTEGER) DESC, latest.id DESC LIMIT 1 \
             ) \
             JOIN campaign_events e ON e.campaign_id = c.id AND e.revision = 1 \
             WHERE c.id = ?1",
        )
        .bind(campaign_id.0.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| repository_error("get_campaign"))?;
        row.map(map_detail).transpose()
    }
}

fn validate_initial(campaign: &InitialCampaign) -> Result<(), RepositoryError> {
    let detail = &campaign.detail;
    let valid = detail.id == campaign.session.campaign_id
        && detail.id == campaign.event.campaign_id
        && detail.last_session_id == campaign.session.id
        && detail.revision == Revision(1)
        && campaign.session.revision == detail.revision
        && campaign.event.revision == detail.revision
        && campaign.event.kind.as_str() == "campaign_created";
    if valid {
        Ok(())
    } else {
        Err(repository_error("validate_initial_campaign"))
    }
}

pub(crate) async fn insert_event(
    transaction: &mut sqlx::Transaction<'_, Sqlite>,
    event: &app_application::models::session::CampaignEvent,
) -> Result<(), RepositoryError> {
    let payload = serde_json::to_string(&event.payload)
        .map_err(|_| repository_error("encode_campaign_event"))?;
    sqlx::query(
        "INSERT INTO campaign_events \
         (campaign_id, revision, event_id, kind, payload_json, causation_id, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(event.campaign_id.0.to_string())
    .bind(revision_i64(event.revision)?)
    .bind(event.event_id.to_string())
    .bind(event.kind.as_str())
    .bind(payload)
    .bind(&event.causation_id)
    .bind(event.created_at_epoch_ms.to_string())
    .execute(&mut **transaction)
    .await
    .map_err(|_| repository_error("insert_campaign_event"))?;
    Ok(())
}

fn map_summary(row: &sqlx::sqlite::SqliteRow) -> Result<CampaignSummary, RepositoryError> {
    Ok(CampaignSummary {
        id: CampaignId(parse_uuid(column(row, "id")?, "decode_campaign_id")?),
        schema_version: parse_schema(column(row, "schema_version")?)?,
        ruleset_id: column(row, "ruleset_id")?,
        title: column(row, "title")?,
        premise: column(row, "premise")?,
        last_session_id: SessionId(parse_uuid(
            column(row, "session_id")?,
            "decode_campaign_session_id",
        )?),
        revision: parse_revision(column(row, "revision")?)?,
        updated_at_epoch_ms: parse_epoch(column(row, "updated_at")?)?,
    })
}

fn map_detail(row: sqlx::sqlite::SqliteRow) -> Result<CampaignDetail, RepositoryError> {
    let summary = map_summary(&row)?;
    let payload: serde_json::Value = serde_json::from_str(&column::<String>(&row, "payload_json")?)
        .map_err(|_| repository_error("decode_campaign_event_payload"))?;
    let seed = serde_json::from_value::<CampaignSeed>(
        payload
            .get("seed")
            .cloned()
            .ok_or_else(|| repository_error("missing_campaign_seed"))?,
    )
    .map_err(|_| repository_error("decode_campaign_seed"))?;
    Ok(CampaignDetail {
        id: summary.id,
        schema_version: summary.schema_version,
        ruleset_id: summary.ruleset_id,
        title: summary.title,
        premise: summary.premise,
        seed,
        last_session_id: summary.last_session_id,
        revision: summary.revision,
        created_at_epoch_ms: parse_epoch(column(&row, "created_at")?)?,
        updated_at_epoch_ms: summary.updated_at_epoch_ms,
    })
}

pub(crate) fn revision_i64(revision: Revision) -> Result<i64, RepositoryError> {
    i64::try_from(revision.0).map_err(|_| repository_error("encode_campaign_revision"))
}

pub(crate) fn parse_revision(value: i64) -> Result<Revision, RepositoryError> {
    u64::try_from(value)
        .map(Revision)
        .map_err(|_| repository_error("decode_campaign_revision"))
}

pub(crate) fn parse_epoch(value: String) -> Result<i64, RepositoryError> {
    value
        .parse()
        .map_err(|_| repository_error("decode_campaign_timestamp"))
}

pub(crate) fn parse_uuid(
    value: String,
    operation: &'static str,
) -> Result<uuid::Uuid, RepositoryError> {
    uuid::Uuid::parse_str(&value).map_err(|_| repository_error(operation))
}

fn parse_schema(value: i64) -> Result<u16, RepositoryError> {
    u16::try_from(value).map_err(|_| repository_error("decode_campaign_schema"))
}

pub(crate) fn column<T>(
    row: &sqlx::sqlite::SqliteRow,
    name: &'static str,
) -> Result<T, RepositoryError>
where
    for<'row> T: Decode<'row, Sqlite> + Type<Sqlite>,
{
    row.try_get(name)
        .map_err(|_| repository_error("decode_campaign_row"))
}
