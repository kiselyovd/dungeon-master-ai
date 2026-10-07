use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::SocketAddr;
use uuid::Uuid;

/// Closed native operation catalog. Identity and destination come from native state.
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NativeRoomOperation {
    State {},
    Subscribe {
        after: Option<u64>,
    },
    DmTurn {
        request_id: Uuid,
        player_message: String,
        resume: bool,
    },
    PendingAction {},
    Updates {
        after: u64,
    },
    Party {},
    OwnProposal {},
    PendingProposals {},
    RestStatus {},
    SubmitCharacter {
        proposal_id: Uuid,
        character_id: Uuid,
        character: Value,
    },
    ApproveCharacter {
        proposal_id: Uuid,
        expected_revision: u64,
        causation_id: String,
    },
    Adventure {
        request_id: String,
        expected_revision: u64,
        action: super::adventure::Action,
    },
    Rest {
        request_id: String,
        expected_revision: u64,
        intent: super::rest::RestIntent,
    },
    Combat {
        encounter_id: Uuid,
        request_id: String,
        expected_revision: u64,
        action: crate::agent::hero_intentions::HeroCombatIntent,
    },
}
/// Privileged loopback DTO, never accepted from the renderer or public listener.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeRoomTransportRequest {
    pub address: SocketAddr,
    pub room_id: Uuid,
    pub member_id: Uuid,
    pub capability: String,
    pub operation: NativeRoomOperation,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeRoomTransportResult {
    pub status: u16,
    pub body: Option<Value>,
}
