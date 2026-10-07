use super::identity::{CampaignId, Revision, SessionId};
use super::party::PartyRole;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoomMember {
    pub member_id: Uuid,
    pub display_name: String,
    pub role: PartyRole,
    pub active: bool,
    pub credential_id: Uuid,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoomInvitation {
    #[serde(default)]
    pub target_member_id: Option<Uuid>,
    pub invitation_id: Uuid,
    pub expires_at: u64,
    pub consumed_by: Option<Uuid>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoomRecord {
    pub room_id: Uuid,
    pub campaign_id: CampaignId,
    pub session_id: SessionId,
    pub host_member_id: Uuid,
    pub epoch: u64,
    pub active: bool,
    pub invitation: RoomInvitation,
    pub members: Vec<RoomMember>,
}
#[derive(Debug, Clone, Serialize)]
pub struct CreateLocalRoom {
    pub campaign_id: CampaignId,
    pub session_id: SessionId,
    pub expected_revision: Revision,
    pub causation_id: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoomJoinRecord {
    pub room_id: Uuid,
    pub join_id: Uuid,
    pub member_id: Uuid,
    pub display_name: String,
    pub credential_id: Uuid,
}
#[derive(Debug, Clone, Serialize)]
pub struct CommitRoomJoin {
    pub record: RoomJoinRecord,
    pub expected_epoch: u64,
    pub invitation_id: Uuid,
    pub now: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PublicRoomMember {
    pub member_id: Uuid,
    pub display_name: String,
    pub role: PartyRole,
    pub active: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PublicRoom {
    pub room_id: Uuid,
    pub campaign_id: CampaignId,
    pub session_id: SessionId,
    pub active: bool,
    pub members: Vec<PublicRoomMember>,
}
impl RoomRecord {
    pub fn public_view(&self) -> PublicRoom {
        PublicRoom {
            room_id: self.room_id,
            campaign_id: self.campaign_id,
            session_id: self.session_id,
            active: self.active,
            members: self
                .members
                .iter()
                .map(|member| PublicRoomMember {
                    member_id: member.member_id,
                    display_name: member.display_name.clone(),
                    role: member.role,
                    active: member.active,
                })
                .collect(),
        }
    }
    pub fn valid(&self) -> bool {
        use std::collections::BTreeSet;
        !self.room_id.is_nil()
            && !self.campaign_id.0.is_nil()
            && !self.session_id.0.is_nil()
            && !self.host_member_id.is_nil()
            && self.epoch > 0
            && self.epoch <= i64::MAX as u64
            && self.members.len() <= app_domain::party::MAX_PARTY_MEMBERS
            && !self.members.is_empty()
            && self.members.iter().all(|member| {
                !member.member_id.is_nil()
                    && !member.credential_id.is_nil()
                    && valid_display_name(&member.display_name)
            })
            && self
                .members
                .iter()
                .map(|member| member.member_id)
                .collect::<BTreeSet<_>>()
                .len()
                == self.members.len()
            && self
                .members
                .iter()
                .map(|member| member.credential_id)
                .collect::<BTreeSet<_>>()
                .len()
                == self.members.len()
            && self
                .members
                .iter()
                .filter(|member| member.role == PartyRole::Host)
                .count()
                == 1
            && self.members.iter().any(|member| {
                member.member_id == self.host_member_id
                    && member.role == PartyRole::Host
                    && member.active
            })
            && !self.invitation.invitation_id.is_nil()
            && self.invitation.expires_at <= i64::MAX as u64
            && self.invitation.consumed_by.is_none_or(|id| !id.is_nil())
            && self.invitation.target_member_id.is_none_or(|id| {
                self.members
                    .iter()
                    .any(|member| member.member_id == id && member.role == PartyRole::Player)
            })
    }
}
pub fn valid_display_name(value: &str) -> bool {
    !value.trim().is_empty() && value.chars().count() <= 64 && !value.chars().any(char::is_control)
}

#[derive(Debug, Clone)]
pub struct RoomLease {
    pub(crate) room_id: Uuid,
    pub(crate) member_id: Uuid,
    pub(crate) campaign_id: CampaignId,
    pub(crate) session_id: SessionId,
    pub(crate) epoch: u64,
    pub(crate) role: PartyRole,
}
impl RoomLease {
    pub fn room_id(&self) -> Uuid {
        self.room_id
    }
    pub fn member_id(&self) -> Uuid {
        self.member_id
    }
    pub fn campaign_id(&self) -> CampaignId {
        self.campaign_id
    }
    pub fn session_id(&self) -> SessionId {
        self.session_id
    }
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    pub fn role(&self) -> PartyRole {
        self.role
    }
}
