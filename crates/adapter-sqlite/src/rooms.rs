use app_application::{
    models::{identity::CampaignId, party::PartyRole, room::*},
    ports::repositories::{RepositoryError, RoomRepository},
};
use sqlx::{Row, Sqlite, Transaction};
use uuid::Uuid;
fn error(code: &'static str) -> RepositoryError {
    RepositoryError::Operation {
        operation: "room",
        code,
    }
}
fn decode(raw: String) -> Result<RoomRecord, RepositoryError> {
    let room: RoomRecord = serde_json::from_str(&raw).map_err(|_| error("room_record_invalid"))?;
    if !room.valid() {
        return Err(error("room_record_invalid"));
    }
    Ok(room)
}
pub(crate) async fn read(
    tx: &mut Transaction<'_, Sqlite>,
    id: Uuid,
) -> Result<RoomRecord, RepositoryError> {
    let raw: Option<String> =
        sqlx::query_scalar("SELECT record_json FROM coop_rooms WHERE room_id=?1")
            .bind(id.to_string())
            .fetch_optional(&mut **tx)
            .await
            .map_err(|_| error("room_read_failed"))?;
    decode(raw.ok_or(RepositoryError::NotFound)?)
}
async fn write(tx: &mut Transaction<'_, Sqlite>, room: &RoomRecord) -> Result<(), RepositoryError> {
    if !room.valid() {
        return Err(error("room_record_invalid"));
    }
    let updated = sqlx::query("UPDATE coop_rooms SET active=?2,record_json=?3 WHERE room_id=?1")
        .bind(room.room_id.to_string())
        .bind(room.active)
        .bind(serde_json::to_string(room).map_err(|_| error("room_record_invalid"))?)
        .execute(&mut **tx)
        .await
        .map_err(|_| error("room_write_failed"))?;
    if updated.rows_affected() != 1 {
        return Err(error("room_write_failed"));
    }
    Ok(())
}
async fn creation(
    tx: &mut Transaction<'_, Sqlite>,
    campaign: CampaignId,
    id: &str,
    signature: &str,
) -> Result<Option<RoomRecord>, RepositoryError> {
    let row=sqlx::query("SELECT room_id,signature FROM room_creation_receipts WHERE campaign_id=?1 AND causation_id=?2").bind(campaign.0.to_string()).bind(id).fetch_optional(&mut **tx).await.map_err(|_|error("room_receipt_failed"))?;
    let Some(row) = row else { return Ok(None) };
    if row.get::<String, _>("signature") != signature {
        return Err(error("room_causation_mismatch"));
    }
    let id = Uuid::parse_str(&row.get::<String, _>("room_id"))
        .map_err(|_| error("room_record_invalid"))?;
    Ok(Some(read(tx, id).await?))
}
async fn join_receipt(
    tx: &mut Transaction<'_, Sqlite>,
    room: Uuid,
    id: Uuid,
) -> Result<Option<RoomJoinRecord>, RepositoryError> {
    let raw: Option<String> = sqlx::query_scalar(
        "SELECT record_json FROM room_join_receipts WHERE room_id=?1 AND join_id=?2",
    )
    .bind(room.to_string())
    .bind(id.to_string())
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| error("room_receipt_failed"))?;
    raw.map(|raw| serde_json::from_str(&raw).map_err(|_| error("room_record_invalid")))
        .transpose()
}
pub(crate) fn verify_lease(room: &RoomRecord, lease: &RoomLease) -> Result<(), RepositoryError> {
    if !room.active
        || room.room_id != lease.room_id()
        || room.campaign_id != lease.campaign_id()
        || room.session_id != lease.session_id()
        || room.epoch != lease.epoch()
        || !room.members.iter().any(|member| {
            member.member_id == lease.member_id() && member.active && member.role == lease.role()
        })
    {
        return Err(error("room_access_denied"));
    }
    Ok(())
}
async fn host(
    tx: &mut Transaction<'_, Sqlite>,
    lease: &RoomLease,
) -> Result<RoomRecord, RepositoryError> {
    let room = read(tx, lease.room_id()).await?;
    verify_lease(&room, lease)?;
    if lease.role() != PartyRole::Host {
        return Err(error("room_host_required"));
    }
    Ok(room)
}

