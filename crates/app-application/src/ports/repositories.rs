use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

use crate::models::campaign::{
    CampaignSave, JournalEntry, NpcRecord, SceneRecord, SrdReference, StoredMessage,
};
use crate::models::chat::ChatMessage;
use crate::models::combat::CombatProjection;
use crate::models::identity::{CampaignId, Revision, SessionId};
use crate::models::session::{
    CampaignDetail, CampaignEvent, CampaignSession, CampaignSummary, InitialCampaign,
};
use crate::models::snapshot::{
    CampaignSnapshot, CreateSnapshotRecord, RestoreSnapshotRecord, SnapshotCreated,
    SnapshotSaveDetails, SnapshotSaveSummary,
};

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("{operation} failed with code {code}")]
    Operation {
        operation: &'static str,
        code: &'static str,
    },
    #[error("record not found")]
    NotFound,
    #[error("revision conflict: expected {expected}, actual {actual}")]
    RevisionConflict { expected: u64, actual: u64 },
    #[error("campaign revision conflict: expected {expected}, actual {actual}")]
    Conflict {
        expected: Revision,
        actual: Revision,
    },
}

#[async_trait]
pub trait CampaignRepository: Send + Sync {
    /// Atomically persists the campaign, initial session, and first event.
    async fn create(&self, campaign: InitialCampaign) -> Result<CampaignDetail, RepositoryError>;
    async fn list(&self) -> Result<Vec<CampaignSummary>, RepositoryError>;
    async fn get(&self, campaign_id: CampaignId)
        -> Result<Option<CampaignDetail>, RepositoryError>;
}

#[async_trait]
pub trait CampaignStateRepository: Send + Sync {
    async fn load_current(
        &self,
        campaign_id: CampaignId,
    ) -> Result<crate::models::campaign_state::CampaignSessionState, RepositoryError>;
    async fn commit_character(
        &self,
        command: crate::models::campaign_state::CommitCampaignCharacter,
    ) -> Result<crate::models::campaign_state::CampaignSessionState, RepositoryError>;
}

#[async_trait]
pub trait RestRepository: Send + Sync {
    async fn status(
        &self,
        campaign_id: CampaignId,
    ) -> Result<crate::models::rest::RestStatus, RepositoryError>;
    async fn rest(
        &self,
        command: crate::models::rest::RestCommand,
        rng_seed: u64,
    ) -> Result<crate::models::rest::RestResult, RepositoryError>;
}

#[async_trait]
pub trait PartyRepository: Send + Sync {
    async fn initialize_party(
        &self,
        command: crate::models::party::InitializeParty,
    ) -> Result<crate::models::party::PartyState, RepositoryError>;
    async fn party(
        &self,
        campaign_id: CampaignId,
    ) -> Result<Option<crate::models::party::PartyState>, RepositoryError>;
    async fn enroll_party_character(
        &self,
        command: crate::models::party::EnrollPartyCharacter,
    ) -> Result<crate::models::party::PartyState, RepositoryError>;
}

#[async_trait]
pub trait RoomRepository: Send + Sync {
    async fn room_dm_request(
        &self,
        lease: &crate::models::room::RoomLease,
        turn: Uuid,
    ) -> Result<Option<crate::agent::operations::AgentOperation>, RepositoryError>;
    async fn claim_room_dm_turn(
        &self,
        lease: &crate::models::room::RoomLease,
        input: crate::agent::operations::AgentOperation,
        owner: Uuid,
        resume: bool,
    ) -> Result<crate::agent::operations::OperationClaim, RepositoryError>;
    async fn append_room_dm_event(
        &self,
        lease: &crate::models::room::RoomLease,
        turn: Uuid,
        owner: Uuid,
        sequence: u64,
        event: crate::models::agent::AgentEvent,
    ) -> Result<(), RepositoryError>;
    async fn finish_room_dm_turn(
        &self,
        lease: &crate::models::room::RoomLease,
        turn: Uuid,
        owner: Uuid,
        complete: bool,
    ) -> Result<(), RepositoryError>;

