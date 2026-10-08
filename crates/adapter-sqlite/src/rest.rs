use crate::{messages::repository_error, SqliteStore};
use app_application::{
    models::{
        identity::{CampaignId, SessionId},
        rest::{AdventuringState, RestCommand, RestResult, RestStatus},
    },
    ports::repositories::{RepositoryError, RestRepository},
};
use async_trait::async_trait;
use sqlx::{Row, Sqlite, Transaction};

pub(crate) async fn active_combat(
    tx: &mut Transaction<'_, Sqlite>,
    session: SessionId,
) -> Result<bool, RepositoryError> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM combat_encounters WHERE session_id=?1 AND ended_at IS NULL)",
    )
    .bind(session.0.to_string())
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| repository_error("read_rest_combat"))
}

pub(crate) async fn read_clock(
    tx: &mut Transaction<'_, Sqlite>,
    session: SessionId,
) -> Result<Option<AdventuringState>, RepositoryError> {
    let value: Option<i64> =
        sqlx::query_scalar("SELECT elapsed_seconds FROM adventuring_states WHERE session_id=?1")
            .bind(session.0.to_string())
            .fetch_optional(&mut **tx)
            .await
            .map_err(|_| repository_error("read_adventuring_clock"))?;
    value
        .map(|value| {
            u64::try_from(value)
                .map(|elapsed_seconds| AdventuringState { elapsed_seconds })
                .map_err(|_| repository_error("adventuring_clock_invalid"))
        })
        .transpose()
}
async fn persist_clock(
    tx: &mut Transaction<'_, Sqlite>,
    session: SessionId,
    seconds: u64,
) -> Result<(), RepositoryError> {
    let seconds =
        i64::try_from(seconds).map_err(|_| repository_error("adventuring_clock_invalid"))?;
    sqlx::query("INSERT INTO adventuring_states(session_id,elapsed_seconds) VALUES(?1,?2) ON CONFLICT(session_id) DO UPDATE SET elapsed_seconds=excluded.elapsed_seconds").bind(session.0.to_string()).bind(seconds).execute(&mut **tx).await.map_err(|_|repository_error("write_adventuring_clock"))?;
    Ok(())
}
pub(crate) async fn write_clock(
    tx: &mut Transaction<'_, Sqlite>,
    session: SessionId,
    seconds: u64,
) -> Result<(), RepositoryError> {
    write_clock_with_rng(
        tx,
        session,
        seconds,
        &mut app_application::recovery::RecoveryEngine::random(),
    )
    .await?;
    Ok(())
}
pub(crate) async fn write_clock_with_rng(
    tx: &mut Transaction<'_, Sqlite>,
    session: SessionId,
    seconds: u64,
    rng: &mut app_application::recovery::RecoveryEngine,
) -> Result<Vec<app_application::models::room_recovery::RecoveryDeathSave>, RepositoryError> {
    let previous = read_clock(tx, session)
        .await?
        .unwrap_or_default()
        .elapsed_seconds;
    persist_clock(tx, session, seconds).await?;
    crate::recovery_clock::passage(tx, session, previous, seconds, rng).await
}
/// Combat owns vitality until its final projection is synchronized, including opening.
pub(crate) async fn advance_combat_clock(
    tx: &mut Transaction<'_, Sqlite>,
    session: SessionId,
    seconds: u64,
) -> Result<(), RepositoryError> {
    if seconds == 0 {
        return Ok(());
    }
    let now = read_clock(tx, session)
        .await?
        .unwrap_or_default()
        .elapsed_seconds;
    persist_clock(
        tx,
        session,
        now.checked_add(seconds)
            .ok_or_else(|| repository_error("adventuring_clock_invalid"))?,
    )
    .await
}
pub(crate) async fn advance_clock(
    tx: &mut Transaction<'_, Sqlite>,
    session: SessionId,
    seconds: u64,
) -> Result<(), RepositoryError> {
    if seconds == 0 {
        return Ok(());
    }
    let current = read_clock(tx, session)
        .await?
        .unwrap_or_default()
        .elapsed_seconds;
    write_clock(
        tx,
        session,
        current
            .checked_add(seconds)
            .ok_or_else(|| repository_error("adventuring_clock_invalid"))?,
    )
    .await
}
pub(crate) async fn restore_clock(
    tx: &mut Transaction<'_, Sqlite>,
    session: SessionId,
    state: Option<&AdventuringState>,
) -> Result<(), RepositoryError> {
    sqlx::query("DELETE FROM rest_operations WHERE session_id=?1")
        .bind(session.0.to_string())
        .execute(&mut **tx)
        .await
        .map_err(|_| repository_error("clear_rest_receipts"))?;
    sqlx::query("DELETE FROM solo_recovery_receipts WHERE session_id=?1")
        .bind(session.0.to_string())
        .execute(&mut **tx)
        .await
        .map_err(|_| repository_error("clear_solo_recovery_receipts"))?;
    if let Some(state) = state {
        persist_clock(tx, session, state.elapsed_seconds).await?;
    } else {
        sqlx::query("DELETE FROM adventuring_states WHERE session_id=?1")
            .bind(session.0.to_string())
            .execute(&mut **tx)
            .await
            .map_err(|_| repository_error("restore_adventuring_clock"))?;
    }
    Ok(())
}

