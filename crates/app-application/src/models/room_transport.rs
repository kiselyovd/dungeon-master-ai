use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::SocketAddr;
use uuid::Uuid;

/// Closed native operation catalog. Identity and destination come from native state.
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NativeRoomOperation {
    Leave {
        request_id: String,
        expected_epoch: u64,
    },
    State {},
    Subscribe {
        after: Option<u64>,
    },
    DmTurn {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        clarification: Option<crate::agent::human_commands::HumanCommandClarificationReply>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        awaiting_clarification: Option<Uuid>,
        request_id: Uuid,
        player_message: String,
        resume: bool,
    },
    PendingAction {},
    AgentRecovery {},
    Updates {
        after: u64,
    },
    Party {},
    OwnProposal {},
    PendingProposals {},
    RestStatus {},
    Recovery {
        request_id: String,
        expected_revision: u64,
        intent: super::room_recovery::RecoveryIntent,
    },
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
