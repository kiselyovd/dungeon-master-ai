//! Compatibility persistence operations used while legacy agent tools are
//! migrated to dedicated application use cases. SQL and row decoding remain
//! inside the outbound adapter.

use app_application::ports::repositories::RepositoryError;
use sqlx::Row;
use uuid::Uuid;

use crate::messages::repository_error;
use crate::SqliteStore;

#[derive(Debug, Clone)]
pub struct LegacyTokenState {
    pub current_hp: i32,
    pub max_hp: i32,
    pub resistances: Option<String>,
    pub immunities: Option<String>,
    pub vulnerabilities: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct LegacyTokenPatch {
    pub hp: Option<i32>,
    pub position: Option<(i32, i32)>,
    pub resistances: Option<String>,
    pub immunities: Option<String>,
    pub vulnerabilities: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LegacyTokenInsert<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub hp: i32,
    pub max_hp: i32,
    pub ac: i32,
    pub x: i32,
    pub y: i32,
    pub resistances: Option<String>,
    pub immunities: Option<String>,
    pub vulnerabilities: Option<String>,
}

impl SqliteStore {
    pub async fn is_combat_active(&self) -> Result<bool, RepositoryError> {
        sqlx::query("SELECT id FROM combat_encounters WHERE ended_at IS NULL LIMIT 1")
            .fetch_optional(&self.pool)
            .await
            .map(|row| row.is_some())
            .map_err(|_| repository_error("read_active_combat"))
    }

    pub async fn active_combat_id(&self) -> Result<Option<Uuid>, RepositoryError> {
        let row = sqlx::query(
            "SELECT id FROM combat_encounters WHERE ended_at IS NULL \
             ORDER BY started_at DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| repository_error("read_active_combat"))?;
        row.map(|row| {
            let id: String = row
                .try_get("id")
                .map_err(|_| repository_error("decode_active_combat"))?;
            Uuid::parse_str(&id).map_err(|_| repository_error("decode_active_combat_id"))
        })
        .transpose()
    }

    pub async fn token_state(
        &self,
        token_id: &str,
    ) -> Result<Option<LegacyTokenState>, RepositoryError> {
        let row = sqlx::query(
            "SELECT current_hp, max_hp, resistances, immunities, vulnerabilities \
             FROM combat_tokens WHERE id = ?1",
        )
        .bind(token_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| repository_error("read_combat_token"))?;
        row.map(|row| {
            Ok(LegacyTokenState {
                current_hp: row
                    .try_get("current_hp")
                    .map_err(|_| repository_error("decode_combat_token"))?,
                max_hp: row
                    .try_get("max_hp")
                    .map_err(|_| repository_error("decode_combat_token"))?,
                resistances: row
                    .try_get::<Option<String>, _>("resistances")
                    .ok()
                    .flatten(),
                immunities: row
                    .try_get::<Option<String>, _>("immunities")
                    .ok()
                    .flatten(),
                vulnerabilities: row
                    .try_get::<Option<String>, _>("vulnerabilities")
                    .ok()
                    .flatten(),
            })
        })
        .transpose()
    }

    pub async fn set_token_hp(&self, token_id: &str, hp: i32) -> Result<(), RepositoryError> {
        sqlx::query("UPDATE combat_tokens SET current_hp = ?1 WHERE id = ?2")
            .bind(hp)
            .bind(token_id)
            .execute(&self.pool)
            .await
            .map_err(|_| repository_error("update_combat_token_hp"))?;
        Ok(())
    }

    pub async fn add_token(&self, token: LegacyTokenInsert<'_>) -> Result<(), RepositoryError> {
        let encounter_id = self
            .active_combat_id()
            .await?
            .ok_or(RepositoryError::NotFound)?;
        sqlx::query(
            "INSERT OR REPLACE INTO combat_tokens \
             (id, encounter_id, name, current_hp, max_hp, ac, pos_x, pos_y, conditions, \
              resistances, immunities, vulnerabilities) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,'[]',?9,?10,?11)",
        )
        .bind(token.id)
        .bind(encounter_id.to_string())
        .bind(token.name)
        .bind(token.hp)
        .bind(token.max_hp)
        .bind(token.ac)
        .bind(token.x)
        .bind(token.y)
        .bind(token.resistances)
        .bind(token.immunities)
        .bind(token.vulnerabilities)
        .execute(&self.pool)
        .await
        .map_err(|_| repository_error("add_combat_token"))?;
        Ok(())
    }

    pub async fn update_token(
        &self,
        token_id: &str,
        patch: LegacyTokenPatch,
    ) -> Result<(), RepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| repository_error("begin_update_combat_token"))?;
        if let Some(hp) = patch.hp {
            sqlx::query("UPDATE combat_tokens SET current_hp = ?1 WHERE id = ?2")
                .bind(hp)
                .bind(token_id)
                .execute(&mut *transaction)
                .await
                .map_err(|_| repository_error("update_combat_token_hp"))?;
        }
        if let Some((x, y)) = patch.position {
            sqlx::query("UPDATE combat_tokens SET pos_x = ?1, pos_y = ?2 WHERE id = ?3")
                .bind(x)
                .bind(y)
                .bind(token_id)
                .execute(&mut *transaction)
                .await
                .map_err(|_| repository_error("update_combat_token_position"))?;
        }
        if let Some(value) = patch.resistances {
            sqlx::query("UPDATE combat_tokens SET resistances = ?1 WHERE id = ?2")
                .bind(value)
                .bind(token_id)
                .execute(&mut *transaction)
                .await
                .map_err(|_| repository_error("update_combat_token_resistances"))?;
        }
        if let Some(value) = patch.immunities {
            sqlx::query("UPDATE combat_tokens SET immunities = ?1 WHERE id = ?2")
                .bind(value)
                .bind(token_id)
                .execute(&mut *transaction)
                .await
                .map_err(|_| repository_error("update_combat_token_immunities"))?;
        }
        if let Some(value) = patch.vulnerabilities {
            sqlx::query("UPDATE combat_tokens SET vulnerabilities = ?1 WHERE id = ?2")
                .bind(value)
                .bind(token_id)
                .execute(&mut *transaction)
                .await
                .map_err(|_| repository_error("update_combat_token_vulnerabilities"))?;
        }
        transaction
            .commit()
            .await
            .map_err(|_| repository_error("commit_update_combat_token"))?;
        Ok(())
    }

    pub async fn mark_token_dead(&self, token_id: &str) -> Result<(), RepositoryError> {
        sqlx::query("UPDATE combat_tokens SET is_dead = 1 WHERE id = ?1")
            .bind(token_id)
            .execute(&self.pool)
            .await
            .map_err(|_| repository_error("remove_combat_token"))?;
        Ok(())
    }

    pub async fn insert_quick_save(
        &self,
        save_id: Uuid,
        session_id: Uuid,
        game_state: &serde_json::Value,
        title: &str,
        summary: &str,
        tag: &str,
    ) -> Result<(), RepositoryError> {
        let state_json =
            serde_json::to_string(game_state).map_err(|_| repository_error("encode_quick_save"))?;
        sqlx::query(
            "INSERT INTO snapshots \
             (id, session_id, turn_number, created_at, game_state, player_action, kind, title, summary, tag) \
             VALUES (?1, ?2, 0, ?3, ?4, NULL, 'auto', ?5, ?6, ?7)",
        )
        .bind(save_id.to_string())
        .bind(session_id.to_string())
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(state_json)
        .bind(title)
        .bind(summary)
        .bind(tag)
        .execute(&self.pool)
        .await
        .map_err(|_| repository_error("insert_quick_save"))?;
        Ok(())
    }
}
