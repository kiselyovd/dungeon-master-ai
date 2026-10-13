use super::identity::{CampaignId, Revision, SessionId};
pub use app_domain::adventure::{
    Action, AdventureState, DefenseOutcome, Fact, Location, BELL_ADVENTURE_ID,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdventureProjection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map: Option<app_domain::combat::geometry::BattleMap>,
    pub state: AdventureState,
    pub available_actions: Vec<Action>,
    /// Explicit objectives offered by the current public scene. No future or
    /// private-canon objective is inferred by a presentation adapter.
    #[serde(default)]
    pub public_goals: Vec<Action>,
}

impl From<AdventureState> for AdventureProjection {
    fn from(state: AdventureState) -> Self {
        let available_actions = state.available_actions();
        let public_goals = available_actions
            .iter()
            .filter(|action| !matches!(action, Action::Move(_) | Action::Finish))
            .copied()
            .collect();
        Self {
            map: if state.location == Location::Archive {
                Some(app_domain::combat::geometry::bell_archive_map())
            } else {
                None
            },
            state,
            available_actions,
            public_goals,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_goals_only_offer_current_scene_actions() {
        let projection = AdventureProjection::from(AdventureState::new());
        assert!(!projection.public_goals.is_empty());
        assert!(projection.public_goals.iter().all(|goal| {
            projection.available_actions.contains(goal)
                && !matches!(goal, Action::Move(_) | Action::Finish)
        }));
        assert!(!projection.public_goals.contains(&Action::RecoverLedger));
        let mut closed = projection.state;
        closed.closed = true;
        assert!(AdventureProjection::from(closed).public_goals.is_empty());
    }
}
