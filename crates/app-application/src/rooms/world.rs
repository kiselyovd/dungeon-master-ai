use super::{characters::repository_error, RoomError, RoomService, RoomSubject, ROOM_MUTATION};
use crate::models::{rest::RestStatus, room_world::*};
fn valid_nonce(id: &str) -> bool {
    !id.trim().is_empty() && id.len() <= 64 && !id.chars().any(char::is_control)
}
impl RoomService {
    pub async fn recover_hero(
        &self,
        subject: &RoomSubject,
        command: crate::models::room_recovery::RoomRecoveryCommand,
    ) -> Result<crate::models::room_recovery::RoomRecoveryReceipt, RoomError> {
        let _lane = ROOM_MUTATION.lock().await;
        self.authorized_room(subject).await?;
        if !valid_nonce(&command.request_id) {
            return Err(RoomError::InvalidInput);
        }
        self.rooms
            .recover_room_hero(
                &subject.lease,
                command,
                uuid::Uuid::new_v4().as_u128() as u64,
            )
            .await
            .map_err(repository_error)
    }
    pub async fn adventure(
        &self,
        subject: &RoomSubject,
        command: RoomAdventureCommand,
    ) -> Result<RoomWorldReceipt, RoomError> {
        let _lane = ROOM_MUTATION.lock().await;
        self.authorized_room(subject).await?;
        if !valid_nonce(&command.request_id) {
            return Err(RoomError::InvalidInput);
        }
        self.rooms
            .act_room_adventure(&subject.lease, command)
            .await
            .map_err(repository_error)
    }
    pub async fn rest_status(&self, subject: &RoomSubject) -> Result<RestStatus, RoomError> {
        let _lane = ROOM_MUTATION.lock().await;
        self.authorized_room(subject).await?;
        self.rooms
            .room_rest_status(&subject.lease)
            .await
            .map_err(repository_error)
    }
    pub async fn rest(
        &self,
        subject: &RoomSubject,
        command: RoomRestCommand,
    ) -> Result<RoomWorldReceipt, RoomError> {
        let _lane = ROOM_MUTATION.lock().await;
        self.authorized_room(subject).await?;
        if !valid_nonce(&command.request_id) {
            return Err(RoomError::InvalidInput);
        }
        self.rooms
            .rest_room(
                &subject.lease,
                command,
                uuid::Uuid::new_v4().as_u128() as u64,
            )
            .await
            .map_err(repository_error)
    }
}
