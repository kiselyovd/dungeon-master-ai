use crate::campaigns::{advance_session_revision, map_detail, parse_uuid, CURRENT_CAMPAIGN_QUERY};
use crate::messages::repository_error;
use crate::saves::capture_projection;
use crate::SqliteStore;

pub(crate) async fn snapshot_character(
    tx: &mut Transaction<'_, Sqlite>,
    id: CampaignId,
    supplied: Value,
) -> Result<Value, RepositoryError> {
    if supplied
        .as_object()
        .is_some_and(|document| document.is_empty())
    {
        let current: Option<String> = sqlx::query_scalar(
            "SELECT character_json FROM campaign_characters WHERE campaign_id = ?1",
        )
        .bind(id.0.to_string())
        .fetch_optional(&mut **tx)
        .await
        .map_err(|_| repository_error("read_checkpoint_character"))?;
        if let Some(current) = current {
            return serde_json::from_str(&current)
                .map_err(|_| repository_error("decode_checkpoint_character"));
        }
    }
    Ok(supplied)
}
use app_application::models::campaign_state::{
    valid_character_document, within_character_budget, CampaignSessionState,
    CommitCampaignCharacter,
};
use app_application::models::identity::CampaignId;
use app_application::ports::repositories::{CampaignStateRepository, RepositoryError};
use async_trait::async_trait;
use serde_json::{json, Value};
use sqlx::{Row, Sqlite, Transaction};
use uuid::Uuid;

#[async_trait]
impl CampaignStateRepository for SqliteStore {
    async fn load_current(
        &self,
        campaign_id: CampaignId,
    ) -> Result<CampaignSessionState, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| repository_error("begin_campaign_state"))?;
        let state = load_in_transaction(&mut tx, campaign_id).await?;
        tx.commit()
            .await
            .map_err(|_| repository_error("commit_campaign_state_read"))?;
        Ok(state)
    }

    async fn commit_character(
        &self,
        command: CommitCampaignCharacter,
    ) -> Result<CampaignSessionState, RepositoryError> {
        if command.character_id.is_nil()
            || command.causation_id.trim().is_empty()
            || command.causation_id.len() > 256
            || !valid_character_document(&command.character)
        {
            return Err(repository_error("character_invalid"));
        }
        let request = serde_json::to_string(&command)
            .map_err(|_| repository_error("encode_character_commit"))?;
        if !within_character_budget(&command.character) {
            return Err(repository_error("character_too_large"));
        }
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| repository_error("begin_character_commit"))?;
        if let Some(row) = sqlx::query("SELECT request_json, state_json FROM character_commits WHERE campaign_id = ?1 AND causation_id = ?2")
            .bind(command.campaign_id.0.to_string()).bind(&command.causation_id)
            .fetch_optional(&mut *tx).await.map_err(|_| repository_error("read_character_retry"))? {
            let original: String = row.try_get("request_json").map_err(|_| repository_error("decode_character_retry"))?;
            if original != request { return Err(repository_error("character_causation_mismatch")); }
            let state: String = row.try_get("state_json").map_err(|_| repository_error("decode_character_retry"))?;
            return serde_json::from_str(&state).map_err(|_| repository_error("decode_character_retry"));
        }
        let current = load_in_transaction(&mut tx, command.campaign_id).await?;
        if current.session_id != command.session_id {
            return Err(repository_error("character_session_mismatch"));
        }
        if current.campaign.revision != command.expected_revision {
            return Err(RepositoryError::Conflict {
                expected: command.expected_revision,
                actual: current.campaign.revision,
            });
        }
        if current
            .character_id
            .is_some_and(|id| id != command.character_id)
        {
            return Err(repository_error("character_identity_mismatch"));
        }
        advance_session_revision(
            &mut tx,
            command.campaign_id,
            command.session_id,
            command.expected_revision,
            "character_committed",
            &command.causation_id,
        )
        .await?;
        let mut document = command.character;
        document["characterId"] = json!(command.character_id);
        persist_character(&mut tx, command.campaign_id, command.character_id, document).await?;
        let state = load_in_transaction(&mut tx, command.campaign_id).await?;
        sqlx::query("INSERT INTO character_commits (campaign_id, causation_id, request_json, state_json) VALUES (?1, ?2, ?3, ?4)")
            .bind(command.campaign_id.0.to_string()).bind(&command.causation_id).bind(request)
            .bind(serde_json::to_string(&state).map_err(|_| repository_error("encode_character_receipt"))?)
            .execute(&mut *tx).await.map_err(|_| repository_error("record_character_commit"))?;
        tx.commit()
            .await
            .map_err(|_| repository_error("commit_character"))?;
        Ok(state)
    }
}

