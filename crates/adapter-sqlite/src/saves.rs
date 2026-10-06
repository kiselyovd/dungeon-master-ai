use app_application::models::campaign::CampaignSave;
use app_application::models::identity::{CampaignId, Revision, SessionId};
use app_application::models::snapshot::{
    CampaignSnapshot, CreateSnapshotRecord, RestoreSnapshotRecord, SnapshotCreated,
    SnapshotProjection, SnapshotSaveDetails, SnapshotSaveSummary,
};
use app_application::ports::repositories::{RepositoryError, SaveRepository, SnapshotRepository};
use async_trait::async_trait;
use serde_json::{json, Value};
use sqlx::{Row, Sqlite, Transaction};
use uuid::Uuid;

use crate::campaigns::{column, parse_revision, parse_uuid, revision_i64};
use crate::messages::{decode_message, repository_error};
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

#[async_trait]
impl SnapshotRepository for SqliteStore {
    #[tracing::instrument(skip_all, fields(
        campaign_id = %record.campaign_id.0,
        session_id = %record.session_id.0,
        expected_revision = record.expected_revision.0,
        repository = "snapshots"
    ))]
    async fn create_snapshot(
        &self,
        record: CreateSnapshotRecord,
    ) -> Result<SnapshotCreated, RepositoryError> {
        let mut transaction = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| repository_error("begin_snapshot_create"))?;
        if let Some(existing) =
            snapshot_by_causation(&mut transaction, &record.causation_id).await?
        {
            if existing.snapshot.campaign_id != record.campaign_id
                || existing.snapshot.session_id != record.session_id
            {
                return Err(repository_error("snapshot_causation_mismatch"));
            }
            transaction
                .commit()
                .await
                .map_err(|_| repository_error("commit_snapshot_retry"))?;
            return Ok(existing);
        }
        validate_session(
            &mut transaction,
            record.campaign_id,
            record.session_id,
            record.expected_revision,
            record.schema_version,
            &record.ruleset_id,
        )
        .await?;
        let character = crate::campaign_state::snapshot_character(
            &mut transaction,
            record.campaign_id,
            record.character,
        )
        .await?;
        let projection = capture_projection(
            &mut transaction,
            record.campaign_id,
            record.session_id,
            character,
        )
        .await?;
        let created_at = chrono::Utc::now().to_rfc3339();
        crate::campaign_state::persist_snapshot_character(
            &mut transaction,
            record.campaign_id,
            &projection.character,
            false,
        )
        .await?;
        let snapshot_revision = crate::campaigns::advance_session_revision(
            &mut transaction,
            record.campaign_id,
            record.session_id,
            record.expected_revision,
            "snapshot_created",
            &record.causation_id,
        )
        .await?;
        let snapshot =
            match snapshot_by_revision(&mut transaction, record.campaign_id, snapshot_revision)
                .await?
            {
                Some(snapshot) => snapshot,
                None => {
                    let snapshot = CampaignSnapshot {
                        id: Uuid::new_v4(),
                        campaign_id: record.campaign_id,
                        session_id: record.session_id,
                        revision: snapshot_revision,
                        schema_version: record.schema_version,
                        ruleset_id: record.ruleset_id,
                        projection,
                    };
                    let projection_json = serde_json::to_string(&snapshot.projection)
                        .map_err(|_| repository_error("encode_snapshot_projection"))?;
                    sqlx::query(
                        "INSERT INTO campaign_snapshots \
                     (id, campaign_id, session_id, revision, schema_version, ruleset_id, \
                      projection_json, created_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    )
                    .bind(snapshot.id.to_string())
                    .bind(record.campaign_id.0.to_string())
                    .bind(record.session_id.0.to_string())
                    .bind(revision_i64(snapshot_revision)?)
                    .bind(i64::from(record.schema_version))
                    .bind(&snapshot.ruleset_id)
                    .bind(projection_json)
                    .bind(&created_at)
                    .execute(&mut *transaction)
                    .await
                    .map_err(|_| repository_error("insert_campaign_snapshot"))?;
                    snapshot
                }
            };
        sqlx::query(
            "INSERT INTO saves \
             (id, campaign_id, session_id, snapshot_id, revision, title, kind, summary, tag, causation_id, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        )
        .bind(record.id.to_string())
        .bind(record.campaign_id.0.to_string())
        .bind(record.session_id.0.to_string())
        .bind(snapshot.id.to_string())
        .bind(revision_i64(snapshot_revision)?)
        .bind(record.title)
        .bind(record.kind)
        .bind(record.summary)
        .bind(record.tag)
        .bind(record.causation_id)
        .bind(created_at)
        .execute(&mut *transaction)
        .await
        .map_err(|_| repository_error("insert_campaign_save"))?;
        transaction
            .commit()
            .await
            .map_err(|_| repository_error("commit_snapshot_create"))?;
        Ok(SnapshotCreated {
            save_id: record.id,
            snapshot,
        })
    }

    #[tracing::instrument(skip_all, fields(
        campaign_id = %record.campaign_id.0,
        session_id = %record.session_id.0,
        expected_revision = record.expected_revision.0,
        repository = "snapshots"
    ))]
    async fn restore_snapshot(
        &self,
        record: RestoreSnapshotRecord,
    ) -> Result<CampaignSnapshot, RepositoryError> {
        let mut transaction = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| repository_error("begin_snapshot_restore"))?;
        if let Some(existing) = restored_snapshot_by_causation(
            &mut transaction,
            record.session_id,
            &record.causation_id,
        )
        .await?
        {
            transaction
                .commit()
                .await
                .map_err(|_| repository_error("commit_snapshot_restore_retry"))?;
            return Ok(existing);
        }
        validate_session(
            &mut transaction,
            record.campaign_id,
            record.session_id,
            record.expected_revision,
            1,
            "srd-5.2.1",
        )
        .await?;
        let snapshot = snapshot_by_save(&mut transaction, record.save_id)
            .await?
            .ok_or(RepositoryError::NotFound)?;
        if snapshot.campaign_id != record.campaign_id
            || snapshot.session_id != record.session_id
            || snapshot.revision.0 > record.expected_revision.0
        {
            return Err(repository_error("snapshot_identity_mismatch"));
        }
        apply_projection(&mut transaction, &snapshot).await?;
        crate::campaign_state::persist_snapshot_character(
            &mut transaction,
            record.campaign_id,
            &snapshot.projection.character,
            true,
        )
        .await?;
        let updated = sqlx::query(
            "UPDATE sessions SET active_snapshot_id = ?1 WHERE id = ?2 AND revision = ?3",
        )
        .bind(snapshot.id.to_string())
        .bind(record.session_id.0.to_string())
        .bind(revision_i64(record.expected_revision)?)
        .execute(&mut *transaction)
        .await
        .map_err(|_| repository_error("activate_snapshot"))?;
        if updated.rows_affected() != 1 {
            return Err(RepositoryError::Conflict {
                expected: record.expected_revision,
                actual: current_revision(&mut transaction, record.session_id).await?,
            });
        }
        crate::campaigns::advance_session_revision(
            &mut transaction,
            record.campaign_id,
            record.session_id,
            record.expected_revision,
            "snapshot_restored",
            &record.causation_id,
        )
        .await?;
        sqlx::query(
            "INSERT INTO snapshot_restores \
             (causation_id, save_id, session_id, snapshot_id, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(record.causation_id)
        .bind(record.save_id.to_string())
        .bind(record.session_id.0.to_string())
        .bind(snapshot.id.to_string())
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&mut *transaction)
        .await
        .map_err(|_| repository_error("record_snapshot_restore"))?;
        transaction
            .commit()
            .await
            .map_err(|_| repository_error("commit_snapshot_restore"))?;
        Ok(snapshot)
    }

    async fn list_snapshots(
        &self,
        session_id: SessionId,
    ) -> Result<Vec<SnapshotSaveSummary>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, session_id, snapshot_id, revision, kind, title, summary, tag, created_at \
             FROM saves WHERE session_id = ?1 ORDER BY created_at DESC, id DESC",
        )
        .bind(session_id.0.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|_| repository_error("list_campaign_saves"))?;
        rows.iter().map(map_save_summary).collect()
    }

    async fn get_snapshot(
        &self,
        save_id: Uuid,
    ) -> Result<Option<SnapshotSaveDetails>, RepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| repository_error("begin_snapshot_get"))?;
        let summary_row = sqlx::query(
            "SELECT id, campaign_id, session_id, snapshot_id, revision, kind, title, summary, tag, created_at \
             FROM saves WHERE id = ?1",
        )
        .bind(save_id.to_string())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| repository_error("get_campaign_save"))?;
        let Some(summary_row) = summary_row else {
            return Ok(None);
        };
        let summary = map_save_summary(&summary_row)?;
        let snapshot = snapshot_query(
            &mut transaction,
            "SELECT id, campaign_id, session_id, revision, schema_version, ruleset_id, projection_json \
             FROM campaign_snapshots WHERE id = ?1",
            &summary.snapshot_id.to_string(),
        )
        .await?
        .ok_or_else(|| repository_error("missing_campaign_snapshot"))?;
        transaction
            .commit()
            .await
            .map_err(|_| repository_error("commit_snapshot_get"))?;
        Ok(Some(SnapshotSaveDetails { summary, snapshot }))
    }

    async fn delete_snapshot(&self, save_id: Uuid) -> Result<bool, RepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| repository_error("begin_snapshot_delete"))?;
        let snapshot_id =
            sqlx::query_scalar::<_, Option<String>>("SELECT snapshot_id FROM saves WHERE id = ?1")
                .bind(save_id.to_string())
                .fetch_optional(&mut *transaction)
                .await
                .map_err(|_| repository_error("read_snapshot_for_delete"))?
                .flatten();
        let deleted = sqlx::query("DELETE FROM saves WHERE id = ?1")
            .bind(save_id.to_string())
            .execute(&mut *transaction)
            .await
            .map_err(|_| repository_error("delete_campaign_save"))?;
        if let Some(snapshot_id) = snapshot_id {
            sqlx::query(
                "DELETE FROM campaign_snapshots WHERE id = ?1 \
                 AND NOT EXISTS (SELECT 1 FROM saves WHERE snapshot_id = ?1)",
            )
            .bind(snapshot_id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| repository_error("delete_campaign_snapshot"))?;
        }
        transaction
            .commit()
            .await
            .map_err(|_| repository_error("commit_snapshot_delete"))?;
        Ok(deleted.rows_affected() == 1)
    }

    async fn update_snapshot_metadata(
        &self,
        save_id: Uuid,
        kind: String,
        title: String,
        summary: String,
        tag: String,
    ) -> Result<bool, RepositoryError> {
        let updated = sqlx::query(
            "UPDATE saves SET kind = ?1, title = ?2, summary = ?3, tag = ?4 WHERE id = ?5",
        )
        .bind(kind)
        .bind(title)
        .bind(summary)
        .bind(tag)
        .bind(save_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|_| repository_error("update_campaign_save"))?;
        Ok(updated.rows_affected() == 1)
    }
}

