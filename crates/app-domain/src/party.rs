use serde::{Deserialize, Serialize};
use uuid::Uuid;
pub const MAX_PARTY_MEMBERS: usize = 4;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartyRole {
    Host,
    Player,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartyMember {
    pub member_id: Uuid,
    pub character_id: Uuid,
    pub role: PartyRole,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartyRoster {
    pub host_member_id: Uuid,
    pub members: Vec<PartyMember>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PartyError {
    #[error("invalid party identity")]
    InvalidIdentity,
    #[error("unknown party member")]
    UnknownMember,
    #[error("character belongs to another member")]
    NotOwner,
    #[error("character already owned")]
    CharacterAlreadyOwned,
    #[error("member already bound")]
    MemberAlreadyBound,
    #[error("party is full")]
    Full,
}
impl PartyRoster {
    pub fn new(host_member_id: Uuid, character_id: Uuid) -> Result<Self, PartyError> {
        let party = Self {
            host_member_id,
            members: vec![PartyMember {
                member_id: host_member_id,
                character_id,
                role: PartyRole::Host,
            }],
        };
        party.validate()?;
        Ok(party)
    }
    pub fn validate(&self) -> Result<(), PartyError> {
        use std::collections::BTreeSet;
        if self.host_member_id.is_nil()
            || self.members.is_empty()
            || self.members.len() > MAX_PARTY_MEMBERS
            || self
                .members
                .iter()
                .any(|member| member.member_id.is_nil() || member.character_id.is_nil())
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
            || self
                .members
                .iter()
                .filter(|member| member.role == PartyRole::Host)
                .count()
                != 1
            || !self.members.iter().any(|member| {
                member.member_id == self.host_member_id && member.role == PartyRole::Host
            })
        {
            return Err(PartyError::InvalidIdentity);
        }
        Ok(())
    }
    pub fn join(&mut self, member_id: Uuid, character_id: Uuid) -> Result<(), PartyError> {
        self.validate()?;
        if member_id.is_nil() || character_id.is_nil() {
            return Err(PartyError::InvalidIdentity);
        }
        if let Some(member) = self
            .members
            .iter()
            .find(|member| member.member_id == member_id)
        {
            return if member.character_id == character_id {
                Ok(())
            } else {
                Err(PartyError::MemberAlreadyBound)
            };
        }
        if self
            .members
            .iter()
            .any(|member| member.character_id == character_id)
        {
            return Err(PartyError::CharacterAlreadyOwned);
        }
        if self.members.len() >= MAX_PARTY_MEMBERS {
            return Err(PartyError::Full);
        }
        self.members.push(PartyMember {
            member_id,
            character_id,
            role: PartyRole::Player,
        });
        Ok(())
    }
    /// The transport must authenticate member_id before asking this rule.
    pub fn authorize(&self, member_id: Uuid, character_id: Uuid) -> Result<(), PartyError> {
        self.validate()?;
        let member = self
            .members
            .iter()
            .find(|member| member.member_id == member_id)
            .ok_or(PartyError::UnknownMember)?;
        if member.character_id != character_id {
            return Err(PartyError::NotOwner);
        }
        Ok(())
    }
}
