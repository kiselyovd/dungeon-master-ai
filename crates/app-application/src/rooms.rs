use crate::{
    models::{
        party::{InitializeParty, PartyRole, PartyView},
        room::*,
    },
    ports::{
        repositories::{PartyRepository, RoomRepository},
        secrets::SecretsStore,
    },
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
pub mod agent;
mod characters;
mod combat;
pub mod dm_turns;
mod state;
mod world;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RoomError {
    #[error("room_action_rejected")]
    ActionRejected,
    #[error("room_state_conflict")]
    Conflict,
    #[error("room_access_denied")]
    Denied,
    #[error("room_input_invalid")]
    InvalidInput,
    #[error("room_repository_failed")]
    Repository,
    #[error("room_secrets_unavailable")]
    Secrets,
}
/// Deliberately neither Debug nor Serialize: transport must explicitly handle it.
pub struct RoomSecret(String);
impl RoomSecret {
    pub fn generate() -> Self {
        Self(format!(
            "{}{}",
            Uuid::new_v4().simple(),
            Uuid::new_v4().simple()
        ))
    }
    pub fn from_input(value: String) -> Result<Self, RoomError> {
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(RoomError::Denied);
        }
        Ok(Self(value))
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
}
pub struct RoomGrant {
    pub room: PublicRoom,
    pub member_id: Uuid,
    pub capability: RoomSecret,
}
pub struct CreatedRoom {
    pub grant: RoomGrant,
    pub invitation: RoomSecret,
}
pub struct JoinRoom {
    pub room_id: Uuid,
    pub join_id: Uuid,
    pub display_name: String,
    pub invitation: RoomSecret,
    pub resume_proof: RoomSecret,
}
pub trait RoomClock: Send + Sync {
    fn now(&self) -> u64;
}
pub struct SystemRoomClock;
impl RoomClock for SystemRoomClock {
    fn now(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs())
    }
}
#[derive(Clone)]
pub struct RoomSubject {
    lease: RoomLease,
}
#[derive(Debug, Clone, Copy)]
pub enum RoomOperation {
    ReadState,
    SubmitOwnCharacter,
    ActOwnCharacter,
    ApproveCharacters,
    ManageSnapshots,
    ManageRoom,
    ConfigureProvider,
}
impl RoomSubject {
    pub fn lease(&self) -> &RoomLease {
        &self.lease
    }
    pub fn permits(&self, operation: RoomOperation) -> bool {
        match operation {
            RoomOperation::ConfigureProvider => false,
            RoomOperation::ReadState
            | RoomOperation::SubmitOwnCharacter
            | RoomOperation::ActOwnCharacter => true,
            _ => self.lease.role == PartyRole::Host,
        }
    }
}
#[derive(Serialize, Deserialize)]
struct PendingJoin {
    member_id: Uuid,
    credential_id: Uuid,
    display_name: String,
    resume_proof: String,
}
fn credential_key(room: Uuid, credential: Uuid) -> String {
    format!("room:{room}:credential:{credential}")
}
fn invitation_key(room: Uuid, invitation: Uuid) -> String {
    format!("room:{room}:invitation:{invitation}")
}
fn join_key(room: Uuid, join: Uuid) -> String {
    format!("room:{room}:join:{join}")
}
fn secret_equal(expected: &str, provided: &str) -> bool {
    if expected.len() != 64 || provided.len() != 64 {
        return false;
    }
    expected
        .bytes()
        .zip(provided.bytes())
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

// All instances in the process share the lane; bootstrap also owns the database
// authority lock before composing any service against persistent stores.
static ROOM_MUTATION: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
pub struct RoomService {
    parties: Arc<dyn PartyRepository>,
    rooms: Arc<dyn RoomRepository>,
    secrets: Arc<dyn SecretsStore>,
    clock: Arc<dyn RoomClock>,
    media_assets: Option<Arc<crate::media_assets::MediaAssets>>,
    dossiers: Option<Arc<dyn crate::ports::campaign_bundles::PlayerDossierExporter>>,
}
impl RoomService {
    pub fn new(
        parties: Arc<dyn PartyRepository>,
        rooms: Arc<dyn RoomRepository>,
        secrets: Arc<dyn SecretsStore>,
        clock: Arc<dyn RoomClock>,
    ) -> Self {
        Self {
            parties,
            rooms,
            secrets,
            clock,
            media_assets: None,
            dossiers: None,
        }
    }
    pub fn with_media_assets(mut self, assets: Arc<crate::media_assets::MediaAssets>) -> Self {
        self.media_assets = Some(assets);
        self
    }
    pub fn with_dossiers(
        mut self,
        exporter: Arc<dyn crate::ports::campaign_bundles::PlayerDossierExporter>,
    ) -> Self {
        self.dossiers = Some(exporter);
        self
    }
    pub async fn export_own_dossier(&self, subject: &RoomSubject) -> Result<Vec<u8>, RoomError> {
        let _lane = ROOM_MUTATION.lock().await;
        let room = self.authorized_room(subject).await?;
        let party = self
            .parties
            .party(room.campaign_id)
            .await
            .map_err(|_| RoomError::Repository)?
            .ok_or(RoomError::Denied)?;
        if party.session_id != room.session_id {
            return Err(RoomError::Denied);
        }
        let own = party
            .view_for(subject.lease.member_id)
            .map_err(|_| RoomError::Denied)?;
        let actor = own
            .own_character
            .get("characterId")
            .and_then(serde_json::Value::as_str)
            .and_then(|value| Uuid::parse_str(value).ok())
            .ok_or(RoomError::Denied)?;
        let exporter = self.dossiers.as_ref().ok_or(RoomError::Repository)?;
        let bytes = exporter
            .export_own(
                room.campaign_id,
                actor,
                i64::try_from(self.clock.now().saturating_mul(1000))
                    .map_err(|_| RoomError::Repository)?,
            )
            .await
            .map_err(|_| RoomError::Repository)?;
        self.authorized_room(subject).await?;
        Ok(bytes)
    }
    pub async fn upload_portrait(
        &self,
        subject: &RoomSubject,
        id: Uuid,
        mime_type: &str,
        bytes: Vec<u8>,
    ) -> Result<
        crate::models::media_asset::MediaReference,
        crate::models::media_asset::MediaAssetError,
    > {
        use crate::models::media_asset::*;
        let _lane = ROOM_MUTATION.lock().await;
        let room = self
            .authorized_room(subject)
            .await
            .map_err(|_| MediaAssetError::Denied)?;
        let assets = self.media_assets.as_ref().ok_or(MediaAssetError::Storage)?;
        assets
            .register_as(
                id,
                MediaOwner::RoomMember {
                    campaign_id: room.campaign_id.0,
                    member_id: subject.lease.member_id(),
                },
                MediaCategory::Portrait,
                MediaOrigin::Uploaded,
                mime_type,
                bytes,
            )
            .await
    }
    pub async fn public_asset(
        &self,
        subject: &RoomSubject,
        id: Uuid,
    ) -> Result<
        crate::models::media_asset::OwnedMediaAsset,
        crate::models::media_asset::MediaAssetError,
    > {
        use crate::models::media_asset::*;
        self.media_assets
            .as_ref()
            .ok_or(MediaAssetError::Storage)?
            .read(id, &MediaReadScope::Room(subject.lease.clone()))
            .await
    }
    async fn read_secret(&self, key: &str) -> Result<RoomSecret, RoomError> {
        let value = self
            .secrets
            .get(key)
            .await
            .map_err(|_| RoomError::Secrets)?
            .ok_or(RoomError::Secrets)?;
        RoomSecret::from_input(value).map_err(|_| RoomError::Secrets)
    }
    async fn grant(&self, room: &RoomRecord, member: Uuid) -> Result<RoomGrant, RoomError> {
        let member = room
            .members
            .iter()
            .find(|record| record.member_id == member && record.active)
            .ok_or(RoomError::Denied)?;
        Ok(RoomGrant {
            room: room.public_view(),
            member_id: member.member_id,
            capability: self
                .read_secret(&credential_key(room.room_id, member.credential_id))
                .await?,
        })
    }
    /// Only an authenticated local application owner may call this entry point.
    pub async fn create_local_room(
        &self,
        command: CreateLocalRoom,
    ) -> Result<CreatedRoom, RoomError> {
        let _lane = ROOM_MUTATION.lock().await;
        if command.campaign_id.0.is_nil()
            || command.session_id.0.is_nil()
            || command.causation_id.trim().is_empty()
            || command.causation_id.len() > 200
        {
            return Err(RoomError::InvalidInput);
        }
        let signature = serde_json::to_string(&command).map_err(|_| RoomError::InvalidInput)?;
        if let Some(room) = self
            .rooms
            .find_creation(command.campaign_id, &command.causation_id, &signature)
            .await
            .map_err(|_| RoomError::Repository)?
        {
            return Ok(CreatedRoom {
                grant: self.grant(&room, room.host_member_id).await?,
                invitation: self
                    .read_secret(&invitation_key(room.room_id, room.invitation.invitation_id))
                    .await?,
            });
        }
        let party = self
            .parties
            .initialize_party(InitializeParty {
                campaign_id: command.campaign_id,
                session_id: command.session_id,
                expected_revision: command.expected_revision,
                causation_id: format!("room-party:{}", command.causation_id),
            })
            .await
            .map_err(|_| RoomError::Repository)?;
        let room_id = Uuid::new_v4();
        let invitation_id = Uuid::new_v4();
        let invitation = RoomSecret::generate();
        self.secrets
            .set(&invitation_key(room_id, invitation_id), invitation.expose())
            .await
            .map_err(|_| RoomError::Secrets)?;
        let mut members = Vec::new();
        for member in &party.roster.members {
            let credential_id = Uuid::new_v4();
            self.secrets
                .set(
                    &credential_key(room_id, credential_id),
                    RoomSecret::generate().expose(),
                )
                .await
                .map_err(|_| RoomError::Secrets)?;
            let display_name = party.characters[&member.character_id]["name"]
                .as_str()
                .ok_or(RoomError::InvalidInput)?
                .trim()
                .chars()
                .filter(|character| !character.is_control())
                .take(64)
                .collect::<String>();
            members.push(RoomMember {
                left: false,
                departure_epoch: 0,
                member_id: member.member_id,
                display_name,
                role: member.role,
                active: member.role == PartyRole::Host,
                credential_id,
            });
        }
        let room = RoomRecord {
            room_id,
            campaign_id: party.campaign_id,
            session_id: party.session_id,
            host_member_id: party.roster.host_member_id,
            epoch: 1,
            active: true,
            invitation: RoomInvitation {
                target_member_id: None,
                invitation_id,
                expires_at: self
                    .clock
                    .now()
                    .checked_add(600)
                    .ok_or(RoomError::InvalidInput)?,
                consumed_by: None,
            },
            members,
        };
        let room = self
            .rooms
            .create_room(room, &command.causation_id, &signature)
            .await
            .map_err(|_| RoomError::Repository)?;
        Ok(CreatedRoom {
            grant: self.grant(&room, room.host_member_id).await?,
            invitation: self
                .read_secret(&invitation_key(room.room_id, room.invitation.invitation_id))
                .await?,
        })
    }
    /// Native-owner compensation only. The immutable creation receipt proves the
    /// exact command's room, including when credential delivery failed after commit.
    pub async fn cancel_local_creation(&self, command: &CreateLocalRoom) -> Result<(), RoomError> {
        let _lane = ROOM_MUTATION.lock().await;
        let signature = serde_json::to_string(command).map_err(|_| RoomError::InvalidInput)?;
        let room = match self
            .rooms
            .find_creation(command.campaign_id, &command.causation_id, &signature)
            .await
        {
            Ok(room) => room,
            // A mismatched immutable input owns no creation and cannot compensate it.
            Err(crate::ports::repositories::RepositoryError::Operation {
                code: "room_causation_mismatch",
                ..
            }) => return Ok(()),
            Err(_) => return Err(RoomError::Repository),
        };
        let Some(room) = room else { return Ok(()) };
        if room.campaign_id != command.campaign_id || room.session_id != command.session_id {
            return Err(RoomError::Denied);
        }
        if room.active {
            self.rooms
                .close_room(&RoomLease {
                    room_id: room.room_id,
                    member_id: room.host_member_id,
                    campaign_id: room.campaign_id,
                    session_id: room.session_id,
                    epoch: room.epoch,
                    role: PartyRole::Host,
                })
                .await
                .map_err(|_| RoomError::Repository)?;
        }
        Ok(())
    }

    pub async fn authenticate(
        &self,
        room_id: Uuid,
        member_id: Uuid,
        capability: &RoomSecret,
    ) -> Result<RoomSubject, RoomError> {
        self.authenticate_membership(room_id, member_id, capability, false)
            .await
    }
    pub async fn authenticate_departure(
        &self,
        room_id: Uuid,
        member_id: Uuid,
        capability: &RoomSecret,
    ) -> Result<RoomSubject, RoomError> {
        self.authenticate_membership(room_id, member_id, capability, true)
            .await
    }
    async fn authenticate_membership(
        &self,
        room_id: Uuid,
        member_id: Uuid,
        capability: &RoomSecret,
        departure: bool,
    ) -> Result<RoomSubject, RoomError> {
        let room = self
            .rooms
            .room(room_id)
            .await
            .map_err(|_| RoomError::Repository)?
            .filter(|room| room.active && room.valid())
            .ok_or(RoomError::Denied)?;
        let member = room
            .members
            .iter()
            .find(|member| {
                member.member_id == member_id
                    && (member.active || departure && member.left)
                    && (!departure || member.role == PartyRole::Player)
            })
            .ok_or(RoomError::Denied)?;
        let expected = self
            .read_secret(&credential_key(room_id, member.credential_id))
            .await?;
        if !secret_equal(expected.expose(), capability.expose()) {
            return Err(RoomError::Denied);
        }
        Ok(RoomSubject {
            lease: RoomLease {
                room_id,
                member_id,
                campaign_id: room.campaign_id,
                session_id: room.session_id,
                epoch: room.epoch,
                role: member.role,
            },
        })
    }
    pub async fn current_room(&self, subject: &RoomSubject) -> Result<PublicRoom, RoomError> {
        let _lane = ROOM_MUTATION.lock().await;
        self.authorized_room(subject)
            .await
            .map(|room| room.public_view())
    }
    pub async fn own_party(&self, subject: &RoomSubject) -> Result<Option<PartyView>, RoomError> {
        let _lane = ROOM_MUTATION.lock().await;
        let room = self.authorized_room(subject).await?;
        let party = self
            .parties
            .party(room.campaign_id)
            .await
            .map_err(|_| RoomError::Repository)?
            .ok_or(RoomError::Repository)?;
        if party.session_id != room.session_id {
            return Err(RoomError::Denied);
        }
        let Some(member) = party
            .roster
            .members
            .iter()
            .find(|member| member.member_id == subject.lease.member_id)
        else {
            return Ok(None);
        };
        if member.role != subject.lease.role {
            return Err(RoomError::Denied);
        }
        party
            .view_for(member.member_id)
            .map(Some)
            .map_err(|_| RoomError::Repository)
    }
    async fn authorized_room(&self, subject: &RoomSubject) -> Result<RoomRecord, RoomError> {
        let room = self
            .rooms
            .room(subject.lease.room_id)
            .await
            .map_err(|_| RoomError::Repository)?
            .ok_or(RoomError::Denied)?;
        if !room.valid()
            || !room.active
            || room.epoch != subject.lease.epoch
            || room.campaign_id != subject.lease.campaign_id
            || room.session_id != subject.lease.session_id
            || !room.members.iter().any(|member| {
                member.active
                    && member.member_id == subject.lease.member_id
                    && member.role == subject.lease.role
            })
        {
            return Err(RoomError::Denied);
        }
        Ok(room)
    }
    pub async fn join(&self, command: JoinRoom) -> Result<RoomGrant, RoomError> {
        self.join_with_return(command, None).await
    }
    pub async fn return_join(
        &self,
        command: JoinRoom,
        intent: crate::models::room::RoomReturnIntent,
    ) -> Result<RoomGrant, RoomError> {
        if intent.expected_departure_epoch == 0
            || intent.request_id.trim().is_empty()
            || intent.request_id.len() > 64
            || intent.request_id.chars().any(char::is_control)
        {
            return Err(RoomError::InvalidInput);
        }
        self.join_with_return(command, Some(intent)).await
    }
    async fn join_with_return(
        &self,
        command: JoinRoom,
        return_intent: Option<crate::models::room::RoomReturnIntent>,
    ) -> Result<RoomGrant, RoomError> {
        let _lane = ROOM_MUTATION.lock().await;
        if command.room_id.is_nil()
            || command.join_id.is_nil()
            || !valid_display_name(&command.display_name)
        {
            return Err(RoomError::InvalidInput);
        }
        let name = command.display_name.trim().to_owned();
        let room = self
            .rooms
            .room(command.room_id)
            .await
            .map_err(|_| RoomError::Repository)?
            .filter(|room| room.active && room.valid())
            .ok_or(RoomError::Denied)?;
        let stored = self
            .secrets
            .get(&join_key(command.room_id, command.join_id))
            .await
            .map_err(|_| RoomError::Secrets)?;
        let pending: Option<PendingJoin> = stored
            .map(|raw| serde_json::from_str(&raw).map_err(|_| RoomError::Secrets))
            .transpose()?;
        if pending.as_ref().is_some_and(|pending| {
            pending.display_name != name
                || !secret_equal(&pending.resume_proof, command.resume_proof.expose())
        }) {
            return Err(RoomError::Denied);
        }
        if let Some(record) = self
            .rooms
            .find_join(room.room_id, command.join_id)
            .await
            .map_err(|_| RoomError::Repository)?
        {
            let pending = pending.ok_or(RoomError::Denied)?;
            if record.member_id != pending.member_id
                || record.credential_id != pending.credential_id
                || record.display_name != name
                || !room.members.iter().any(|member| {
                    (member.active || member.left)
                        && member.member_id == record.member_id
                        && member.credential_id == record.credential_id
                })
            {
                return Err(RoomError::Denied);
            }
            let departed = room
                .members
                .iter()
                .any(|member| member.member_id == record.member_id && member.left);
            if departed && return_intent.is_none() {
                return Err(RoomError::Denied);
            }
            if let Some(intent) = return_intent {
                self.rooms
                    .return_member(
                        room.room_id,
                        record.member_id,
                        record.credential_id,
                        room.epoch,
                        intent.expected_departure_epoch,
                        &intent.request_id,
                    )
                    .await
                    .map_err(|error| match error {
                        crate::ports::repositories::RepositoryError::Operation {
                            code: "room_departure_conflict" | "room_causation_mismatch",
                            ..
                        } => RoomError::Conflict,
                        crate::ports::repositories::RepositoryError::Operation {
                            code: "room_access_denied",
                            ..
                        } => RoomError::Denied,
                        _ => RoomError::Repository,
                    })?;
                let current = self
                    .rooms
                    .room(room.room_id)
                    .await
                    .map_err(|_| RoomError::Repository)?
                    .ok_or(RoomError::Denied)?;
                return self.grant(&current, record.member_id).await;
            }
            return self.grant(&room, record.member_id).await;
        }
        if return_intent.is_some() {
            return Err(RoomError::Denied);
        }
        if room.invitation.consumed_by.is_some() || self.clock.now() >= room.invitation.expires_at {
            return Err(RoomError::Denied);
        }
        let expected = self
            .read_secret(&invitation_key(room.room_id, room.invitation.invitation_id))
            .await?;
        if !secret_equal(expected.expose(), command.invitation.expose()) {
            return Err(RoomError::Denied);
        }
        let pending = pending.unwrap_or_else(|| PendingJoin {
            member_id: room
                .invitation
                .target_member_id
                .unwrap_or_else(Uuid::new_v4),
            credential_id: Uuid::new_v4(),
            display_name: name.clone(),
            resume_proof: command.resume_proof.expose().into(),
        });
        if room
            .invitation
            .target_member_id
            .is_some_and(|target| target != pending.member_id)
        {
            return Err(RoomError::Denied);
        }
        self.secrets
            .set(
                &join_key(room.room_id, command.join_id),
                &serde_json::to_string(&pending).map_err(|_| RoomError::Secrets)?,
            )
            .await
            .map_err(|_| RoomError::Secrets)?;
        if self
            .secrets
            .get(&credential_key(room.room_id, pending.credential_id))
            .await
            .map_err(|_| RoomError::Secrets)?
            .is_none()
        {
            self.secrets
                .set(
                    &credential_key(room.room_id, pending.credential_id),
                    RoomSecret::generate().expose(),
                )
                .await
                .map_err(|_| RoomError::Secrets)?;
        }
        let joined = self
            .rooms
            .commit_join(CommitRoomJoin {
                record: RoomJoinRecord {
                    room_id: room.room_id,
                    join_id: command.join_id,
                    member_id: pending.member_id,
                    display_name: name,
                    credential_id: pending.credential_id,
                },
                expected_epoch: room.epoch,
                invitation_id: room.invitation.invitation_id,
                now: self.clock.now(),
            })
            .await
            .map_err(|_| RoomError::Repository)?;
        let room = self
            .rooms
            .room(room.room_id)
            .await
            .map_err(|_| RoomError::Repository)?
            .ok_or(RoomError::Repository)?;
        self.grant(&room, joined.member_id).await
    }
    pub async fn rotate_invitation(&self, subject: &RoomSubject) -> Result<RoomSecret, RoomError> {
        self.issue_invitation(subject, None).await
    }
    pub async fn invite_member(
        &self,
        subject: &RoomSubject,
        member_id: Uuid,
    ) -> Result<RoomSecret, RoomError> {
        self.issue_invitation(subject, Some(member_id)).await
    }
    async fn issue_invitation(
        &self,
        subject: &RoomSubject,
        target_member_id: Option<Uuid>,
    ) -> Result<RoomSecret, RoomError> {
        let _lane = ROOM_MUTATION.lock().await;
        let current = self
            .rooms
            .room(subject.lease.room_id)
            .await
            .map_err(|_| RoomError::Repository)?
            .ok_or(RoomError::Denied)?;
        if !current.active
            || current.epoch != subject.lease.epoch
            || current.campaign_id != subject.lease.campaign_id
            || current.session_id != subject.lease.session_id
            || subject.lease.role != PartyRole::Host
            || !current.members.iter().any(|member| {
                member.member_id == subject.lease.member_id
                    && member.role == PartyRole::Host
                    && member.active
            })
            || target_member_id.is_some_and(|id| {
                !current
                    .members
                    .iter()
                    .any(|member| member.member_id == id && member.role == PartyRole::Player)
            })
        {
            return Err(RoomError::Denied);
        }
        let invitation_id = Uuid::new_v4();
        let secret = RoomSecret::generate();
        self.secrets
            .set(
                &invitation_key(subject.lease.room_id, invitation_id),
                secret.expose(),
            )
            .await
            .map_err(|_| RoomError::Secrets)?;
        let result = self
            .rooms
            .rotate_invitation(
                &subject.lease,
                RoomInvitation {
                    target_member_id,
                    invitation_id,
                    expires_at: self
                        .clock
                        .now()
                        .checked_add(600)
                        .ok_or(RoomError::InvalidInput)?,
                    consumed_by: None,
                },
            )
            .await;
        if result.is_err() {
            if let Ok(Some(current)) = self.rooms.room(subject.lease.room_id).await {
                if current.invitation.invitation_id != invitation_id {
                    self.secrets
                        .delete(&invitation_key(subject.lease.room_id, invitation_id))
                        .await
                        .map_err(|_| RoomError::Secrets)?;
                }
            }
            return Err(RoomError::Denied);
        }
        Ok(secret)
    }
    pub async fn leave(
        &self,
        subject: &RoomSubject,
        expected_epoch: u64,
        request_id: &str,
    ) -> Result<crate::models::room::RoomDepartureReceipt, RoomError> {
        if subject.lease.role != PartyRole::Player
            || expected_epoch == 0
            || request_id.trim().is_empty()
            || request_id.len() > 64
            || request_id.chars().any(char::is_control)
        {
            return Err(RoomError::InvalidInput);
        }
        let _lane = ROOM_MUTATION.lock().await;
        self.rooms
            .leave_member(&subject.lease, expected_epoch, request_id)
            .await
            .map_err(|error| match error {
                crate::ports::repositories::RepositoryError::Operation {
                    code: "room_departure_conflict" | "room_causation_mismatch",
                    ..
                } => RoomError::Conflict,
                crate::ports::repositories::RepositoryError::Operation {
                    code: "room_access_denied",
                    ..
                } => RoomError::Denied,
                _ => RoomError::Repository,
            })
    }
    pub async fn revoke(&self, subject: &RoomSubject, member_id: Uuid) -> Result<(), RoomError> {
        let _lane = ROOM_MUTATION.lock().await;
        self.rooms
            .revoke_member(&subject.lease, member_id)
            .await
            .map_err(|_| RoomError::Denied)
    }
    pub async fn close(&self, subject: &RoomSubject) -> Result<(), RoomError> {
        let _lane = ROOM_MUTATION.lock().await;
        self.rooms
            .close_room(&subject.lease)
            .await
            .map_err(|_| RoomError::Denied)
    }
}