    /// Trusted Dungeon Master context only; never return this through a player endpoint.
    async fn room_dm_party(
        &self,
        lease: &crate::models::room::RoomLease,
    ) -> Result<crate::models::party::PartyState, RepositoryError>;
    async fn append_room_agent_message(
        &self,
        lease: &crate::models::room::RoomLease,
        turn_id: Uuid,
        message: crate::models::chat::ChatMessage,
    ) -> Result<crate::models::campaign::StoredMessage, RepositoryError>;
    async fn room_agent_checkpoint(
        &self,
        lease: &crate::models::room::RoomLease,
        turn_id: Uuid,
    ) -> Result<Vec<crate::models::campaign::StoredMessage>, RepositoryError>;
    async fn prepare_room_agent_tool(
        &self,
        lease: &crate::models::room::RoomLease,
        turn_id: Uuid,
        tool_id: &str,
        command: crate::agent::commands::AgentToolCommand,
    ) -> Result<crate::models::room_agent::PreparedRoomTool, RepositoryError>;
    async fn act_room_adventure(
        &self,
        lease: &crate::models::room::RoomLease,
        command: crate::models::room_world::RoomAdventureCommand,
    ) -> Result<crate::models::room_world::RoomWorldReceipt, RepositoryError>;
    async fn recover_room_hero(
        &self,
        lease: &crate::models::room::RoomLease,
        command: crate::models::room_recovery::RoomRecoveryCommand,
        rng_seed: u64,
    ) -> Result<crate::models::room_recovery::RoomRecoveryReceipt, RepositoryError>;
    async fn room_rest_status(
        &self,
        lease: &crate::models::room::RoomLease,
    ) -> Result<crate::models::rest::RestStatus, RepositoryError>;
    async fn rest_room(
        &self,
        lease: &crate::models::room::RoomLease,
        command: crate::models::room_world::RoomRestCommand,
        rng_seed: u64,
    ) -> Result<crate::models::room_world::RoomWorldReceipt, RepositoryError>;
    async fn room_updates(
        &self,
        lease: &crate::models::room::RoomLease,
        after: Option<u64>,
        force_snapshot: bool,
    ) -> Result<Option<crate::models::room_state::RoomUpdate>, RepositoryError>;
    async fn room_combat(
        &self,
        lease: &crate::models::room::RoomLease,
        encounter: Uuid,
    ) -> Result<crate::models::combat::CombatProjection, RepositoryError>;
    async fn act_room_combat(
        &self,
        lease: &crate::models::room::RoomLease,
        encounter: Uuid,
        command: crate::models::room_combat::RoomCombatCommand,
        rng_seed: u64,
    ) -> Result<crate::models::room_combat::RoomCombatResult, RepositoryError>;
    async fn submit_character_proposal(
        &self,
        lease: &crate::models::room::RoomLease,
        command: crate::models::room_character::SubmitRoomCharacter,
    ) -> Result<crate::models::room_character::ProposalSummary, RepositoryError>;
    async fn own_character_proposal(
        &self,
        lease: &crate::models::room::RoomLease,
    ) -> Result<Option<crate::models::room_character::OwnCharacterProposal>, RepositoryError>;
    async fn pending_character_proposals(
        &self,
        lease: &crate::models::room::RoomLease,
    ) -> Result<Vec<crate::models::room_character::ProposalSummary>, RepositoryError>;
    async fn approve_character_proposal(
        &self,
        lease: &crate::models::room::RoomLease,
        command: crate::models::room_character::ApproveRoomCharacter,
    ) -> Result<crate::models::room_character::CharacterApproval, RepositoryError>;
    async fn room(
        &self,
        room_id: Uuid,
    ) -> Result<Option<crate::models::room::RoomRecord>, RepositoryError>;
    async fn find_creation(
        &self,
        campaign_id: CampaignId,
        id: &str,
        signature: &str,
    ) -> Result<Option<crate::models::room::RoomRecord>, RepositoryError>;
    async fn create_room(
        &self,
        room: crate::models::room::RoomRecord,
        id: &str,
        signature: &str,
    ) -> Result<crate::models::room::RoomRecord, RepositoryError>;
    async fn find_join(
        &self,
        room_id: Uuid,
        join_id: Uuid,
    ) -> Result<Option<crate::models::room::RoomJoinRecord>, RepositoryError>;
    async fn commit_join(
        &self,
        command: crate::models::room::CommitRoomJoin,
    ) -> Result<crate::models::room::RoomJoinRecord, RepositoryError>;
    async fn close_room(
        &self,
        lease: &crate::models::room::RoomLease,
    ) -> Result<(), RepositoryError>;
    async fn revoke_member(
        &self,
        lease: &crate::models::room::RoomLease,
        member_id: Uuid,
    ) -> Result<(), RepositoryError>;
    async fn rotate_invitation(
        &self,
        lease: &crate::models::room::RoomLease,
        invitation: crate::models::room::RoomInvitation,
    ) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait AdventureRepository: Send + Sync {
    async fn act_as_dm(
        &self,
        _intent: crate::models::adventure::AdventureIntent,
    ) -> Result<crate::models::campaign_state::CampaignSessionState, RepositoryError> {
        Err(RepositoryError::NotFound)
    }
    async fn act(
        &self,
        command: crate::models::adventure::AdventureCommand,
    ) -> Result<crate::models::campaign_state::CampaignSessionState, RepositoryError>;
}

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn get(&self, session_id: SessionId) -> Result<Option<CampaignSession>, RepositoryError>;
    async fn latest_for_campaign(
        &self,
        campaign_id: CampaignId,
    ) -> Result<Option<CampaignSession>, RepositoryError>;
}