#[async_trait::async_trait]
impl RoomRepository for crate::SqliteStore {
    async fn room_dm_request(
        &self,
        lease: &RoomLease,
        turn: Uuid,
    ) -> Result<Option<app_application::agent::operations::AgentOperation>, RepositoryError> {
        crate::room_dm_turns::request(self, lease, turn).await
    }
    async fn claim_room_dm_turn(
        &self,
        lease: &RoomLease,
        input: app_application::agent::operations::AgentOperation,
        owner: Uuid,
        resume: bool,
    ) -> Result<app_application::agent::operations::OperationClaim, RepositoryError> {
        crate::room_dm_turns::claim(self, lease, input, owner, resume).await
    }
    async fn append_room_dm_event(
        &self,
        lease: &RoomLease,
        turn: Uuid,
        owner: Uuid,
        sequence: u64,
        event: app_application::models::agent::AgentEvent,
    ) -> Result<(), RepositoryError> {
        crate::room_dm_turns::append_event(self, lease, turn, owner, sequence, event).await
    }
    async fn finish_room_dm_turn(
        &self,
        lease: &RoomLease,
        turn: Uuid,
        owner: Uuid,
        complete: bool,
    ) -> Result<(), RepositoryError> {
        crate::room_dm_turns::finish(self, lease, turn, owner, complete).await
    }

