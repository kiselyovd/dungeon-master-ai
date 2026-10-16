use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedRoomTool {
    pub nonce: String,
    pub expected_revision: u64,
    pub encounter_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check_result: Option<serde_json::Value>,
}
