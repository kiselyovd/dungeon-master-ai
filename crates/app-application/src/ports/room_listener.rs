//! Native-controlled room transport lifecycle, separate from the privileged local API.
use std::{future::Future, net::SocketAddr, pin::Pin};
use uuid::Uuid;
pub type RoomListenerId = Uuid;
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RoomListenerSpec {
    pub room_id: RoomListenerId,
    pub bind_addr: SocketAddr,
}
#[derive(Debug, thiserror::Error)]
#[error("room listener failed with code {code}")]
pub struct RoomListenerError {
    pub code: &'static str,
}
pub type RoomListenerFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, RoomListenerError>> + Send + 'a>>;
/// Only trusted native host composition invokes this port; it is not a player API.
pub trait RoomListenerControl: Send + Sync {
    fn start(&self, spec: RoomListenerSpec) -> RoomListenerFuture<'_, SocketAddr>;
    /// The host use case revokes/closes the room before stopping transport.
    fn stop(&self, room_id: RoomListenerId) -> RoomListenerFuture<'_, ()>;
}
