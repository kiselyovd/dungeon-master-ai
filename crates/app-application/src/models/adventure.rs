use super::identity::{CampaignId, Revision, SessionId};
pub use app_domain::adventure::{Action, AdventureState, Fact, Location, BELL_ADVENTURE_ID};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdventureProjection {
    pub state: AdventureState,
    pub available_actions: Vec<Action>,
}

impl From<AdventureState> for AdventureProjection {
    fn from(state: AdventureState) -> Self {
        let available_actions = state.available_actions();
        Self {
            state,
            available_actions,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdventureCommand {
    pub campaign_id: CampaignId,
    pub session_id: SessionId,
    pub character_id: Uuid,
    pub expected_revision: Revision,
    pub causation_id: String,
    pub action: Action,
}

#[derive(Debug, Clone)]
pub struct AdventureIntent {
    pub campaign_id: CampaignId,
    pub session_id: SessionId,
    pub causation_id: String,
    pub action: Action,
}
