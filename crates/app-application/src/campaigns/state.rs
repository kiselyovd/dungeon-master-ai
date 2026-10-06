use super::{CampaignError, RULESET_ID};
use crate::models::campaign_state::{
    valid_character_document, within_character_budget, CampaignSessionState,
    CommitCampaignCharacter,
};
use crate::models::identity::CampaignId;
use crate::ports::repositories::{CampaignStateRepository, RepositoryError};
use std::sync::Arc;

pub struct LoadCampaignSession<R>(Arc<R>);
pub struct CommitCharacter<R>(Arc<R>);

pub(super) fn failure(error: RepositoryError) -> CampaignError {
    let code = match error {
        RepositoryError::NotFound => "campaign_not_found",
        RepositoryError::Conflict { .. } | RepositoryError::RevisionConflict { .. } => {
            "campaign_revision_conflict"
        }
        RepositoryError::Operation { code, .. } => code,
    };
    CampaignError::Persistence { code }
}

fn validate(
    state: CampaignSessionState,
    id: CampaignId,
) -> Result<CampaignSessionState, CampaignError> {
    if state.campaign.id != id
        || state.session_id != state.campaign.last_session_id
        || state.campaign.schema_version != 1
        || state.campaign.ruleset_id != RULESET_ID
        || !state.projection.is_complete()
        || (state.character_id.is_some() && !valid_character_document(&state.projection.character))
    {
        return Err(CampaignError::InvalidInput {
            code: "campaign_state_invalid",
        });
    }
    Ok(state)
}

impl<R: CampaignStateRepository> LoadCampaignSession<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self(repository)
    }
    pub async fn execute(&self, id: CampaignId) -> Result<CampaignSessionState, CampaignError> {
        validate(self.0.load_current(id).await.map_err(failure)?, id)
    }
}

impl<R: CampaignStateRepository> CommitCharacter<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self(repository)
    }
    pub async fn execute(
        &self,
        command: CommitCampaignCharacter,
    ) -> Result<CampaignSessionState, CampaignError> {
        if command.character_id.is_nil()
            || command.causation_id.trim().is_empty()
            || command.causation_id.len() > 256
            || !valid_character_document(&command.character)
            || !within_character_budget(&command.character)
        {
            return Err(CampaignError::InvalidInput {
                code: "character_invalid",
            });
        }
        let id = command.campaign_id;
        validate(self.0.commit_character(command).await.map_err(failure)?, id)
    }
}