    async fn room_dm_party(
        &self,
        lease: &RoomLease,
    ) -> Result<app_application::models::party::PartyState, RepositoryError> {
        crate::room_agent::dm_party(self, lease).await
    }
    async fn append_room_agent_message(
        &self,
        lease: &RoomLease,
        turn: Uuid,
        message: app_application::models::chat::ChatMessage,
    ) -> Result<app_application::models::campaign::StoredMessage, RepositoryError> {
        crate::room_agent::append(self, lease, turn, message).await
    }
    async fn room_agent_checkpoint(
        &self,
        lease: &RoomLease,
        turn: Uuid,
    ) -> Result<Vec<app_application::models::campaign::StoredMessage>, RepositoryError> {
        crate::room_agent::load_checkpoint(self, lease, turn).await
    }
    async fn prepare_room_agent_tool(
        &self,
        lease: &RoomLease,
        turn_id: Uuid,
        tool_id: &str,
        command: app_application::agent::commands::AgentToolCommand,
    ) -> Result<app_application::models::room_agent::PreparedRoomTool, RepositoryError> {
        crate::room_agent::prepare(self, lease, turn_id, tool_id, command).await
    }
    async fn resolve_room_skill_check(
        &self,
        lease: &RoomLease,
        turn_id: Uuid,
        tool_id: &str,
        command: app_application::agent::commands::RollDiceCommand,
        rng_seed: u64,
    ) -> Result<serde_json::Value, RepositoryError> {
        crate::room_checks::resolve(self, lease, turn_id, tool_id, command, rng_seed).await
    }
    async fn act_room_adventure(
        &self,
        lease: &RoomLease,
        command: app_application::models::room_world::RoomAdventureCommand,
    ) -> Result<app_application::models::room_world::RoomWorldReceipt, RepositoryError> {
        crate::room_world::adventure(self, lease, command).await
    }
    async fn recover_room_hero(
        &self,
        lease: &RoomLease,
        command: app_application::models::room_recovery::RoomRecoveryCommand,
        rng_seed: u64,
    ) -> Result<app_application::models::room_recovery::RoomRecoveryReceipt, RepositoryError> {
        crate::room_recovery::recover(self, lease, command, rng_seed).await
    }
    async fn room_rest_status(
        &self,
        lease: &RoomLease,
    ) -> Result<app_application::models::rest::RestStatus, RepositoryError> {
        crate::room_world::status(self, lease).await
    }
    async fn rest_room(
        &self,
        lease: &RoomLease,
        command: app_application::models::room_world::RoomRestCommand,
        rng_seed: u64,
    ) -> Result<app_application::models::room_world::RoomWorldReceipt, RepositoryError> {
        crate::room_world::rest(self, lease, command, rng_seed).await
    }
    async fn room_updates(
        &self,
        lease: &RoomLease,
        after: Option<u64>,
        force_snapshot: bool,
    ) -> Result<Option<app_application::models::room_state::RoomUpdate>, RepositoryError> {
        crate::room_state::updates(self, lease, after, force_snapshot).await
    }
    async fn room_combat(
        &self,
        lease: &RoomLease,
        encounter: Uuid,
    ) -> Result<app_application::models::combat::CombatProjection, RepositoryError> {
        crate::room_combat::read(self, lease, encounter).await
    }
    async fn act_room_combat(
        &self,
        lease: &RoomLease,
        encounter: Uuid,
        command: app_application::models::room_combat::RoomCombatCommand,
        rng_seed: u64,
    ) -> Result<app_application::models::room_combat::RoomCombatResult, RepositoryError> {
        crate::room_combat::act(self, lease, encounter, command, rng_seed).await
    }
    async fn submit_character_proposal(
        &self,
        lease: &RoomLease,
        command: app_application::models::room_character::SubmitRoomCharacter,
    ) -> Result<app_application::models::room_character::ProposalSummary, RepositoryError> {
        crate::room_characters::submit(self, lease, command).await
    }
    async fn own_character_proposal(
        &self,
        lease: &RoomLease,
    ) -> Result<
        Option<app_application::models::room_character::OwnCharacterProposal>,
        RepositoryError,
    > {
        crate::room_characters::own(self, lease).await
    }
    async fn pending_character_proposals(
        &self,
        lease: &RoomLease,
    ) -> Result<Vec<app_application::models::room_character::ProposalSummary>, RepositoryError>
    {
        crate::room_characters::pending(self, lease).await
    }
    async fn approve_character_proposal(
        &self,
        lease: &RoomLease,
        command: app_application::models::room_character::ApproveRoomCharacter,
    ) -> Result<app_application::models::room_character::CharacterApproval, RepositoryError> {
        crate::room_characters::approve(self, lease, command).await
    }
    async fn room(&self, id: Uuid) -> Result<Option<RoomRecord>, RepositoryError> {
        let raw: Option<String> =
            sqlx::query_scalar("SELECT record_json FROM coop_rooms WHERE room_id=?1")
                .bind(id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(|_| error("room_read_failed"))?;
        raw.map(decode).transpose()
    }
    async fn find_creation(
        &self,
        campaign: CampaignId,
        id: &str,
        signature: &str,
    ) -> Result<Option<RoomRecord>, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| error("room_begin_failed"))?;
        let room = creation(&mut tx, campaign, id, signature).await?;
        tx.commit().await.map_err(|_| error("room_commit_failed"))?;
        Ok(room)
    }
    async fn create_room(
        &self,
        mut room: RoomRecord,
        id: &str,
        signature: &str,
    ) -> Result<RoomRecord, RepositoryError> {
        if !room.valid() || id.trim().is_empty() || id.len() > 200 {
            return Err(error("room_record_invalid"));
        }
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| error("room_begin_failed"))?;
        if let Some(existing) = creation(&mut tx, room.campaign_id, id, signature).await? {
            tx.commit().await.map_err(|_| error("room_commit_failed"))?;
            return Ok(existing);
        }
        let current = crate::campaign_state::load_in_transaction(&mut tx, room.campaign_id).await?;
        let leader: Option<String> =
            sqlx::query_scalar("SELECT host_member_id FROM campaign_parties WHERE campaign_id=?1")
                .bind(room.campaign_id.0.to_string())
                .fetch_optional(&mut *tx)
                .await
                .map_err(|_| error("room_read_failed"))?;
        if current.session_id != room.session_id
            || leader.as_deref() != Some(room.host_member_id.to_string().as_str())
        {
            return Err(error("room_identity_mismatch"));
        }
        let active:Option<String>=sqlx::query_scalar("SELECT record_json FROM coop_rooms WHERE campaign_id=?1 AND session_id=?2 AND active=1").bind(room.campaign_id.0.to_string()).bind(room.session_id.0.to_string()).fetch_optional(&mut *tx).await.map_err(|_|error("room_read_failed"))?;
        if let Some(active) = active {
            room = decode(active)?;
        } else {
            sqlx::query("INSERT INTO coop_rooms(room_id,campaign_id,session_id,active,record_json) VALUES(?1,?2,?3,?4,?5)").bind(room.room_id.to_string()).bind(room.campaign_id.0.to_string()).bind(room.session_id.0.to_string()).bind(room.active).bind(serde_json::to_string(&room).map_err(|_|error("room_record_invalid"))?).execute(&mut *tx).await.map_err(|_|error("room_write_failed"))?;
        }
        sqlx::query("INSERT INTO room_creation_receipts(campaign_id,causation_id,room_id,signature) VALUES(?1,?2,?3,?4)").bind(room.campaign_id.0.to_string()).bind(id).bind(room.room_id.to_string()).bind(signature).execute(&mut *tx).await.map_err(|_|error("room_receipt_failed"))?;
        tx.commit().await.map_err(|_| error("room_commit_failed"))?;
        Ok(room)
    }
    async fn find_join(
        &self,
        room: Uuid,
        id: Uuid,
    ) -> Result<Option<RoomJoinRecord>, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| error("room_begin_failed"))?;
        let record = join_receipt(&mut tx, room, id).await?;
        tx.commit().await.map_err(|_| error("room_commit_failed"))?;
        Ok(record)
    }
    async fn commit_join(
        &self,
        command: CommitRoomJoin,
    ) -> Result<RoomJoinRecord, RepositoryError> {
        let record = &command.record;
        if record.join_id.is_nil()
            || record.member_id.is_nil()
            || record.credential_id.is_nil()
            || !valid_display_name(&record.display_name)
        {
            return Err(error("room_join_invalid"));
        }
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| error("room_begin_failed"))?;
        if let Some(old) = join_receipt(&mut tx, record.room_id, record.join_id).await? {
            if old != *record {
                return Err(error("room_causation_mismatch"));
            }
            tx.commit().await.map_err(|_| error("room_commit_failed"))?;
            return Ok(old);
        }
        let mut room = read(&mut tx, record.room_id).await?;
        if !room.active
            || room.epoch != command.expected_epoch
            || room.invitation.invitation_id != command.invitation_id
            || room.invitation.consumed_by.is_some()
            || command.now >= room.invitation.expires_at
        {
            return Err(error("room_invitation_invalid"));
        }
        if room
            .members
            .iter()
            .any(|member| member.credential_id == record.credential_id)
        {
            return Err(error("room_full_or_duplicate"));
        }
        if let Some(target) = room.invitation.target_member_id {
            if target != record.member_id {
                return Err(error("room_invitation_invalid"));
            }
            let member = room
                .members
                .iter_mut()
                .find(|member| member.member_id == target && member.role == PartyRole::Player)
                .ok_or_else(|| error("room_member_invalid"))?;
            member.credential_id = record.credential_id;
            member.display_name = record.display_name.clone();
            member.active = true;
            room.epoch = room
                .epoch
                .checked_add(1)
                .ok_or_else(|| error("room_epoch_overflow"))?;
        } else {
            if room.members.len() >= app_application::models::party::MAX_PARTY_MEMBERS
                || room
                    .members
                    .iter()
                    .any(|member| member.member_id == record.member_id)
            {
                return Err(error("room_full_or_duplicate"));
            }
            room.members.push(RoomMember {
                member_id: record.member_id,
                display_name: record.display_name.clone(),
                credential_id: record.credential_id,
                role: PartyRole::Player,
                active: true,
            });
        }
        room.invitation.consumed_by = Some(record.join_id);
        write(&mut tx, &room).await?;
        sqlx::query("INSERT INTO room_join_receipts(room_id,join_id,record_json) VALUES(?1,?2,?3)")
            .bind(record.room_id.to_string())
            .bind(record.join_id.to_string())
            .bind(serde_json::to_string(record).map_err(|_| error("room_record_invalid"))?)
            .execute(&mut *tx)
            .await
            .map_err(|_| error("room_receipt_failed"))?;
        tx.commit().await.map_err(|_| error("room_commit_failed"))?;
        Ok(command.record)
    }
    async fn close_room(&self, lease: &RoomLease) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| error("room_begin_failed"))?;
        let mut room = host(&mut tx, lease).await?;
        room.active = false;
        room.epoch = room
            .epoch
            .checked_add(1)
            .ok_or_else(|| error("room_epoch_overflow"))?;
        write(&mut tx, &room).await?;
        crate::combat_binding::reconcile_membership(&mut tx, room.campaign_id, room.session_id)
            .await?;
        tx.commit().await.map_err(|_| error("room_commit_failed"))?;
        Ok(())
    }
    async fn revoke_member(&self, lease: &RoomLease, member: Uuid) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| error("room_begin_failed"))?;
        let mut room = host(&mut tx, lease).await?;
        let target = room
            .members
            .iter_mut()
            .find(|target| {
                target.member_id == member && target.role == PartyRole::Player && target.active
            })
            .ok_or_else(|| error("room_member_invalid"))?;
        target.active = false;
        room.epoch = room
            .epoch
            .checked_add(1)
            .ok_or_else(|| error("room_epoch_overflow"))?;
        write(&mut tx, &room).await?;
        crate::combat_binding::reconcile_membership(&mut tx, room.campaign_id, room.session_id)
            .await?;
        tx.commit().await.map_err(|_| error("room_commit_failed"))?;
        Ok(())
    }
    async fn rotate_invitation(
        &self,
        lease: &RoomLease,
        invitation: RoomInvitation,
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| error("room_begin_failed"))?;
        let mut room = host(&mut tx, lease).await?;
        room.invitation = invitation;
        write(&mut tx, &room).await?;
        tx.commit().await.map_err(|_| error("room_commit_failed"))?;
        Ok(())
    }
}
