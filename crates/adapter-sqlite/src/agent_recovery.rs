//! Scoped read-only recovery. Canonical replay inspection must not repair delivery.
use crate::SqliteStore;
use app_application::{
    agent::{
        human_commands::{CommandAuthority, CommandScope},
        operations::AgentOperation,
        recovery::{project_recovery, AgentRecoveryProjection},
    },
    models::{identity::CampaignId, room::RoomLease},
    ports::repositories::RepositoryError,
};
use sqlx::Row;
use uuid::Uuid;

fn error(code: &'static str) -> RepositoryError {
    RepositoryError::Operation {
        operation: "read_agent_recovery",
        code,
    }
}

impl SqliteStore {
    pub async fn solo_agent_recovery(
        &self,
        campaign: Uuid,
        session: Uuid,
        process_owner: Uuid,
    ) -> Result<Option<AgentRecoveryProjection>, RepositoryError> {
        tracing::debug!(%campaign, %session, "agent.recovery.read_solo");
        let mut tx = self
            .pool()
            .begin()
            .await
            .map_err(|_| error("agent_recovery_read_failed"))?;
        let current =
            crate::campaign_state::load_in_transaction(&mut tx, CampaignId(campaign)).await?;
        let Some(actor_id) = current.character_id else {
            return Ok(None);
        };
        if current.session_id.0 != session {
            return Ok(None);
        }
        let room: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM coop_rooms WHERE campaign_id=?1 AND active=1)",
        )
        .bind(campaign.to_string())
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| error("agent_recovery_read_failed"))?;
        if room {
            return Ok(None);
        }
        let row = sqlx::query("SELECT request_id,request_json,status,owner_id,restore_generation,actor_id FROM agent_turn_operations WHERE campaign_id=?1 AND session_id=?2 ORDER BY updated_at DESC,created_at DESC,rowid DESC LIMIT 1")
            .bind(campaign.to_string()).bind(session.to_string()).fetch_optional(&mut *tx).await.map_err(|_| error("agent_recovery_read_failed"))?;
        let Some(row) = row else { return Ok(None) };
        let status: String = row.get("status");
        if !matches!(status.as_str(), "running" | "interrupted") {
            return Ok(None);
        }
        let generation: i64 =
            sqlx::query_scalar("SELECT restore_generation FROM sessions WHERE id=?1")
                .bind(session.to_string())
                .fetch_one(&mut *tx)
                .await
                .map_err(|_| error("agent_recovery_read_failed"))?;
        if generation != row.get::<i64, _>("restore_generation") {
            return Ok(None);
        }
        if row.get::<Option<String>, _>("actor_id").as_deref()
            != Some(actor_id.to_string().as_str())
        {
            return Ok(None);
        }
        let input: AgentOperation = serde_json::from_str(&row.get::<String, _>("request_json"))
            .map_err(|_| error("agent_recovery_data_invalid"))?;
        if input.request.campaign_id != campaign
            || input.request.session_id != session
            || input.request_id.to_string() != row.get::<String, _>("request_id")
        {
            return Err(error("agent_recovery_scope_invalid"));
        }
        let scope = CommandScope {
            request_id: input.request_id,
            campaign_id: campaign,
            session_id: session,
            actor_id,
            authority: CommandAuthority::Solo {
                restore_generation: u64::try_from(generation)
                    .map_err(|_| error("agent_recovery_data_invalid"))?,
            },
        };
        let ledger =
            crate::human_commands::read_ledger(&mut tx, &scope, &input.request.player_message)
                .await?;
        let running =
            status == "running" && row.get::<String, _>("owner_id") == process_owner.to_string();
        let projection = project_recovery(scope, input.model, running, ledger.as_ref())
            .map_err(|_| error("agent_recovery_data_invalid"))?;
        tx.commit()
            .await
            .map_err(|_| error("agent_recovery_read_failed"))?;
        tracing::debug!(request_id=%projection.scope.request_id, progress_known=projection.progress_known, effects=projection.confirmed_effects.len(), "agent.recovery.solo_ready");
        Ok(Some(projection))
    }

    pub async fn room_agent_recovery(
        &self,
        lease: &RoomLease,
        process_owner: Uuid,
    ) -> Result<Option<AgentRecoveryProjection>, RepositoryError> {
        tracing::debug!(room_id=%lease.room_id(), member_id=%lease.member_id(), "agent.recovery.read_room");
        let mut tx = self
            .pool()
            .begin()
            .await
            .map_err(|_| error("agent_recovery_read_failed"))?;
        crate::room_agent::guard(&mut tx, lease).await?;
        let row = sqlx::query("SELECT turn_id,request_json,status,process_owner,room_epoch,reset_cursor,actor_id FROM room_dm_turns WHERE room_id=?1 AND member_id=?2 ORDER BY rowid DESC LIMIT 1")
            .bind(lease.room_id().to_string()).bind(lease.member_id().to_string()).fetch_optional(&mut *tx).await.map_err(|_| error("agent_recovery_read_failed"))?;
        let Some(row) = row else { return Ok(None) };
        let status: String = row.get("status");
        if !matches!(status.as_str(), "running" | "interrupted") {
            return Ok(None);
        }
        let reset: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(sequence),0) FROM room_changes WHERE room_id=?1 AND kind='timeline_reset'")
            .bind(lease.room_id().to_string()).fetch_one(&mut *tx).await.map_err(|_| error("agent_recovery_read_failed"))?;
        if row.get::<i64, _>("reset_cursor") != reset
            || u64::try_from(row.get::<i64, _>("room_epoch")).ok() != Some(lease.epoch())
        {
            return Ok(None);
        }
        let input: AgentOperation = serde_json::from_str(&row.get::<String, _>("request_json"))
            .map_err(|_| error("agent_recovery_data_invalid"))?;
        if input.request_id.to_string() != row.get::<String, _>("turn_id")
            || input.request.campaign_id != lease.campaign_id().0
            || input.request.session_id != lease.session_id().0
        {
            return Err(error("agent_recovery_scope_invalid"));
        }
        let (_, _, actor_id, _) = crate::room_world::context(&mut tx, lease).await?;
        if row.get::<Option<String>, _>("actor_id").as_deref()
            != Some(actor_id.to_string().as_str())
        {
            return Ok(None);
        }
        let scope = CommandScope {
            request_id: input.request_id,
            campaign_id: lease.campaign_id().0,
            session_id: lease.session_id().0,
            actor_id,
            authority: CommandAuthority::Room {
                room_id: lease.room_id(),
                member_id: lease.member_id(),
                epoch: lease.epoch(),
                reset_cursor: u64::try_from(reset)
                    .map_err(|_| error("agent_recovery_data_invalid"))?,
            },
        };
        let journal = crate::room_human_commands::RoomHumanCommandLedger::new(
            self.clone(),
            lease.clone(),
            input.request_id,
            process_owner,
        );
        let ledger = journal
            .load(&mut tx, &scope, &input.request.player_message)
            .await?;
        let running = status == "running"
            && row.get::<String, _>("process_owner") == process_owner.to_string();
        let projection = project_recovery(scope, input.model, running, ledger.as_ref())
            .map_err(|_| error("agent_recovery_data_invalid"))?;
        tx.commit()
            .await
            .map_err(|_| error("agent_recovery_read_failed"))?;
        tracing::debug!(request_id=%projection.scope.request_id, progress_known=projection.progress_known, effects=projection.confirmed_effects.len(), "agent.recovery.room_ready");
        Ok(Some(projection))
    }
}
