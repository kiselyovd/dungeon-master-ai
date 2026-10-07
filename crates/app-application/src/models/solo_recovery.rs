use super::{
    identity::{CampaignId, Revision, SessionId},
    room_recovery::{PublicHeroRecovery, RecoveryIntent, RoomRecoveryReceipt},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoloRecoveryCommand {
    pub campaign_id: CampaignId,
    pub session_id: SessionId,
    pub character_id: Uuid,
    pub expected_revision: Revision,
    pub causation_id: String,
    pub intent: RecoveryIntent,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoloRecoveryStatus {
    pub campaign_id: Uuid,
    pub session_id: Uuid,
    pub character_id: Uuid,
    pub revision: u64,
    pub hp: i32,
    pub max_hp: i32,
    pub elapsed_seconds: u64,
    pub safe: bool,
    pub combat_active: bool,
    pub recovery: PublicHeroRecovery,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoloRecoveryResult {
    pub status: SoloRecoveryStatus,
    pub receipt: RoomRecoveryReceipt,
}