fn map_save_summary(row: &sqlx::sqlite::SqliteRow) -> Result<SnapshotSaveSummary, RepositoryError> {
    Ok(SnapshotSaveSummary {
        id: parse_uuid(column(row, "id")?, "decode_save_id")?,
        campaign_id: CampaignId(parse_uuid(
            column(row, "campaign_id")?,
            "decode_save_campaign_id",
        )?),
        session_id: SessionId(parse_uuid(
            column(row, "session_id")?,
            "decode_save_session_id",
        )?),
        snapshot_id: parse_uuid(column(row, "snapshot_id")?, "decode_save_snapshot_id")?,
        revision: parse_revision(column(row, "revision")?)?,
        kind: column(row, "kind")?,
        title: column(row, "title")?,
        summary: column(row, "summary")?,
        tag: column(row, "tag")?,
        created_at: column(row, "created_at")?,
    })
}

async fn validate_session(
    transaction: &mut Transaction<'_, Sqlite>,
    campaign_id: CampaignId,
    session_id: SessionId,
    expected_revision: Revision,
    schema_version: u16,
    ruleset_id: &str,
) -> Result<(), RepositoryError> {
    let row = sqlx::query(
        "SELECT s.revision, c.schema_version, c.ruleset_id \
         FROM sessions s JOIN campaigns c ON c.id = s.campaign_id \
         WHERE s.id = ?1 AND s.campaign_id = ?2",
    )
    .bind(session_id.0.to_string())
    .bind(campaign_id.0.to_string())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|_| repository_error("read_snapshot_session"))?
    .ok_or(RepositoryError::NotFound)?;
    let actual = parse_revision(column(&row, "revision")?)?;
    if actual != expected_revision {
        return Err(RepositoryError::Conflict {
            expected: expected_revision,
            actual,
        });
    }
    let actual_schema: i64 = column(&row, "schema_version")?;
    let actual_ruleset: String = column(&row, "ruleset_id")?;
    if actual_schema != i64::from(schema_version) || actual_ruleset != ruleset_id {
        return Err(repository_error("snapshot_ruleset_mismatch"));
    }
    Ok(())
}

