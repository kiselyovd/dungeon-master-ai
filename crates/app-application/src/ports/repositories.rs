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
