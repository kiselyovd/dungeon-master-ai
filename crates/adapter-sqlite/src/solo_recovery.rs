use crate::SqliteStore;
use app_application::{
    models::{
        identity::{CampaignId, SessionId},
        room_recovery::{RecoveryIntent, RoomRecoveryReceipt},
        solo_recovery::*,
    },
    ports::repositories::{RepositoryError, SoloRecoveryRepository},
    vitality,
};
use async_trait::async_trait;
use sqlx::{Row, Sqlite, Transaction};
fn error(code: &'static str) -> RepositoryError {
    RepositoryError::Operation {
        operation: "solo_recovery",
        code,
    }
}
async fn require_solo(
    tx: &mut Transaction<'_, Sqlite>,
    id: CampaignId,
) -> Result<(), RepositoryError> {
    let active: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM coop_rooms WHERE campaign_id=?1 AND active=1)",
    )
    .bind(id.0.to_string())
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| error("recovery_read_failed"))?;
    if active {
        return Err(error("recovery_requires_room"));
    }
    Ok(())
}
#[async_trait]
impl SoloRecoveryRepository for SqliteStore {
    async fn recovery_status(&self, id: CampaignId) -> Result<SoloRecoveryStatus, RepositoryError> {
        let mut tx = self
            .pool()
            .begin()
            .await
            .map_err(|_| error("recovery_begin_failed"))?;
        require_solo(&mut tx, id).await?;
        let state = crate::campaign_state::load_in_transaction(&mut tx, id).await?;
        let active = crate::rest::active_combat(&mut tx, state.session_id).await?;
        let result =
            app_application::solo_recovery::status_for_state(&state, active).map_err(error)?;
        tx.commit()
            .await
            .map_err(|_| error("recovery_commit_failed"))?;
        Ok(result)
    }
    async fn recover_solo(
        &self,
        command: SoloRecoveryCommand,
        seed: u64,
    ) -> Result<SoloRecoveryResult, RepositoryError> {
        crate::room_world::nonce(&command.causation_id)?;
        let request =
            serde_json::to_string(&command).map_err(|_| error("rest_intention_invalid"))?;
        // Dedicated receipts avoid collisions with ordinary rest intentions.
        let key = command.causation_id.clone();
        let mut tx = self
            .pool()
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| error("recovery_begin_failed"))?;
        require_solo(&mut tx, command.campaign_id).await?;
        let current =
            crate::campaign_state::load_in_transaction(&mut tx, command.campaign_id).await?;
        if current.session_id != command.session_id
            || current.character_id != Some(command.character_id)
        {
            return Err(error("rest_identity_mismatch"));
        }
        if let Some(row)=sqlx::query("SELECT request_json,result_json FROM solo_recovery_receipts WHERE session_id=?1 AND causation_id=?2").bind(command.session_id.0.to_string()).bind(&key).fetch_optional(&mut *tx).await.map_err(|_|error("recovery_read_failed"))? {
   if row.get::<String,_>("request_json")!=request {return Err(error("rest_causation_mismatch"));}
   let mut result:SoloRecoveryResult=serde_json::from_str(&row.get::<String,_>("result_json")).map_err(|_|error("recovery_read_failed"))?;
   result.receipt.duplicate=true;tx.commit().await.map_err(|_|error("recovery_commit_failed"))?;return Ok(result);
  }
        if current.campaign.revision != command.expected_revision {
            return Err(RepositoryError::Conflict {
                expected: command.expected_revision,
                actual: current.campaign.revision,
            });
        }
        if crate::rest::active_combat(&mut tx, command.session_id).await? {
            return Err(error("recovery_in_combat"));
        }
        let now = current
            .projection
            .adventuring
            .as_ref()
            .map_or(0, |s| s.elapsed_seconds);
        let id = command.character_id;
        let mut sheet = current.projection.character.clone();
        let target = app_application::recovery::actor(&sheet, id).map_err(error)?;
        if target.current_hp != 0 || target.is_dead {
            return Err(error("hero_recovery_not_needed"));
        }
        let mut rng = app_application::recovery::RecoveryEngine::new(seed);
        let until = match command.intent {
            RecoveryIntent::DeathSave {} if !target.death_saves.stable => now
                .checked_add(6)
                .ok_or_else(|| error("adventuring_clock_invalid"))?,
            RecoveryIntent::Wait {
                target_character_id,
            } if target_character_id == id && target.death_saves.stable => {
                if !app_application::recovery::safe(&current) {
                    return Err(error("rest_unsafe"));
                }
                let timer = if let Some(timer) = vitality::read(&sheet, id)
                    .map_err(error)?
                    .and_then(|v| v.natural_recovery)
                {
                    timer
                } else {
                    let timer = rng.natural_deadline(&target, now).map_err(error)?;
                    vitality::store_actor(&mut sheet, &target).map_err(error)?;
                    vitality::schedule_natural_recovery(&mut sheet, id, timer.clone())
                        .map_err(error)?;
                    crate::campaign_state::persist_character(
                        &mut tx,
                        command.campaign_id,
                        id,
                        sheet,
                    )
                    .await?;
                    timer
                };
                timer.ready_at_seconds.max(now)
            }
            _ => return Err(error("hero_aid_invalid")),
        };
        let death_saves = crate::rest::write_clock_with_rng(
            &mut tx,
            SessionId(command.session_id.0),
            until,
            &mut rng,
        )
        .await?;
        let revision = crate::campaigns::advance_session_revision(
            &mut tx,
            command.campaign_id,
            command.session_id,
            command.expected_revision,
            "hero_recovery",
            &key,
        )
        .await?;
        let final_state =
            crate::campaign_state::load_in_transaction(&mut tx, command.campaign_id).await?;
        let status =
            app_application::solo_recovery::status_for_state(&final_state, false).map_err(error)?;
        let vitality = vitality::read(&final_state.projection.character, id).map_err(error)?;
        let result = SoloRecoveryResult {
            receipt: RoomRecoveryReceipt {
                receipt_revision: revision,
                duplicate: false,
                target_character_id: id,
                stabilized: vitality.as_ref().is_some_and(|v| v.death_saves.stable),
                recovered: status.hp > 0,
                medicine: None,
                death_saves,
                ready_at_seconds: status.recovery.ready_at_seconds,
            },
            status,
        };
        sqlx::query("INSERT INTO solo_recovery_receipts(session_id,causation_id,request_json,result_json) VALUES(?1,?2,?3,?4)").bind(command.session_id.0.to_string()).bind(key).bind(request).bind(serde_json::to_string(&result).map_err(|_|error("recovery_write_failed"))?).execute(&mut *tx).await.map_err(|_|error("recovery_write_failed"))?;
        tx.commit()
            .await
            .map_err(|_| error("recovery_commit_failed"))?;
        Ok(result)
    }
}
