use super::{
    identity::Revision,
    party::{PartyRole, PublicPartyActor},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    Pending,
    Accepted,
    Superseded,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RoomCharacterProposal {
    pub room_id: Uuid,
    pub proposal_id: Uuid,
    pub member_id: Uuid,
    pub character_id: Uuid,
    pub status: ProposalStatus,
    pub character: Value,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProposalSummary {
    pub proposal_id: Uuid,
    pub status: ProposalStatus,
    pub actor: PublicPartyActor,
}
#[derive(Clone, Serialize)]
pub struct OwnCharacterProposal {
    pub summary: ProposalSummary,
    pub character: Value,
}
#[derive(Clone, Serialize)]
pub struct SubmitRoomCharacter {
    pub proposal_id: Uuid,
    pub character_id: Uuid,
    pub character: Value,
}
#[derive(Debug, Clone, Serialize)]
pub struct ApproveRoomCharacter {
    pub proposal_id: Uuid,
    pub expected_revision: Revision,
    pub causation_id: String,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CharacterApproval {
    pub proposal: ProposalSummary,
    pub revision: Revision,
}
impl RoomCharacterProposal {
    pub fn summary(&self) -> Option<ProposalSummary> {
        if self.room_id.is_nil()
            || self.proposal_id.is_nil()
            || self.member_id.is_nil()
            || self.character_id.is_nil()
            || !super::campaign_state::valid_character_document(&self.character)
            || !super::campaign_state::within_character_budget(&self.character)
            || self.character["characterId"]
                .as_str()
                .and_then(|value| Uuid::parse_str(value).ok())
                != Some(self.character_id)
        {
            return None;
        }
        Some(ProposalSummary {
            proposal_id: self.proposal_id,
            status: self.status,
            actor: PublicPartyActor {
                member_id: self.member_id,
                character_id: self.character_id,
                role: PartyRole::Player,
                name: self.character["name"].as_str()?.into(),
                class_id: self.character["heroClass"].as_str()?.into(),
                level: self.character["level"].as_u64()? as u8,
                hp: self.character["hp"].as_i64()? as i32,
                max_hp: self.character["hpMax"].as_i64()? as i32,
                ac: self.character["ac"].as_i64()? as i32,
            },
        })
    }
}
