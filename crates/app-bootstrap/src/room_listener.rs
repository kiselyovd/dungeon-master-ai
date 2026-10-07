//! Room sockets run inside the existing backend child; Tauri remains its process owner.
use app_application::{
    ports::room_listener::{
        RoomListenerControl, RoomListenerError, RoomListenerFuture, RoomListenerId,
        RoomListenerSpec,
    },
    rooms::RoomService,
};
use app_server::AppState;
use std::{
    future::Future,
    io,
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::TcpListener,
    sync::{oneshot, watch, Mutex},
    task::JoinHandle,
};
struct HostedListener {
    spec: RoomListenerSpec,
    address: SocketAddr,
    shutdown: oneshot::Sender<()>,
    connections: watch::Sender<bool>,
    task: JoinHandle<()>,
}
pub struct RoomTcpListener {
    state: AppState,
    rooms: Arc<RoomService>,
    hosted: Mutex<Option<HostedListener>>,
}
impl RoomTcpListener {
    pub fn new(state: AppState, rooms: Arc<RoomService>) -> Self {
        Self {
            state,
            rooms,
            hosted: Mutex::new(None),
        }
    }
}
impl RoomListenerControl for RoomTcpListener {
    fn start(&self, spec: RoomListenerSpec) -> RoomListenerFuture<'_, SocketAddr> {
        Box::pin(async move {
            if spec.room_id.is_nil() || spec.bind_addr.ip().is_multicast() {
                return Err(RoomListenerError {
                    code: "room_listener_invalid",
                });
            }
            let mut hosted = self.hosted.lock().await;
            if let Some(current) = hosted.as_ref() {
                if !current.task.is_finished() {
                    if current.spec == spec {
                        return Ok(current.address);
                    }
                    return Err(RoomListenerError {
                        code: "room_listener_busy",
                    });
                }
            }
            let listener =
                TcpListener::bind(spec.bind_addr)
                    .await
                    .map_err(|_| RoomListenerError {
                        code: "room_listener_bind_failed",
                    })?;
            let address = listener.local_addr().map_err(|_| RoomListenerError {
                code: "room_listener_address_failed",
            })?;
            let dm = app_server::http_services::room_dm_http_service(
                self.state.clone(),
                self.rooms.clone(),
            );
            let router = adapter_http::rooms::router_for_room(self.rooms.clone(), dm, spec.room_id);
            let (shutdown, stopped) = oneshot::channel();
            let (connections, cancelled) = watch::channel(false);
            let listener = CancelableListener {
                inner: listener,
                cancelled,
            };
            let task = tokio::spawn(async move {
                if axum::serve(listener, router)
                    .with_graceful_shutdown(async {
                        let _ = stopped.await;
                    })
                    .await
                    .is_err()
                {
                    tracing::error!(code = "room_listener_serve_failed");
                }
            });
            *hosted = Some(HostedListener {
                spec,
                address,
                shutdown,
                connections,
                task,
            });
            tracing::info!(process_role="backend",room_id=%spec.room_id,%address,"room listener started");
            Ok(address)
        })
    }
    fn stop(&self, room_id: RoomListenerId) -> RoomListenerFuture<'_, ()> {
        Box::pin(async move {
            let mut hosted = self.hosted.lock().await;
            let Some(current) = hosted.as_ref() else {
                return Ok(());
            };
            if current.spec.room_id != room_id {
                return Err(RoomListenerError {
                    code: "room_listener_scope_changed",
                });
            }
            let mut current = hosted.take().expect("room listener checked above");
            let _ = current.connections.send(true);
            let _ = current.shutdown.send(());
            if tokio::time::timeout(std::time::Duration::from_secs(2), &mut current.task)
                .await
                .is_err()
            {
                current.task.abort();
            }
            tracing::info!(process_role="backend",%room_id,"room listener stopped");
            Ok(())
        })
    }
}