async fn current_revision(
    transaction: &mut Transaction<'_, Sqlite>,
    session_id: SessionId,
) -> Result<Revision, RepositoryError> {
    let revision = sqlx::query_scalar::<_, i64>("SELECT revision FROM sessions WHERE id = ?1")
        .bind(session_id.0.to_string())
        .fetch_one(&mut **transaction)
        .await
        .map_err(|_| repository_error("read_snapshot_revision"))?;
    parse_revision(revision)
}

pub(crate) async fn capture_projection(
    transaction: &mut Transaction<'_, Sqlite>,
    campaign_id: CampaignId,
    session_id: SessionId,
    character: Value,
) -> Result<SnapshotProjection, RepositoryError> {
    let message_rows = sqlx::query(
        "SELECT role, parts, tool_calls, tool_call_id, is_error FROM messages \
         WHERE session_id = ?1 ORDER BY created_at ASC, id ASC",
    )
    .bind(session_id.0.to_string())
    .fetch_all(&mut **transaction)
    .await
    .map_err(|_| repository_error("snapshot_messages"))?;
    let messages = message_rows
        .into_iter()
        .map(|row| {
            decode_message(
                &column::<String>(&row, "role")?,
                &column::<String>(&row, "parts")?,
                column::<Option<String>>(&row, "tool_calls")?.as_deref(),
                column::<Option<String>>(&row, "tool_call_id")?.as_deref(),
                column::<i64>(&row, "is_error")? != 0,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let combat = sqlx::query_scalar::<_, Option<String>>(
        "SELECT projection FROM combat_encounters \
         WHERE session_id = ?1 AND ended_at IS NULL ORDER BY started_at DESC LIMIT 1",
    )
    .bind(session_id.0.to_string())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|_| repository_error("snapshot_combat"))?
    .flatten()
    .map(|value| serde_json::from_str(&value))
    .transpose()
    .map_err(|_| repository_error("decode_snapshot_combat"))?;
    let scene = sqlx::query(
        "SELECT id, title, subtitle, mode FROM scenes WHERE campaign_id = ?1 \
         ORDER BY created_at DESC, id DESC LIMIT 1",
    )
    .bind(campaign_id.0.to_string())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|_| repository_error("snapshot_scene"))?
    .map(|row| {
        Ok(json!({
            "id": column::<String>(&row, "id")?,
            "title": column::<String>(&row, "title")?,
            "subtitle": column::<Option<String>>(&row, "subtitle")?,
            "mode": column::<String>(&row, "mode")?,
        }))
    })
    .transpose()?;
    let journal = capture_rows(
        transaction,
        "SELECT id, chapter, entry_html, created_at FROM journal_entries \
         WHERE campaign_id = ?1 ORDER BY created_at ASC, id ASC",
        campaign_id,
        |row| {
            Ok(json!({
                "id": column::<String>(row, "id")?,
                "campaign_id": campaign_id.0,
                "chapter": column::<Option<String>>(row, "chapter")?,
                "entry_html": column::<String>(row, "entry_html")?,
                "created_at": column::<String>(row, "created_at")?,
            }))
        },
    )
    .await?;
    let npcs = capture_rows(
        transaction,
        "SELECT id, name, role, disposition, trust, facts, updated_at FROM npc_memory \
         WHERE campaign_id = ?1 ORDER BY name ASC",
        campaign_id,
        |row| {
            Ok(json!({
                "id": column::<String>(row, "id")?,
                "campaign_id": campaign_id.0,
                "name": column::<String>(row, "name")?,
                "role": column::<String>(row, "role")?,
                "disposition": column::<String>(row, "disposition")?,
                "trust": column::<i64>(row, "trust")?,
                "facts": serde_json::from_str::<Value>(&column::<String>(row, "facts")?)
                    .map_err(|_| repository_error("decode_snapshot_npc"))?,
                "updated_at": column::<String>(row, "updated_at")?,
            }))
        },
    )
    .await?;
    Ok(SnapshotProjection {
        adventure: crate::adventure::read(transaction, campaign_id).await?,
        messages,
        character,
        combat,
        scene,
        journal,
        npcs,
    })
}

async fn capture_rows<F>(
    transaction: &mut Transaction<'_, Sqlite>,
    query: &'static str,
    campaign_id: CampaignId,
    map: F,
) -> Result<Vec<Value>, RepositoryError>
where
    F: Fn(&sqlx::sqlite::SqliteRow) -> Result<Value, RepositoryError>,
{
    sqlx::query(query)
        .bind(campaign_id.0.to_string())
        .fetch_all(&mut **transaction)
        .await
        .map_err(|_| repository_error("snapshot_projection_rows"))?
        .iter()
        .map(map)
        .collect()
}

async fn snapshot_by_causation(
    transaction: &mut Transaction<'_, Sqlite>,
    causation_id: &str,
) -> Result<Option<SnapshotCreated>, RepositoryError> {
    let row = sqlx::query_scalar::<_, String>("SELECT id FROM saves WHERE causation_id = ?1")
        .bind(causation_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(|_| repository_error("read_snapshot_causation"))?;
    match row {
        Some(save_id) => {
            let save_id = parse_uuid(save_id, "decode_causation_save_id")?;
            Ok(snapshot_by_save(transaction, save_id)
                .await?
                .map(|snapshot| SnapshotCreated { save_id, snapshot }))
        }
        None => Ok(None),
    }
}

async fn snapshot_by_revision(
    transaction: &mut Transaction<'_, Sqlite>,
    campaign_id: CampaignId,
    revision: Revision,
) -> Result<Option<CampaignSnapshot>, RepositoryError> {
    let row = sqlx::query(
        "SELECT id, campaign_id, session_id, revision, schema_version, ruleset_id, projection_json \
         FROM campaign_snapshots WHERE campaign_id = ?1 AND revision = ?2",
    )
    .bind(campaign_id.0.to_string())
    .bind(revision_i64(revision)?)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|_| repository_error("read_snapshot_revision"))?;
    row.map(map_snapshot).transpose()
}

async fn snapshot_by_save(
    transaction: &mut Transaction<'_, Sqlite>,
    save_id: Uuid,
) -> Result<Option<CampaignSnapshot>, RepositoryError> {
    snapshot_query(
        transaction,
        "SELECT cs.id, cs.campaign_id, cs.session_id, cs.revision, cs.schema_version, \
                cs.ruleset_id, cs.projection_json \
         FROM saves s JOIN campaign_snapshots cs ON cs.id = s.snapshot_id WHERE s.id = ?1",
        &save_id.to_string(),
    )
    .await
}

async fn restored_snapshot_by_causation(
    transaction: &mut Transaction<'_, Sqlite>,
    session_id: SessionId,
    causation_id: &str,
) -> Result<Option<CampaignSnapshot>, RepositoryError> {
    let row = sqlx::query_scalar::<_, String>(
        "SELECT snapshot_id FROM snapshot_restores WHERE causation_id = ?1 AND session_id = ?2",
    )
    .bind(causation_id)
    .bind(session_id.0.to_string())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|_| repository_error("read_snapshot_restore_retry"))?;
    match row {
        Some(snapshot_id) => {
            snapshot_query(
                transaction,
                "SELECT id, campaign_id, session_id, revision, schema_version, ruleset_id, projection_json \
                 FROM campaign_snapshots WHERE id = ?1",
                &snapshot_id,
            )
            .await
        }
        None => Ok(None),
    }
}

