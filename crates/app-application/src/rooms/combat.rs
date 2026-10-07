use super::{characters::repository_error, RoomError, RoomService, RoomSubject, ROOM_MUTATION};
use crate::models::{combat::CombatProjection, room_combat::*};
use uuid::Uuid;

impl RoomService {
    async fn combat_owner(&self, subject: &RoomSubject) -> Result<Uuid, RoomError> {
        let room = self.authorized_room(subject).await?;
        let party = self
            .parties
            .party(room.campaign_id)
            .await
            .map_err(repository_error)?
            .ok_or(RoomError::Denied)?;
        if party.session_id != room.session_id {
            return Err(RoomError::Denied);
        }
        party
            .roster
            .members
            .iter()
            .find(|member| {
                member.member_id == subject.lease.member_id && member.role == subject.lease.role
            })
            .map(|member| member.character_id)
            .ok_or(RoomError::Denied)
    }
    pub async fn room_combat(
        &self,
        subject: &RoomSubject,
        encounter: Uuid,
    ) -> Result<CombatProjection, RoomError> {
        let _lane = ROOM_MUTATION.lock().await;
        let owner = self.combat_owner(subject).await?;
        let mut projection = self
            .rooms
            .room_combat(&subject.lease, encounter)
            .await
            .map_err(repository_error)?;
        project_for_owner(&mut projection, owner);
        Ok(projection)
    }
    pub async fn act_combat(
        &self,
        subject: &RoomSubject,
        encounter: Uuid,
        command: RoomCombatCommand,
    ) -> Result<RoomCombatResult, RoomError> {
        let _lane = ROOM_MUTATION.lock().await;
        let owner = self.combat_owner(subject).await?;
        if encounter.is_nil()
            || command.request_id.trim().is_empty()
            || command.request_id.len() > 64
            || command.request_id.chars().any(char::is_control)
        {
            return Err(RoomError::InvalidInput);
        }
        let mut result = self
            .rooms
            .act_room_combat(
                &subject.lease,
                encounter,
                command,
                Uuid::new_v4().as_u128() as u64,
            )
            .await
            .map_err(repository_error)?;
        project_for_owner(&mut result.projection, owner);
        Ok(result)
    }
}
