use super::room::PublicRoom;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeGuestJoinRequest {
    pub address: SocketAddr,
    pub room_id: Uuid,
    pub join_id: Uuid,
    pub display_name: String,
    pub invitation: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeGuestResumeRequest {
    pub room_id: Uuid,
    pub join_id: Uuid,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeGuestConnection {
    pub address: SocketAddr,
    pub room: PublicRoom,
    pub member_id: Uuid,
    pub capability: String,
    pub join_id: Uuid,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredNativeGuestJoin {
    pub request: NativeGuestJoinRequest,
    pub resume_proof: String,
    pub connection: Option<NativeGuestConnection>,
}
