use super::CampaignError;
use crate::models::adventure::AdventureCommand;
use crate::models::campaign_state::CampaignSessionState;
use crate::ports::repositories::AdventureRepository;
use std::sync::Arc;

pub struct ActInAdventure<R>(Arc<R>);
impl<R: AdventureRepository> ActInAdventure<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self(repository)
    }
    pub async fn execute(
        &self,
        command: AdventureCommand,
    ) -> Result<CampaignSessionState, CampaignError> {
        if command.character_id.is_nil()
            || command.causation_id.trim().is_empty()
            || command.causation_id.len() > 256
        {
            return Err(CampaignError::InvalidInput {
                code: "adventure_command_invalid",
            });
        }
        self.0.act(command).await.map_err(super::state::failure)
    }
}