async fn snapshot_query(
    transaction: &mut Transaction<'_, Sqlite>,
    query: &'static str,
    key: &str,
) -> Result<Option<CampaignSnapshot>, RepositoryError> {
    sqlx::query(query)
        .bind(key)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(|_| repository_error("read_campaign_snapshot"))?
        .map(map_snapshot)
        .transpose()
}

fn map_snapshot(row: sqlx::sqlite::SqliteRow) -> Result<CampaignSnapshot, RepositoryError> {
    Ok(CampaignSnapshot {
        id: parse_uuid(column(&row, "id")?, "decode_snapshot_id")?,
        campaign_id: CampaignId(parse_uuid(
            column(&row, "campaign_id")?,
            "decode_snapshot_campaign_id",
        )?),
        session_id: SessionId(parse_uuid(
            column(&row, "session_id")?,
            "decode_snapshot_session_id",
        )?),
        revision: parse_revision(column(&row, "revision")?)?,
        schema_version: u16::try_from(column::<i64>(&row, "schema_version")?)
            .map_err(|_| repository_error("decode_snapshot_schema"))?,
        ruleset_id: column(&row, "ruleset_id")?,
        projection: serde_json::from_str(&column::<String>(&row, "projection_json")?)
            .map_err(|_| repository_error("decode_snapshot_projection"))?,
    })
}

