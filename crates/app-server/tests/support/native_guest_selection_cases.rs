use super::*;
use adapter_http::services::NativeRoomClientHttpService;
use app_application::models::room_client::NativeGuestJoinRequest;

#[tokio::test]
async fn a_delayed_join_does_not_block_selection_in_an_independent_native_client() {
    let f = fixture().await;
    let fast_fixture = fixture().await;
    let host = f
        .rooms
        .authenticate(f.host.room.room_id, f.host.member_id, &f.host.capability)
        .await
        .unwrap();
    let invitation = f.rooms.rotate_invitation(&host).await.unwrap();
    let fast_host = fast_fixture
        .rooms
        .authenticate(
            fast_fixture.host.room.room_id,
            fast_fixture.host.member_id,
            &fast_fixture.host.capability,
        )
        .await
        .unwrap();
    let fast_invitation = fast_fixture
        .rooms
        .rotate_invitation(&fast_host)
        .await
        .unwrap();
    let started = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let started_server = started.clone();
    let release_server = release.clone();
    let delayed = adapter_http::rooms::router(f.rooms.clone()).layer(axum::middleware::from_fn(
        move |request: axum::extract::Request, next: axum::middleware::Next| {
            let started = started_server.clone();
            let release = release_server.clone();
            async move {
                let join = request.uri().path().ends_with("/join");
                let response = next.run(request).await;
                if join {
                    started.notify_one();
                    release.notified().await;
                }
                response
            }
        },
    ));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let slow_address = listener.local_addr().unwrap();
    let slow_server = tokio::spawn(async move { axum::serve(listener, delayed).await.unwrap() });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let fast_address = listener.local_addr().unwrap();
    let router = adapter_http::rooms::router(fast_fixture.rooms.clone());
    let fast_server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    async fn client() -> Arc<dyn NativeRoomClientHttpService> {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        pool::migrate(&db).await.unwrap();
        let state = app_server::AppState::new(Arc::new(Provider::default()), "fixture".into(), db);
        state.set_secrets_repo(Arc::new(InMemorySecretsStore::default()));
        app_server::native_room_client::service(state).unwrap()
    }
    let slow = client().await;
    let fast = client().await;
    let room_id = f.host.room.room_id;
    let request = NativeGuestJoinRequest {
        address: slow_address,
        room_id,
        join_id: Uuid::new_v4(),
        display_name: "Slow".into(),
        invitation: invitation.expose().into(),
    };
    let slow_join = tokio::spawn(async move { slow.join(request).await });
    tokio::time::timeout(std::time::Duration::from_secs(3), started.notified())
        .await
        .unwrap();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        fast.join(NativeGuestJoinRequest {
            address: fast_address,
            room_id: fast_fixture.host.room.room_id,
            join_id: Uuid::new_v4(),
            display_name: "Fast".into(),
            invitation: fast_invitation.expose().into(),
        }),
    )
    .await;
    release.notify_one();
    slow_join.await.unwrap().unwrap();
    slow_server.abort();
    fast_server.abort();
    assert!(
        result.is_ok(),
        "a separate vault must not wait for another client's remote ACK"
    );
    result.unwrap().unwrap();
}
