use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedRoomTool {
    pub nonce: String,
    pub expected_revision: u64,
    pub encounter_id: Option<Uuid>,
}