pub(crate) async fn load_in_transaction(
    tx: &mut Transaction<'_, Sqlite>,
    id: CampaignId,
) -> Result<CampaignSessionState, RepositoryError> {
    let row = sqlx::query(CURRENT_CAMPAIGN_QUERY)
        .bind(id.0.to_string())
        .fetch_optional(&mut **tx)
        .await
        .map_err(|_| repository_error("load_current_campaign"))?
        .ok_or(RepositoryError::NotFound)?;
    let campaign = map_detail(row)?;
    let character = sqlx::query(
        "SELECT character_id, character_json FROM campaign_characters WHERE campaign_id = ?1",
    )
    .bind(id.0.to_string())
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| repository_error("load_current_character"))?;
    let (character_id, document) = match character {
        None => (None, json!({})),
        Some(row) => {
            let raw: String = row
                .try_get("character_json")
                .map_err(|_| repository_error("decode_current_character"))?;
            let mut doc: Value = serde_json::from_str(&raw)
                .map_err(|_| repository_error("decode_current_character"))?;
            if !valid_character_document(&doc) {
                return Err(repository_error("character_document_invalid"));
            }
            let character_id = parse_uuid(
                row.try_get("character_id")
                    .map_err(|_| repository_error("decode_current_character"))?,
                "character_identity_invalid",
            )?;
            doc["characterId"] = json!(character_id);
            (Some(character_id), doc)
        }
    };
    let session_id = campaign.last_session_id;
    let projection = capture_projection(tx, id, session_id, document).await?;
    if !projection.is_complete() {
        return Err(repository_error("campaign_projection_invalid"));
    }
    Ok(CampaignSessionState {
        campaign,
        session_id,
        character_id,
        projection,
    })
}

async fn persist_character(
    tx: &mut Transaction<'_, Sqlite>,
    id: CampaignId,
    character_id: Uuid,
    document: Value,
) -> Result<(), RepositoryError> {
    sqlx::query("INSERT INTO campaign_characters (campaign_id, character_id, schema_version, ruleset_id, character_json, updated_at)
        VALUES (?1, ?2, 1, 'srd-5.2.1', ?3, ?4) ON CONFLICT(campaign_id) DO UPDATE SET
        character_id = excluded.character_id, character_json = excluded.character_json, updated_at = excluded.updated_at")
        .bind(id.0.to_string()).bind(character_id.to_string())
        .bind(serde_json::to_string(&document).map_err(|_| repository_error("encode_character"))?)
        .bind(chrono::Utc::now().to_rfc3339()).execute(&mut **tx).await
        .map_err(|_| repository_error("persist_current_character"))?;
    Ok(())
}

pub(crate) async fn persist_snapshot_character(
    tx: &mut Transaction<'_, Sqlite>,
    id: CampaignId,
    document: &Value,
    restoring: bool,
) -> Result<(), RepositoryError> {
    if !valid_character_document(document) {
        if restoring {
            sqlx::query("DELETE FROM campaign_characters WHERE campaign_id = ?1")
                .bind(id.0.to_string())
                .execute(&mut **tx)
                .await
                .map_err(|_| repository_error("clear_restored_character"))?;
        }
        return Ok(());
    }
    let current: Option<String> =
        sqlx::query_scalar("SELECT character_id FROM campaign_characters WHERE campaign_id = ?1")
            .bind(id.0.to_string())
            .fetch_optional(&mut **tx)
            .await
            .map_err(|_| repository_error("read_snapshot_character_identity"))?;
    let explicit = document.get("characterId").and_then(Value::as_str);
    let character_id = match explicit.or(current.as_deref()) {
        Some(value) => {
            Uuid::parse_str(value).map_err(|_| repository_error("character_identity_invalid"))?
        }
        None => Uuid::new_v4(),
    };
    let mut document = document.clone();
    document["characterId"] = json!(character_id);
    persist_character(tx, id, character_id, document).await
}
