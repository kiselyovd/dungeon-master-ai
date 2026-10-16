use super::{characters::repository_error, RoomError, RoomService, RoomSubject, ROOM_MUTATION};
use crate::models::room_state::RoomUpdate;
impl RoomService {
    pub async fn updates(
        &self,
        subject: &RoomSubject,
        after: Option<u64>,
    ) -> Result<Option<RoomUpdate>, RoomError> {
        let _lane = ROOM_MUTATION.lock().await;
        self.authorized_room(subject).await?;
        self.rooms
            .room_updates(&subject.lease, after, false)
            .await
            .map_err(repository_error)
    }
    pub async fn initial_updates(
        &self,
        subject: &RoomSubject,
        after: Option<u64>,
    ) -> Result<RoomUpdate, RoomError> {
        let _lane = ROOM_MUTATION.lock().await;
        self.authorized_room(subject).await?;
        self.rooms
            .room_updates(&subject.lease, after, true)
            .await
            .map_err(repository_error)?
            .ok_or(RoomError::Repository)
    }
}
