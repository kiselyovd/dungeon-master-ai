use super::identity::Revision;
pub use app_domain::combat::result_events::DeathSaveResult;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RecoveryIntent {
    Aid { target_character_id: Uuid },
    Wait { target_character_id: Uuid },
    DeathSave {},
}
#[derive(Debug, Clone, Serialize)]
pub struct RoomRecoveryCommand {
    pub request_id: String,
    pub expected_revision: Revision,
    pub intent: RecoveryIntent,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MedicineCheck {
    pub rolls: Vec<i32>,
    pub modifier: i32,
    pub total: i32,
    pub disadvantage: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryDeathSave {
    pub character_id: Uuid,
    pub result: DeathSaveResult,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomRecoveryReceipt {
    pub receipt_revision: Revision,
    pub duplicate: bool,
    pub target_character_id: Uuid,
    pub stabilized: bool,
    pub recovered: bool,
    pub medicine: Option<MedicineCheck>,
    pub death_saves: Vec<RecoveryDeathSave>,
    pub ready_at_seconds: Option<u64>,
}
