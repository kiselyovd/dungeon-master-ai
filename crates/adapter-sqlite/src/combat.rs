use app_application::models::combat::CombatProjection;
use app_application::ports::repositories::{CombatRepository, RepositoryError};
use async_trait::async_trait;
use sqlx::Row;
use uuid::Uuid;

use crate::messages::repository_error;
use crate::SqliteStore;

#[async_trait]
impl CombatRepository for SqliteStore {
    #[tracing::instrument(skip_all, fields(
        encounter_id = %projection.encounter_id,
        session_id = %session_id,
        repository = "combat"
    ))]
    async fn create(
        &self,
        session_id: Uuid,
        projection: CombatProjection,
    ) -> Result<(), RepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| repository_error("begin_combat_create"))?;
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
             (id, session_id, round, active_turn, started_at, initiative, revision, projection) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(projection.encounter_id.to_string())
        .bind(session_id.to_string())
        .bind(projection.snapshot.round as i64)
        .bind(active_turn)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(initiative_json)
        .bind(projection.revision as i64)
        .bind(projection_json)
        .execute(&mut *transaction)
        .await
        .map_err(|_| repository_error("create_combat"))?;
        for combatant in &projection.snapshot.combatants {
            let conditions = serde_json::to_string(&combatant.conditions)
                .map_err(|_| repository_error("encode_combat_conditions"))?;
            sqlx::query(
                "INSERT INTO combat_tokens \
                 (id, encounter_id, name, current_hp, max_hp, ac, pos_x, pos_y, conditions, is_dead) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 0, ?7, ?8)",
            )
            .bind(combatant.id.0.to_string())
            .bind(projection.encounter_id.to_string())
            .bind(&combatant.name)
            .bind(combatant.current_hp)
            .bind(combatant.max_hp)
            .bind(combatant.ac)
            .bind(conditions)
            .bind(i64::from(combatant.is_dead))
            .execute(&mut *transaction)
            .await
            .map_err(|_| repository_error("create_combat_token"))?;
        }
        transaction
            .commit()
            .await
            .map_err(|_| repository_error("commit_combat_create"))?;
        Ok(())
    }

    #[tracing::instrument(skip_all, fields(encounter_id = %encounter_id, repository = "combat"))]
    async fn get(&self, encounter_id: Uuid) -> Result<Option<CombatProjection>, RepositoryError> {
        let row = sqlx::query(
            "SELECT projection FROM combat_encounters WHERE id = ?1 AND ended_at IS NULL",
        )
        .bind(encounter_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| repository_error("get_combat"))?;
        row.map(|row| {
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
        .transpose()
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
        projection: CombatProjection,
    ) -> Result<(), RepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| repository_error("begin_combat"))?;
        let projection_json = serde_json::to_string(&projection)
            .map_err(|_| repository_error("encode_combat_projection"))?;
        let active_turn = projection
            .snapshot
            .current_combatant
            .map(|id| id.0.to_string());
        let updated = sqlx::query(
            "UPDATE combat_encounters SET revision = ?1, projection = ?2, round = ?3, \
             active_turn = ?4 WHERE id = ?5 AND revision = ?6 AND ended_at IS NULL",
        )
        .bind(projection.revision as i64)
        .bind(projection_json)
        .bind(projection.snapshot.round as i64)
        .bind(active_turn)
        .bind(projection.encounter_id.to_string())
        .bind(expected_revision as i64)
        .execute(&mut *transaction)
        .await
        .map_err(|_| repository_error("update_combat_projection"))?;
        if updated.rows_affected() != 1 {
            let actual = sqlx::query("SELECT revision FROM combat_encounters WHERE id = ?1")
                .bind(projection.encounter_id.to_string())
                .fetch_optional(&mut *transaction)
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
                 conditions = ?4, is_dead = ?5 WHERE id = ?6 AND encounter_id = ?7",
            )
            .bind(combatant.current_hp)
            .bind(combatant.max_hp)
            .bind(combatant.ac)
            .bind(conditions)
            .bind(i64::from(combatant.is_dead))
            .bind(combatant.id.0.to_string())
            .bind(projection.encounter_id.to_string())
            .execute(&mut *transaction)
            .await
            .map_err(|_| repository_error("update_combat_token"))?;
        }
        transaction
            .commit()
            .await
            .map_err(|_| repository_error("commit_combat"))?;
        Ok(())
    }

    #[tracing::instrument(skip_all, fields(encounter_id = %encounter_id, repository = "combat"))]
    async fn end(&self, encounter_id: Uuid) -> Result<Option<CombatProjection>, RepositoryError> {
        let Some(mut projection) = self.get(encounter_id).await? else {
            return Ok(None);
        };
        projection.revision = projection.revision.saturating_add(1);
        projection.snapshot.active = false;
        let projection_json = serde_json::to_string(&projection)
            .map_err(|_| repository_error("encode_combat_projection"))?;
        let updated = sqlx::query(
            "UPDATE combat_encounters SET ended_at = ?1, revision = ?2, projection = ?3 \
             WHERE id = ?4 AND ended_at IS NULL",
        )
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(projection.revision as i64)
        .bind(projection_json)
        .bind(encounter_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|_| repository_error("end_combat"))?;
        if updated.rows_affected() == 0 {
            return Ok(None);
        }
        Ok(Some(projection))
    }
}
