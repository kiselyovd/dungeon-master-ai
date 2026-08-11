use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

use crate::models::campaign::{
    CampaignSave, JournalEntry, NpcRecord, SceneRecord, SrdReference, StoredMessage,
};
use crate::models::chat::ChatMessage;
use crate::models::combat::CombatProjection;

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
pub trait CombatRepository: Send + Sync {
    async fn create(
        &self,
        session_id: Uuid,
        projection: CombatProjection,
    ) -> Result<(), RepositoryError>;

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