/// Axum spawns connection tasks independently; each accepted I/O observes stop.
struct CancelableListener {
    inner: TcpListener,
    cancelled: watch::Receiver<bool>,
}
impl axum::serve::Listener for CancelableListener {
    type Io = CancelableIo;
    type Addr = SocketAddr;
    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        let (inner, address) = axum::serve::Listener::accept(&mut self.inner).await;
        let mut signal = self.cancelled.clone();
        let stopped = Box::pin(async move {
            loop {
                if *signal.borrow() {
                    break;
                }
                if signal.changed().await.is_err() {
                    break;
                }
            }
        });
        (
            CancelableIo {
                inner,
                stopped,
                closed: false,
            },
            address,
        )
    }
    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.inner.local_addr()
    }
}
struct CancelableIo {
    inner: tokio::net::TcpStream,
    stopped: Pin<Box<dyn Future<Output = ()> + Send>>,
    closed: bool,
}
impl CancelableIo {
    fn cancelled(&mut self, cx: &mut Context<'_>) -> bool {
        if !self.closed && self.stopped.as_mut().poll(cx).is_ready() {
            self.closed = true;
        }
        self.closed
    }
    fn stopped_error() -> io::Error {
        io::Error::new(io::ErrorKind::ConnectionAborted, "room_listener_stopped")
    }
}
impl AsyncRead for CancelableIo {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let current = self.get_mut();
        if current.cancelled(cx) {
            return Poll::Ready(Err(Self::stopped_error()));
        }
        Pin::new(&mut current.inner).poll_read(cx, buffer)
    }
}
impl AsyncWrite for CancelableIo {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let current = self.get_mut();
        if current.cancelled(cx) {
            return Poll::Ready(Err(Self::stopped_error()));
        }
        Pin::new(&mut current.inner).poll_write(cx, bytes)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let current = self.get_mut();
        if current.cancelled(cx) {
            return Poll::Ready(Err(Self::stopped_error()));
        }
        Pin::new(&mut current.inner).poll_flush(cx)
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapter_sqlite::{pool, SqliteStore};
    use app_application::{
        campaigns::{CreateCampaign, CreateCampaignCommand},
        models::{
            campaign_state::CommitCampaignCharacter, room::CreateLocalRoom, session::CampaignSeed,
        },
        ports::repositories::CampaignStateRepository,
        rooms::{CreatedRoom, SystemRoomClock},
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    async fn create(store: Arc<SqliteStore>, rooms: &RoomService, title: &str) -> CreatedRoom {
        let campaign = CreateCampaign::new(store.clone())
            .execute(CreateCampaignCommand {
                title: title.into(),
                premise: "Road".into(),
                seed: CampaignSeed::Collaborative {
                    answers: Default::default(),
                },
            })
            .await
            .unwrap();
        let character=r#"{"name":"Mira","heroClass":"fighter","level":1,"hp":12,"hpMax":12,"ac":14,"abilities":{"str":15,"dex":14,"con":13,"int":12,"wis":10,"cha":8},"inventory":[]}"#.parse().unwrap();
        let current = store
            .commit_character(CommitCampaignCharacter {
                campaign_id: campaign.id,
                session_id: campaign.last_session_id,
                character_id: RoomListenerId::new_v4(),
                expected_revision: campaign.revision,
                causation_id: "hero".into(),
                character,
            })
            .await
            .unwrap();
        rooms
            .create_local_room(CreateLocalRoom {
                campaign_id: campaign.id,
                session_id: campaign.last_session_id,
                expected_revision: current.campaign.revision,
                causation_id: "room".into(),
            })
            .await
            .unwrap()
    }
    async fn get(
        address: SocketAddr,
        path: &str,
        grant: Option<&app_application::rooms::RoomGrant>,
    ) -> String {
        let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
        let auth = grant
            .map(|grant| {
                format!(
                    "Authorization: Bearer {}\r\nx-room-member-id: {}\r\n",
                    grant.capability.expose(),
                    grant.member_id
                )
            })
            .unwrap_or_default();
        let request =
            format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n{auth}Connection: close\r\n\r\n");
        socket.write_all(request.as_bytes()).await.unwrap();
        let mut bytes = Vec::new();
        tokio::time::timeout(
            std::time::Duration::from_secs(3),
            socket.read_to_end(&mut bytes),
        )
        .await
        .unwrap()
        .unwrap();
        String::from_utf8(bytes).unwrap()
    }
    #[tokio::test]
    async fn real_room_listener_is_scoped_idempotent_and_never_exposes_local_admin() {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        pool::migrate(&db).await.unwrap();
        let store = Arc::new(SqliteStore::new(db.clone()));
        let state = AppState::new(
            Arc::new(app_llm::MockProvider::new(vec![])),
            "fixture".into(),
            db,
        );
        let rooms = Arc::new(RoomService::new(
            store.clone(),
            store.clone(),
            state.secrets_repo(),
            Arc::new(SystemRoomClock),
        ));
        let first = create(store.clone(), &rooms, "First").await;
        let second = create(store, &rooms, "Second").await;
        let native = RoomTcpListener::new(state, rooms.clone());
        let spec = RoomListenerSpec {
            room_id: first.grant.room.room_id,
            bind_addr: "127.0.0.1:0".parse().unwrap(),
        };
        let address = native.start(spec).await.unwrap();
        assert_eq!(address, native.start(spec).await.unwrap());
        let own_path = format!("/rooms/{}", first.grant.room.room_id);
        assert!(get(address, &own_path, None)
            .await
            .starts_with("HTTP/1.1 401"));
        let response = get(address, &own_path, Some(&first.grant)).await;
        assert!(response.starts_with("HTTP/1.1 200"));
        assert!(response.contains("cache-control: no-store"));
        let other_path = format!("/rooms/{}", second.grant.room.room_id);
        assert!(get(address, &other_path, Some(&second.grant))
            .await
            .starts_with("HTTP/1.1 404"));
        for path in [
            "/health",
            "/settings/v2",
            "/providers",
            "/campaigns",
            "/local-llm/status",
        ] {
            assert!(get(address, path, Some(&first.grant))
                .await
                .starts_with("HTTP/1.1 404"));
        }
        assert!(native
            .start(RoomListenerSpec {
                room_id: second.grant.room.room_id,
                ..spec
            })
            .await
            .is_err());
        assert!(native.stop(second.grant.room.room_id).await.is_err());
        let mut slow = tokio::net::TcpStream::connect(address).await.unwrap();
        let partial=format!("POST /rooms/{}/character-proposals HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {}\r\nx-room-member-id: {}\r\nContent-Type: application/json\r\nContent-Length: 1000\r\nExpect: 100-continue\r\nConnection: keep-alive\r\n\r\n{{",first.grant.room.room_id,first.grant.capability.expose(),first.grant.member_id);
        slow.write_all(partial.as_bytes()).await.unwrap();
        let mut accepted = Vec::new();
        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            while !accepted.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                let mut chunk = [0; 128];
                let count = slow.read(&mut chunk).await.unwrap();
                assert!(count > 0);
                accepted.extend_from_slice(&chunk[..count]);
            }
        })
        .await
        .unwrap();
        assert!(accepted.starts_with(b"HTTP/1.1 100"));
        let host = rooms
            .authenticate(
                first.grant.room.room_id,
                first.grant.member_id,
                &first.grant.capability,
            )
            .await
            .unwrap();
        rooms.close(&host).await.unwrap();
        native.stop(first.grant.room.room_id).await.unwrap();
        assert!(tokio::net::TcpStream::connect(address).await.is_err());
        let mut remaining = Vec::new();
        let closed = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            slow.read_to_end(&mut remaining),
        )
        .await
        .expect("accepted partial-body socket must close within the stop deadline");
        if let Err(error) = closed {
            assert!(matches!(
                error.kind(),
                std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::BrokenPipe
            ));
        }

        native.stop(first.grant.room.room_id).await.unwrap();
    }
}
