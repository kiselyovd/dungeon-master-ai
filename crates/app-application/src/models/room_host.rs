use super::{
    identity::{CampaignId, Revision, SessionId},
    room::PublicRoom,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeHostRoomRequest {
    pub request_id: Uuid,
    pub campaign_id: CampaignId,
    pub session_id: SessionId,
    pub expected_revision: Revision,
    pub bind_addr: SocketAddr,
}
/// Native transport retains capability privately; renderer receives a sanitized view.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeHostRoomGrant {
    pub room: PublicRoom,
    pub member_id: Uuid,
    pub capability: String,
    pub invitation: String,
    pub address: SocketAddr,
}