async fn apply_projection(
    transaction: &mut Transaction<'_, Sqlite>,
    snapshot: &CampaignSnapshot,
) -> Result<(), RepositoryError> {
    let session_id = snapshot.session_id.0.to_string();
    sqlx::query("DELETE FROM messages WHERE session_id = ?1")
        .bind(&session_id)
        .execute(&mut **transaction)
        .await
        .map_err(|_| repository_error("clear_snapshot_messages"))?;
    for (index, message) in snapshot.projection.messages.iter().enumerate() {
        let (role, parts, tool_calls, tool_call_id, is_error) =
            crate::messages::encode_message(message);
        sqlx::query(
            "INSERT INTO messages \
             (id, session_id, role, parts, tool_calls, tool_call_id, is_error, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&session_id)
        .bind(role)
        .bind(parts)
        .bind(tool_calls)
        .bind(tool_call_id)
        .bind(i64::from(is_error))
        .bind(format!("{:020}", index))
        .execute(&mut **transaction)
        .await
        .map_err(|_| repository_error("restore_snapshot_message"))?;
    }
    restore_scene(transaction, snapshot).await?;
    restore_journal(transaction, snapshot).await?;
    restore_npcs(transaction, snapshot).await?;
    crate::adventure::persist(
        transaction,
        snapshot.campaign_id,
        snapshot
            .projection
            .adventure
            .as_ref()
            .map(|projection| &projection.state),
    )
    .await?;
    restore_combat(transaction, snapshot).await
}

async fn restore_scene(
    transaction: &mut Transaction<'_, Sqlite>,
    snapshot: &CampaignSnapshot,
) -> Result<(), RepositoryError> {
    sqlx::query("DELETE FROM scenes WHERE campaign_id = ?1")
        .bind(snapshot.campaign_id.0.to_string())
        .execute(&mut **transaction)
        .await
        .map_err(|_| repository_error("clear_snapshot_scene"))?;
    if let Some(scene) = &snapshot.projection.scene {
        sqlx::query(
            "INSERT INTO scenes (id, campaign_id, title, subtitle, mode, image_prompt, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6)",
        )
        .bind(scene["id"].as_str().unwrap_or_default())
        .bind(snapshot.campaign_id.0.to_string())
        .bind(scene["title"].as_str().unwrap_or_default())
        .bind(scene["subtitle"].as_str())
        .bind(scene["mode"].as_str().unwrap_or("exploration"))
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&mut **transaction)
        .await
        .map_err(|_| repository_error("restore_snapshot_scene"))?;
    }
    Ok(())
}

