use super::{
    adventure::Action,
    identity::Revision,
    rest::{RestIntent, RestOutcome},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct RoomAdventureCommand {
    pub request_id: String,
    pub expected_revision: Revision,
    pub action: Action,
}
#[derive(Debug, Clone, Serialize)]
pub struct RoomRestCommand {
    pub request_id: String,
    pub expected_revision: Revision,
    pub intent: RestIntent,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomWorldReceipt {
    pub receipt_revision: Revision,
    pub duplicate: bool,
    pub rest_outcome: Option<RestOutcome>,
}