#[async_trait]
pub trait EventRepository: Send + Sync {
    async fn append(
        &self,
        campaign_id: CampaignId,
        expected: Revision,
        events: Vec<CampaignEvent>,
    ) -> Result<Revision, RepositoryError>;
    async fn since(
        &self,
        campaign_id: CampaignId,
        after: Revision,
    ) -> Result<Vec<CampaignEvent>, RepositoryError>;
}

#[async_trait]
pub trait MessageRepository: Send + Sync {
    async fn append(
        &self,
        session_id: Uuid,
        message: ChatMessage,
    ) -> Result<StoredMessage, RepositoryError>;

    async fn list(&self, session_id: Uuid) -> Result<Vec<StoredMessage>, RepositoryError>;
}

#[async_trait]
pub trait SaveRepository: Send + Sync {
    async fn put(&self, save: CampaignSave) -> Result<(), RepositoryError>;
    async fn get(&self, save_id: Uuid) -> Result<Option<CampaignSave>, RepositoryError>;
}

#[async_trait]
pub trait SnapshotRepository: Send + Sync {
    /// Captures all server-owned projections and the supplied character inside one transaction.
    /// Repeating a causation id must return the first snapshot without creating a duplicate.
    async fn create_snapshot(
        &self,
        record: CreateSnapshotRecord,
    ) -> Result<SnapshotCreated, RepositoryError>;

    /// Validates identity and revision and switches the active snapshot in one transaction.
    async fn restore_snapshot(
        &self,
        record: RestoreSnapshotRecord,
    ) -> Result<CampaignSnapshot, RepositoryError>;

    async fn list_snapshots(
        &self,
        session_id: SessionId,
    ) -> Result<Vec<SnapshotSaveSummary>, RepositoryError>;

    async fn get_snapshot(
        &self,
        save_id: Uuid,
    ) -> Result<Option<SnapshotSaveDetails>, RepositoryError>;

    async fn delete_snapshot(&self, save_id: Uuid) -> Result<bool, RepositoryError>;

    async fn update_snapshot_metadata(
        &self,
        save_id: Uuid,
        kind: String,
        title: String,
        summary: String,
        tag: String,
    ) -> Result<bool, RepositoryError>;
}

#[async_trait]
pub trait CombatRepository: Send + Sync {
    async fn find_action(
        &self,
        encounter_id: Uuid,
        request_id: &str,
        signature: &str,
    ) -> Result<Option<CombatProjection>, RepositoryError>;

    /// Commit the projection and its immutable request/result receipt in the
    /// same transaction. A simultaneous exact duplicate returns that receipt.
    async fn commit_action(
        &self,
        expected_revision: u64,
        projection: CombatProjection,
        request_id: &str,
        signature: &str,
    ) -> Result<crate::models::combat::CombatActionCommit, RepositoryError>;
    async fn create(
        &self,
        session_id: Uuid,
        projection: CombatProjection,
    ) -> Result<CombatProjection, RepositoryError>;

    async fn get(&self, encounter_id: Uuid) -> Result<Option<CombatProjection>, RepositoryError>;

    async fn compare_and_set(
        &self,
        expected_revision: u64,
        projection: CombatProjection,
    ) -> Result<(), RepositoryError>;

    async fn end(&self, encounter_id: Uuid) -> Result<Option<CombatProjection>, RepositoryError>;
}

#[async_trait]
pub trait JournalRepository: Send + Sync {
    async fn list(&self, campaign_id: Uuid) -> Result<Vec<JournalEntry>, RepositoryError>;
    async fn put(&self, entry: JournalEntry) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait NpcRepository: Send + Sync {
    async fn list(&self, campaign_id: Uuid) -> Result<Vec<NpcRecord>, RepositoryError>;
    async fn put(&self, npc: NpcRecord) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait SceneRepository: Send + Sync {
    async fn current(&self, campaign_id: Uuid) -> Result<Option<SceneRecord>, RepositoryError>;
    async fn set_current(&self, scene: SceneRecord) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait SrdRepository: Send + Sync {
    async fn search(&self, query: &str, limit: usize)
        -> Result<Vec<SrdReference>, RepositoryError>;
}
