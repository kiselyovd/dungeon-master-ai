use app_application::models::combat::CombatProjection;
use app_application::ports::repositories::{CombatRepository, RepositoryError};
use async_trait::async_trait;
use sqlx::Row;
use uuid::Uuid;

use crate::messages::repository_error;
use crate::SqliteStore;

#[async_trait]
impl CombatRepository for SqliteStore {
    async fn find_action(
        &self,
        encounter_id: Uuid,
        request_id: &str,
        signature: &str,
    ) -> Result<Option<CombatProjection>, RepositoryError> {
        let row=sqlx::query("SELECT signature,projection_json FROM combat_action_receipts WHERE encounter_id=?1 AND request_id=?2")
            .bind(encounter_id.to_string()).bind(request_id).fetch_optional(&self.pool).await.map_err(|_|repository_error("read_combat_receipt"))?;
        row.map(|row| decode_receipt(&row, signature)).transpose()
    }

    async fn commit_action(
        &self,
        expected_revision: u64,
        mut projection: CombatProjection,
        request_id: &str,
        signature: &str,
    ) -> Result<app_application::models::combat::CombatActionCommit, RepositoryError> {
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| repository_error("begin_combat_action"))?;
        let row=sqlx::query("SELECT signature,projection_json FROM combat_action_receipts WHERE encounter_id=?1 AND request_id=?2")
            .bind(projection.encounter_id.to_string()).bind(request_id).fetch_optional(&mut *tx).await.map_err(|_|repository_error("read_combat_receipt"))?;
        if let Some(row) = row {
            let recorded = decode_receipt(&row, signature)?;
            tx.commit()
                .await
                .map_err(|_| repository_error("commit_combat_replay"))?;
            return Ok(app_application::models::combat::CombatActionCommit {
                projection: recorded,
                duplicate: true,
            });
        }
        if let Some(scope) = &projection.campaign_scope {
            if let Some(prepared) = crate::human_commands::require_prepared_context(
                &mut tx,
                request_id,
                scope.campaign_id,
                scope.session_id,
            )
            .await?
            {
                let stamp = prepared
                    .stamp
                    .combat
                    .ok_or_else(|| repository_error("human_command_context_invalid"))?;
                let app_application::agent::human_commands::HumanAction::Combat(action) =
                    prepared.binding.signature
                else {
                    return Err(repository_error("human_command_signature_mismatch"));
                };
                let actual: serde_json::Value = serde_json::from_str(signature)
                    .map_err(|_| repository_error("human_command_data_invalid"))?;
                if stamp.encounter_id != projection.encounter_id
                    || stamp.revision != expected_revision
                    || actual
                        != serde_json::json!({"encounter_id":stamp.encounter_id,"expected_revision":stamp.revision,"action":action.for_actor_id(stamp.actor_token)})
                {
                    return Err(repository_error("human_command_context_stale"));
                }
            }
        }
        if projection.campaign_scope.is_none()
            && crate::human_commands::has_bound_manifest(&mut tx, request_id).await?
        {
            return Err(repository_error("human_command_context_invalid"));
        }
        persist_projection(&mut tx, expected_revision, &mut projection).await?;
        let raw = serde_json::to_string(&projection)
            .map_err(|_| repository_error("encode_combat_receipt"))?;
        sqlx::query("INSERT INTO combat_action_receipts(encounter_id,request_id,signature,projection_json,created_at) VALUES(?1,?2,?3,?4,?5)")
            .bind(projection.encounter_id.to_string()).bind(request_id).bind(signature).bind(raw).bind(chrono::Utc::now().to_rfc3339()).execute(&mut *tx).await.map_err(|_|repository_error("insert_combat_receipt"))?;
        tx.commit()
            .await
            .map_err(|_| repository_error("commit_combat_action"))?;
        Ok(app_application::models::combat::CombatActionCommit {
            projection,
            duplicate: false,
        })
    }
    #[tracing::instrument(skip_all, fields(
        encounter_id = %projection.encounter_id,
        session_id = %session_id,
        repository = "combat"
    ))]
    async fn create(
        &self,
        session_id: Uuid,
        projection: CombatProjection,
    ) -> Result<CombatProjection, RepositoryError> {
        let mut transaction = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| repository_error("begin_combat_create"))?;
        let projection =
            create_in_transaction(&mut transaction, session_id, projection, true).await?;
        transaction
            .commit()
            .await
            .map_err(|_| repository_error("commit_combat_create"))?;
        Ok(projection)
    }

    #[tracing::instrument(skip_all, fields(encounter_id = %encounter_id, repository = "combat"))]
    async fn get(&self, encounter_id: Uuid) -> Result<Option<CombatProjection>, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| repository_error("begin_combat_read"))?;
        let row = sqlx::query(
            "SELECT projection FROM combat_encounters WHERE id = ?1 AND ended_at IS NULL",
        )
        .bind(encounter_id.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| repository_error("get_combat"))?;
        let mut projection: Option<CombatProjection> = row
            .map(|row| {
                let projection: Option<String> = row
                    .try_get("projection")
                    .map_err(|_| repository_error("decode_combat"))?;
                projection
                    .ok_or_else(|| repository_error("missing_combat_projection"))
                    .and_then(|value| {
                        serde_json::from_str(&value)
                            .map_err(|_| repository_error("decode_combat_projection"))
                    })
            })
            .transpose()?;
        if let Some(projection) = &mut projection {
            crate::combat_binding::recover_loadouts(&mut tx, projection).await?;
        }
        tx.commit()
            .await
            .map_err(|_| repository_error("commit_combat_read"))?;
        Ok(projection)
    }

    #[tracing::instrument(skip_all, fields(
        encounter_id = %projection.encounter_id,
        expected_revision,
        next_revision = projection.revision,
        repository = "combat"
    ))]
    async fn compare_and_set(
        &self,
        expected_revision: u64,
        mut projection: CombatProjection,
    ) -> Result<(), RepositoryError> {
        let mut transaction = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| repository_error("begin_combat"))?;
        persist_projection(&mut transaction, expected_revision, &mut projection).await?;
        transaction
            .commit()
            .await
            .map_err(|_| repository_error("commit_combat"))?;
        Ok(())
    }

    #[tracing::instrument(skip_all, fields(encounter_id = %encounter_id, repository = "combat"))]
    async fn end(&self, encounter_id: Uuid) -> Result<Option<CombatProjection>, RepositoryError> {
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| repository_error("begin_combat_end"))?;
        let raw: Option<String> = sqlx::query_scalar(
            "SELECT projection FROM combat_encounters WHERE id=?1 AND ended_at IS NULL",
        )
        .bind(encounter_id.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| repository_error("read_combat_end"))?;
        let Some(raw) = raw else { return Ok(None) };
        let mut projection: CombatProjection =
            serde_json::from_str(&raw).map_err(|_| repository_error("decode_combat_end"))?;
        app_application::combat::authored::prepare_end(&mut projection)
            .map_err(repository_error)?;
        let previous = projection.revision;
        projection.revision = previous
            .checked_add(1)
            .ok_or_else(|| repository_error("combat_revision_invalid"))?;
        projection.snapshot.active = false;
        persist_projection(&mut tx, previous, &mut projection).await?;
        sqlx::query("UPDATE combat_encounters SET ended_at=?1 WHERE id=?2")
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(encounter_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|_| repository_error("end_combat"))?;
        tx.commit()
            .await
            .map_err(|_| repository_error("commit_combat_end"))?;
        Ok(Some(projection))
    }
}

