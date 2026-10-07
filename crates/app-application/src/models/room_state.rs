use super::{
    adventure::AdventureProjection,
    combat::CombatProjection,
    identity::Revision,
    party::PublicPartyActor,
    rest::AdventuringState,
    room::PublicRoom,
    room_character::{OwnCharacterProposal, ProposalSummary},
};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Serialize)]
pub struct PublicRoomMessage {
    pub id: Uuid,
    pub role: String,
    pub text: String,
    pub truncated: bool,
}
#[derive(Clone, Serialize)]
pub struct RoomState {
    pub campaign: super::session::CampaignDetail,
    pub room: PublicRoom,
    pub title: String,
    pub revision: Revision,
    pub actors: Vec<PublicPartyActor>,
    pub own_character: Option<Value>,
    pub own_proposal: Option<OwnCharacterProposal>,
    pub pending_proposals: Option<Vec<ProposalSummary>>,
    pub messages: Vec<PublicRoomMessage>,
    pub combat: Option<CombatProjection>,
    pub scene: Option<Value>,
    pub adventure: Option<AdventureProjection>,
    pub clock: AdventuringState,
    pub journal: Vec<Value>,
    pub npcs: Vec<Value>,
    pub agent_busy: bool,
}
#[derive(Clone, Serialize)]
pub struct RoomChange {
    pub sequence: u64,
    pub kind: String,
    pub payload: Value,
}
#[derive(Clone, Serialize)]
pub struct RoomUpdate {
    pub cursor: u64,
    pub head: u64,
    pub timeline_reset_cursor: u64,
    pub reset: bool,
    pub events: Vec<RoomChange>,
    pub state: RoomState,
}
