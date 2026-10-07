use super::identity::{CampaignId, Revision, SessionId};
pub use app_domain::rest::{RestIntent, RestOutcome};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdventuringState {
    pub elapsed_seconds: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestCommand {
    pub campaign_id: CampaignId,
    pub session_id: SessionId,
    pub character_id: Uuid,
    pub expected_revision: Revision,
    pub causation_id: String,
    pub intent: RestIntent,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestStatus {
    pub campaign_id: Uuid,
    pub session_id: Uuid,
    pub character_id: Uuid,
    pub revision: u64,
    pub hp: i32,
    pub max_hp: i32,
    pub hit_die: u8,
    pub hit_dice_remaining: u8,
    pub slots: std::collections::BTreeMap<u8, app_domain::combat::magic::SpellSlots>,
    pub elapsed_seconds: u64,
    pub long_rest_ready_in_seconds: u64,
    pub safe: bool,
    pub combat_active: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestResult {
    pub status: RestStatus,
    pub outcome: RestOutcome,
}