pub(crate) async fn persist_projection(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    expected_revision: u64,
    projection: &mut CombatProjection,
) -> Result<(), RepositoryError> {
    if expected_revision.checked_add(1) != Some(projection.revision)
        || projection.revision > i64::MAX as u64
    {
        return Err(repository_error("combat_revision_invalid"));
    }
    let session: String =
        sqlx::query_scalar("SELECT session_id FROM combat_encounters WHERE id=?1")
            .bind(projection.encounter_id.to_string())
            .fetch_one(&mut **transaction)
            .await
            .map_err(|_| repository_error("read_combat_session"))?;
    let previous: String =
        sqlx::query_scalar("SELECT projection FROM combat_encounters WHERE id=?1")
            .bind(projection.encounter_id.to_string())
            .fetch_one(&mut **transaction)
            .await
            .map_err(|_| repository_error("read_combat_bindings"))?;
    let previous: CombatProjection = serde_json::from_str(&previous)
        .map_err(|_| repository_error("decode_combat_projection"))?;
    if previous.campaign_scope.is_some() {
        let bindings = |p: &CombatProjection| {
            let mut ids = p
                .snapshot
                .combatants
                .iter()
                .filter_map(|a| a.character_id.map(|owner| (owner.0, a.id.0)))
                .collect::<Vec<_>>();
            ids.sort();
            ids
        };
        if bindings(&previous) != bindings(projection) {
            return Err(repository_error("combat_hero_identity_mismatch"));
        }
    }
    crate::authored_combat::resolve(
        transaction,
        Uuid::parse_str(&session).map_err(|_| repository_error("combat_session_invalid"))?,
        projection,
    )
    .await?;
    crate::combat_binding::sync_hero(transaction, projection).await?;
    let projection_json = serde_json::to_string(&projection)
        .map_err(|_| repository_error("encode_combat_projection"))?;
    let active_turn = projection
        .snapshot
        .current_combatant
        .map(|id| id.0.to_string());
    let updated = sqlx::query(
        "UPDATE combat_encounters SET revision = ?1, projection = ?2, round = ?3, \
             active_turn = ?4, ended_at = ?7 WHERE id = ?5 AND revision = ?6 AND ended_at IS NULL",
    )
    .bind(projection.revision as i64)
    .bind(projection_json)
    .bind(projection.snapshot.round as i64)
    .bind(active_turn)
    .bind(projection.encounter_id.to_string())
    .bind(expected_revision as i64)
    .bind(if projection.snapshot.active {
        None
    } else {
        Some(chrono::Utc::now().to_rfc3339())
    })
    .execute(&mut **transaction)
    .await
    .map_err(|_| repository_error("update_combat_projection"))?;
    if updated.rows_affected() != 1 {
        let actual = sqlx::query("SELECT revision FROM combat_encounters WHERE id = ?1")
            .bind(projection.encounter_id.to_string())
            .fetch_optional(&mut **transaction)
            .await
            .map_err(|_| repository_error("read_combat_revision"))?
            .and_then(|row| row.try_get::<i64, _>("revision").ok())
            .unwrap_or(0) as u64;
        return Err(RepositoryError::RevisionConflict {
            expected: expected_revision,
            actual,
        });
    }
    for combatant in &projection.snapshot.combatants {
        let conditions = serde_json::to_string(&combatant.conditions)
            .map_err(|_| repository_error("encode_combat_conditions"))?;
        sqlx::query(
                "UPDATE combat_tokens SET current_hp = ?1, max_hp = ?2, ac = ?3, \
                 conditions = ?4, is_dead = ?5, pos_x = ?8, pos_y = ?9 WHERE id = ?6 AND encounter_id = ?7",
            )
            .bind(combatant.current_hp)
            .bind(combatant.max_hp)
            .bind(combatant.ac)
            .bind(conditions)
            .bind(i64::from(combatant.is_dead))
            .bind(combatant.id.0.to_string())
            .bind(projection.encounter_id.to_string())
            .bind(combatant.position.x)
            .bind(combatant.position.y)
            .execute(&mut **transaction)
            .await
            .map_err(|_| repository_error("update_combat_token"))?;
    }
    Ok(())
}

