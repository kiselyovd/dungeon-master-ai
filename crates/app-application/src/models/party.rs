use super::identity::{CampaignId, Revision, SessionId};
pub use app_domain::party::{PartyError, PartyMember, PartyRole, PartyRoster};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartyState {
    pub campaign_id: CampaignId,
    pub session_id: SessionId,
    pub revision: Revision,
    pub roster: PartyRoster,
    pub characters: BTreeMap<Uuid, Value>,
}
#[derive(Debug, Clone, Serialize)]
pub struct InitializeParty {
    pub campaign_id: CampaignId,
    pub session_id: SessionId,
    pub expected_revision: Revision,
    pub causation_id: String,
}
/// verified_host_member_id must come from the authenticated room/local owner.
#[derive(Debug, Clone, Serialize)]
pub struct EnrollPartyCharacter {
    pub campaign_id: CampaignId,
    pub session_id: SessionId,
    pub expected_revision: Revision,
    pub causation_id: String,
    pub verified_host_member_id: Uuid,
    pub member_id: Uuid,
    pub character_id: Uuid,
    pub character: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PublicPartyActor {
    pub member_id: Uuid,
    pub character_id: Uuid,
    pub role: PartyRole,
    pub name: String,
    pub class_id: String,
    pub level: u8,
    pub hp: i32,
    pub max_hp: i32,
    pub ac: i32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartyView {
    pub campaign_id: CampaignId,
    pub session_id: SessionId,
    pub revision: Revision,
    pub roster: PartyRoster,
    pub actors: Vec<PublicPartyActor>,
    pub own_character: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartyCharacterSnapshot {
    pub member_id: Uuid,
    pub character_id: Uuid,
    pub character: Value,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartySnapshot {
    pub host_member_id: Uuid,
    pub members: Vec<PartyCharacterSnapshot>,
}
impl PartySnapshot {
    pub fn validate(&self, host: &Value) -> Result<(), PartyError> {
        use std::collections::BTreeSet;
        if self.members.len() >= app_domain::party::MAX_PARTY_MEMBERS
            || self
                .members
                .iter()
                .any(|member| member.member_id == self.host_member_id)
            || self
                .members
                .iter()
                .map(|member| member.member_id)
                .collect::<BTreeSet<_>>()
                .len()
                != self.members.len()
            || self
                .members
                .iter()
                .map(|member| member.character_id)
                .collect::<BTreeSet<_>>()
                .len()
                != self.members.len()
        {
            return Err(PartyError::InvalidIdentity);
        }
        if !super::campaign_state::valid_character_document(host)
            || !super::campaign_state::within_character_budget(host)
        {
            return Err(PartyError::InvalidIdentity);
        }
        let host_id = host["characterId"]
            .as_str()
            .and_then(|id| Uuid::parse_str(id).ok())
            .ok_or(PartyError::InvalidIdentity)?;
        let mut roster = PartyRoster::new(self.host_member_id, host_id)?;
        for member in &self.members {
            if !super::campaign_state::valid_character_document(&member.character)
                || !super::campaign_state::within_character_budget(&member.character)
                || member.character["characterId"].as_str()
                    != Some(member.character_id.to_string().as_str())
            {
                return Err(PartyError::InvalidIdentity);
            }
            roster.join(member.member_id, member.character_id)?;
        }
        if roster.members.len() != self.members.len() + 1 {
            return Err(PartyError::InvalidIdentity);
        }
        Ok(())
    }
}
impl PartyState {
    pub fn view_for(&self, member_id: Uuid) -> Result<PartyView, PartyError> {
        self.roster.validate()?;
        let member = self
            .roster
            .members
            .iter()
            .find(|member| member.member_id == member_id)
            .ok_or(PartyError::UnknownMember)?;
        let mut actors = Vec::new();
        for member in &self.roster.members {
            let sheet = self
                .characters
                .get(&member.character_id)
                .ok_or(PartyError::InvalidIdentity)?;
            if !super::campaign_state::valid_character_document(sheet) {
                return Err(PartyError::InvalidIdentity);
            }
            actors.push(PublicPartyActor {
                member_id: member.member_id,
                character_id: member.character_id,
                role: member.role,
                name: sheet["name"]
                    .as_str()
                    .ok_or(PartyError::InvalidIdentity)?
                    .into(),
                class_id: sheet["heroClass"]
                    .as_str()
                    .ok_or(PartyError::InvalidIdentity)?
                    .into(),
                level: sheet["level"].as_u64().ok_or(PartyError::InvalidIdentity)? as u8,
                hp: sheet["hp"].as_i64().ok_or(PartyError::InvalidIdentity)? as i32,
                max_hp: sheet["hpMax"].as_i64().ok_or(PartyError::InvalidIdentity)? as i32,
                ac: sheet["ac"].as_i64().ok_or(PartyError::InvalidIdentity)? as i32,
            });
        }
        Ok(PartyView {
            campaign_id: self.campaign_id,
            session_id: self.session_id,
            revision: self.revision,
            roster: self.roster.clone(),
            actors,
            own_character: self
                .characters
                .get(&member.character_id)
                .ok_or(PartyError::InvalidIdentity)?
                .clone(),
        })
    }
}
