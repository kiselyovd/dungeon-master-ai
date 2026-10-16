use std::sync::Arc;

use serde_json::Value;
use uuid::Uuid;

use crate::models::identity::{CampaignId, Revision, SessionId};
use crate::models::snapshot::{
    CampaignSnapshot, CreateSnapshotRecord, RestoreSnapshotRecord, SnapshotCreated,
    SNAPSHOT_SCHEMA_VERSION,
};
use crate::ports::repositories::{RepositoryError, SnapshotRepository};

use super::{CampaignError, RULESET_ID};

#[derive(Debug, Clone)]
pub struct CreateSnapshotCommand {
    pub id: Uuid,
    pub campaign_id: CampaignId,
    pub session_id: SessionId,
    pub expected_revision: Revision,
    pub title: String,
    pub kind: String,
    pub summary: String,
    pub tag: String,
    pub causation_id: String,
    pub character: Value,
}

pub struct CreateSnapshot<R> {
    repository: Arc<R>,
}

impl<R> CreateSnapshot<R>
where
    R: SnapshotRepository,
{
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        command: CreateSnapshotCommand,
    ) -> Result<SnapshotCreated, CampaignError> {
        if command.title.trim().is_empty()
            || command.causation_id.trim().is_empty()
            || !command.character.is_object()
        {
            return Err(CampaignError::InvalidInput {
                code: "snapshot_invalid",
            });
        }
        let record = CreateSnapshotRecord {
            id: command.id,
            campaign_id: command.campaign_id,
            session_id: command.session_id,
            expected_revision: command.expected_revision,
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            ruleset_id: RULESET_ID.to_string(),
            title: command.title,
            kind: command.kind,
            summary: command.summary,
            tag: command.tag,
            causation_id: command.causation_id,
            character: command.character,
        };
        let created = self
            .repository
            .create_snapshot(record)
            .await
            .map_err(map_repository_error)?;
        validate_snapshot(&created.snapshot, command.campaign_id, command.session_id)?;
        Ok(created)
    }
}

#[derive(Debug, Clone)]
pub struct RestoreSnapshotCommand {
    pub save_id: Uuid,
    pub campaign_id: CampaignId,
    pub session_id: SessionId,
    pub expected_revision: Revision,
    pub causation_id: String,
}

pub struct RestoreSnapshot<R> {
    repository: Arc<R>,
}

impl<R> RestoreSnapshot<R>
where
    R: SnapshotRepository,
{
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        command: RestoreSnapshotCommand,
    ) -> Result<CampaignSnapshot, CampaignError> {
        if command.causation_id.trim().is_empty() {
            return Err(CampaignError::InvalidInput {
                code: "snapshot_causation_id_required",
            });
        }
        let campaign_id = command.campaign_id;
        let session_id = command.session_id;
        let snapshot = self
            .repository
            .restore_snapshot(RestoreSnapshotRecord {
                save_id: command.save_id,
                campaign_id,
                session_id,
                expected_revision: command.expected_revision,
                causation_id: command.causation_id,
            })
            .await
            .map_err(map_repository_error)?;
        validate_snapshot(&snapshot, campaign_id, session_id)?;
        Ok(snapshot)
    }
}

fn validate_snapshot(
    snapshot: &CampaignSnapshot,
    campaign_id: CampaignId,
    session_id: SessionId,
) -> Result<(), CampaignError> {
    let valid = snapshot.campaign_id == campaign_id
        && snapshot.session_id == session_id
        && snapshot.schema_version == SNAPSHOT_SCHEMA_VERSION
        && snapshot.ruleset_id == RULESET_ID
        && snapshot.projection.is_complete();
    if valid {
        Ok(())
    } else {
        Err(CampaignError::InvalidInput {
            code: "snapshot_incompatible",
        })
    }
}

fn map_repository_error(error: RepositoryError) -> CampaignError {
    let code = match error {
        RepositoryError::NotFound => "snapshot_not_found",
        RepositoryError::RevisionConflict { .. } | RepositoryError::Conflict { .. } => {
            "snapshot_revision_conflict"
        }
        RepositoryError::Operation { code, .. } => code,
    };
    CampaignError::Persistence { code }
}