fn decode_receipt(
    row: &sqlx::sqlite::SqliteRow,
    signature: &str,
) -> Result<CombatProjection, RepositoryError> {
    if row.get::<String, _>("signature") != signature {
        return Err(RepositoryError::Operation {
            operation: "combat_receipt",
            code: "combat_request_mismatch",
        });
    }
    serde_json::from_str(&row.get::<String, _>("projection_json"))
        .map_err(|_| repository_error("decode_combat_receipt"))
}

pub(crate) async fn create_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    session_id: Uuid,
    mut projection: CombatProjection,
    advance_revision: bool,
) -> Result<CombatProjection, RepositoryError> {
    crate::combat_binding::bind_start(transaction, session_id, &mut projection, advance_revision)
        .await?;
    crate::authored_combat::resolve(transaction, session_id, &mut projection).await?;
    crate::combat_binding::sync_opening_heroes(transaction, &projection).await?;
    let projection_json = serde_json::to_string(&projection)
        .map_err(|_| repository_error("encode_combat_projection"))?;
    let initiative_json = serde_json::to_string(&projection.snapshot.initiative)
        .map_err(|_| repository_error("encode_combat_initiative"))?;
    let active_turn = projection
        .snapshot
        .current_combatant
        .map(|id| id.0.to_string());
    sqlx::query(
            "INSERT INTO combat_encounters \
             (id, session_id, round, active_turn, started_at, initiative, revision, projection, ended_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind(projection.encounter_id.to_string())
        .bind(session_id.to_string())
        .bind(projection.snapshot.round as i64)
        .bind(active_turn)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(initiative_json)
        .bind(projection.revision as i64)
        .bind(projection_json)
        .bind(if projection.snapshot.active{None}else{Some(chrono::Utc::now().to_rfc3339())})
        .execute(&mut **transaction)
        .await
        .map_err(|_| repository_error("create_combat"))?;
    for combatant in &projection.snapshot.combatants {
        let conditions = serde_json::to_string(&combatant.conditions)
            .map_err(|_| repository_error("encode_combat_conditions"))?;
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
            .bind(conditions)
            .bind(i64::from(combatant.is_dead))
            .execute(&mut **transaction)
            .await
            .map_err(|_| repository_error("create_combat_token"))?;
    }
    Ok(projection)
}