#[async_trait]
impl RestRepository for SqliteStore {
    async fn status(&self, id: CampaignId) -> Result<RestStatus, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| repository_error("begin_rest_status"))?;
        let current = crate::campaign_state::load_in_transaction(&mut tx, id).await?;
        let mut status = app_application::rest::status_for_state(&current)?;
        status.combat_active |= active_combat(&mut tx, current.session_id).await?;
        tx.commit()
            .await
            .map_err(|_| repository_error("commit_rest_status"))?;
        Ok(status)
    }
    async fn rest(
        &self,
        command: RestCommand,
        rng_seed: u64,
    ) -> Result<RestResult, RepositoryError> {
        let request = serde_json::to_string(&command)
            .map_err(|_| repository_error("encode_rest_intention"))?;
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| repository_error("begin_rest"))?;
        if let Some(row)=sqlx::query("SELECT request_json,result_json FROM rest_operations WHERE session_id=?1 AND causation_id=?2").bind(command.session_id.0.to_string()).bind(&command.causation_id).fetch_optional(&mut *tx).await.map_err(|_|repository_error("read_rest_receipt"))? {
   if row.get::<String,_>("request_json")!=request{return Err(repository_error("rest_causation_mismatch"));}
   let result=serde_json::from_str(&row.get::<String,_>("result_json")).map_err(|_|repository_error("decode_rest_receipt"))?;
   tx.commit().await.map_err(|_|repository_error("commit_rest_retry"))?;return Ok(result);
  }
        if let Some(prepared) = crate::human_commands::require_prepared_context(
            &mut tx,
            &command.causation_id,
            command.campaign_id.0,
            command.session_id.0,
        )
        .await?
        {
            if prepared.binding.signature
                != app_application::agent::human_commands::HumanAction::Rest(command.intent)
                || prepared.binding.scope.actor_id != command.character_id
                || prepared.stamp.campaign_revision != command.expected_revision.0
            {
                return Err(repository_error("human_command_signature_mismatch"));
            }
        }
        let current =
            crate::campaign_state::load_in_transaction(&mut tx, command.campaign_id).await?;
        if current.session_id != command.session_id
            || current.character_id != Some(command.character_id)
        {
            return Err(repository_error("rest_identity_mismatch"));
        }
        if current.campaign.revision != command.expected_revision {
            return Err(RepositoryError::Conflict {
                expected: command.expected_revision,
                actual: current.campaign.revision,
            });
        }
        if active_combat(&mut tx, current.session_id).await? {
            return Err(repository_error("rest_unsafe"));
        }
        let outcome = app_application::rest::resolve_for_state(&current, command.intent, rng_seed)?;
        let mut sheet = current.projection.character;
        app_application::rest::apply_rest_to_sheet(&mut sheet, &outcome)
            .map_err(repository_error)?;
        crate::campaign_state::persist_character(
            &mut tx,
            command.campaign_id,
            command.character_id,
            sheet,
        )
        .await?;
        write_clock(&mut tx, command.session_id, outcome.elapsed_seconds).await?;
        crate::campaigns::advance_session_revision(
            &mut tx,
            command.campaign_id,
            command.session_id,
            command.expected_revision,
            "rest_completed",
            &command.causation_id,
        )
        .await?;
        let state =
            crate::campaign_state::load_in_transaction(&mut tx, command.campaign_id).await?;
        let result = RestResult {
            status: app_application::rest::status_for_state(&state)?,
            outcome,
        };
        sqlx::query("INSERT INTO rest_operations(session_id,causation_id,request_json,result_json) VALUES(?1,?2,?3,?4)").bind(command.session_id.0.to_string()).bind(&command.causation_id).bind(request).bind(serde_json::to_string(&result).map_err(|_|repository_error("encode_rest_result"))?).execute(&mut *tx).await.map_err(|_|repository_error("record_rest_receipt"))?;
        tx.commit()
            .await
            .map_err(|_| repository_error("commit_rest"))?;
        Ok(result)
    }
}