async fn restore_journal(
    transaction: &mut Transaction<'_, Sqlite>,
    snapshot: &CampaignSnapshot,
) -> Result<(), RepositoryError> {
    sqlx::query("DELETE FROM journal_entries WHERE campaign_id = ?1")
        .bind(snapshot.campaign_id.0.to_string())
        .execute(&mut **transaction)
        .await
        .map_err(|_| repository_error("clear_snapshot_journal"))?;
    for entry in &snapshot.projection.journal {
        sqlx::query(
            "INSERT INTO journal_entries (id, campaign_id, chapter, entry_html, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(entry["id"].as_str().unwrap_or_default())
        .bind(snapshot.campaign_id.0.to_string())
        .bind(entry["chapter"].as_str())
        .bind(entry["entry_html"].as_str().unwrap_or_default())
        .bind(entry["created_at"].as_str().unwrap_or_default())
        .execute(&mut **transaction)
        .await
        .map_err(|_| repository_error("restore_snapshot_journal"))?;
    }
    Ok(())
}

async fn restore_npcs(
    transaction: &mut Transaction<'_, Sqlite>,
    snapshot: &CampaignSnapshot,
) -> Result<(), RepositoryError> {
    sqlx::query("DELETE FROM npc_memory WHERE campaign_id = ?1")
        .bind(snapshot.campaign_id.0.to_string())
        .execute(&mut **transaction)
        .await
        .map_err(|_| repository_error("clear_snapshot_npcs"))?;
    for npc in &snapshot.projection.npcs {
        sqlx::query(
            "INSERT INTO npc_memory \
             (id, campaign_id, name, role, disposition, trust, facts, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(npc["id"].as_str().unwrap_or_default())
        .bind(snapshot.campaign_id.0.to_string())
        .bind(npc["name"].as_str().unwrap_or_default())
        .bind(npc["role"].as_str().unwrap_or_default())
        .bind(npc["disposition"].as_str().unwrap_or("unknown"))
        .bind(npc["trust"].as_i64().unwrap_or_default())
        .bind(serde_json::to_string(&npc["facts"]).unwrap_or_else(|_| "[]".into()))
        .bind(npc["updated_at"].as_str().unwrap_or_default())
        .execute(&mut **transaction)
        .await
        .map_err(|_| repository_error("restore_snapshot_npc"))?;
    }
    Ok(())
}

async fn restore_combat(
    transaction: &mut Transaction<'_, Sqlite>,
    snapshot: &CampaignSnapshot,
) -> Result<(), RepositoryError> {
    sqlx::query(
        "UPDATE combat_encounters SET ended_at = ?1 WHERE session_id = ?2 AND ended_at IS NULL",
    )
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(snapshot.session_id.0.to_string())
    .execute(&mut **transaction)
    .await
    .map_err(|_| repository_error("clear_snapshot_combat"))?;
    let Some(value) = &snapshot.projection.combat else {
        return Ok(());
    };
    let projection: app_application::models::combat::CombatProjection =
        serde_json::from_value(value.clone())
            .map_err(|_| repository_error("decode_snapshot_combat_projection"))?;
    let initiative = serde_json::to_string(&projection.snapshot.initiative)
        .map_err(|_| repository_error("encode_snapshot_initiative"))?;
    let active_turn = projection
        .snapshot
        .current_combatant
        .map(|combatant| combatant.0.to_string());
    sqlx::query("DELETE FROM combat_tokens WHERE encounter_id = ?1")
        .bind(projection.encounter_id.to_string())
        .execute(&mut **transaction)
        .await
        .map_err(|_| repository_error("clear_snapshot_combat_tokens"))?;
    sqlx::query(
        "INSERT OR REPLACE INTO combat_encounters \
         (id, session_id, round, active_turn, started_at, ended_at, initiative, revision, projection) \
         VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, ?8)",
    )
    .bind(projection.encounter_id.to_string())
    .bind(snapshot.session_id.0.to_string())
    .bind(i64::from(projection.snapshot.round))
    .bind(active_turn)
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(initiative)
    .bind(i64::try_from(projection.revision).unwrap_or(i64::MAX))
    .bind(serde_json::to_string(&projection).map_err(|_| repository_error("encode_snapshot_combat"))?)
    .execute(&mut **transaction)
    .await
    .map_err(|_| repository_error("restore_snapshot_combat"))?;
    for combatant in &projection.snapshot.combatants {
        sqlx::query(
            "INSERT INTO combat_tokens \
             (id, encounter_id, name, current_hp, max_hp, ac, pos_x, pos_y, conditions, is_dead) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )
        .bind(combatant.id.0.to_string())
        .bind(projection.encounter_id.to_string())
        .bind(&combatant.name)
        .bind(combatant.current_hp)
        .bind(combatant.max_hp)
        .bind(combatant.ac)
        .bind(combatant.position.x)
        .bind(combatant.position.y)
        .bind(serde_json::to_string(&combatant.conditions).unwrap_or_else(|_| "[]".into()))
        .bind(i64::from(combatant.is_dead))
        .execute(&mut **transaction)
        .await
        .map_err(|_| repository_error("restore_snapshot_combat_token"))?;
    }
    Ok(())
}
