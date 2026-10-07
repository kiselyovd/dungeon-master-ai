use super::{RoomError, RoomService, RoomSubject, ROOM_MUTATION};
use crate::{
    models::{party::PartyRole, room_character::*},
    ports::repositories::RepositoryError,
};

pub(super) fn repository_error(error: RepositoryError) -> RoomError {
    match error {
        RepositoryError::Operation {
            code:
                "adventure_action_unavailable"
                | "adventure_combat_active"
                | "adventure_character_downed"
                | "rest_unsafe"
                | "rest_character_downed"
                | "rest_too_soon"
                | "hero_aid_invalid"
                | "hero_recovery_not_needed"
                | "hero_recovery_not_scheduled"
                | "recovery_in_combat",
            ..
        } => RoomError::ActionRejected,
        RepositoryError::Operation {
            code: "adventure_causation_mismatch" | "room_world_conflict",
            ..
        } => RoomError::Conflict,
        RepositoryError::Operation {
            code: "rest_intention_invalid",
            ..
        } => RoomError::InvalidInput,
        RepositoryError::Operation {
            code: "room_action_rejected",
            ..
        } => RoomError::ActionRejected,
        RepositoryError::Operation {
            code: "room_action_invalid",
            ..
        } => RoomError::InvalidInput,
        RepositoryError::Operation {
            code: "room_combat_conflict" | "combat_request_mismatch",
            ..
        } => RoomError::Conflict,
        RepositoryError::Conflict { .. }
        | RepositoryError::RevisionConflict { .. }
        | RepositoryError::NotFound => RoomError::Conflict,
        RepositoryError::Operation {
            code: "room_access_denied" | "room_host_required",
            ..
        } => RoomError::Denied,
        RepositoryError::Operation {
            code: "room_character_invalid" | "party_character_invalid",
            ..
        } => RoomError::InvalidInput,
        RepositoryError::Operation {
            code:
                "room_character_conflict"
                | "party_combat_active"
                | "party_binding_rejected"
                | "party_character_already_enrolled"
                | "party_causation_mismatch",
            ..
        } => RoomError::Conflict,
        _ => RoomError::Repository,
    }
}
impl RoomService {
    pub async fn submit_character(
        &self,
        subject: &RoomSubject,
        mut command: SubmitRoomCharacter,
    ) -> Result<ProposalSummary, RoomError> {
        let _lane = ROOM_MUTATION.lock().await;
        self.authorized_room(subject).await?;
        if subject.lease.role != PartyRole::Player {
            return Err(RoomError::Denied);
        }
        if command.proposal_id.is_nil()
            || command.character_id.is_nil()
            || !crate::models::campaign_state::valid_character_document(&command.character)
            || !crate::models::campaign_state::within_character_budget(&command.character)
            || command.character.get("characterId").is_some_and(|value| {
                value.as_str().and_then(|id| uuid::Uuid::parse_str(id).ok())
                    != Some(command.character_id)
            })
        {
            return Err(RoomError::InvalidInput);
        }
        command.character["characterId"] = serde_json::json!(command.character_id);
        self.rooms
            .submit_character_proposal(&subject.lease, command)
            .await
            .map_err(repository_error)
    }
    pub async fn own_proposal(
        &self,
        subject: &RoomSubject,
    ) -> Result<Option<OwnCharacterProposal>, RoomError> {
        let _lane = ROOM_MUTATION.lock().await;
        self.authorized_room(subject).await?;
        self.rooms
            .own_character_proposal(&subject.lease)
            .await
            .map_err(repository_error)
    }
    pub async fn pending_proposals(
        &self,
        subject: &RoomSubject,
    ) -> Result<Vec<ProposalSummary>, RoomError> {
        let _lane = ROOM_MUTATION.lock().await;
        self.authorized_room(subject).await?;
        if subject.lease.role != PartyRole::Host {
            return Err(RoomError::Denied);
        }
        self.rooms
            .pending_character_proposals(&subject.lease)
            .await
            .map_err(repository_error)
    }
    pub async fn approve_character(
        &self,
        subject: &RoomSubject,
        command: ApproveRoomCharacter,
    ) -> Result<CharacterApproval, RoomError> {
        let _lane = ROOM_MUTATION.lock().await;
        self.authorized_room(subject).await?;
        if subject.lease.role != PartyRole::Host {
            return Err(RoomError::Denied);
        }
        if command.proposal_id.is_nil()
            || command.causation_id.trim().is_empty()
            || command.causation_id.len() > 64
        {
            return Err(RoomError::InvalidInput);
        }
        self.rooms
            .approve_character_proposal(&subject.lease, command)
            .await
            .map_err(repository_error)
    }
}
