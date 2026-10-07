use adapter_sqlite::{pool, SqliteStore};
use app_application::{
    campaigns::{CreateCampaign, CreateCampaignCommand},
    models::{
        campaign_state::CommitCampaignCharacter,
        room_host::{NativeHostRoomGrant, NativeHostRoomRequest},
        session::CampaignSeed,
    },
    ports::{repositories::CampaignStateRepository, room_listener::RoomListenerId},
};
use app_server::AppState;
use std::sync::Arc;
#[tokio::test]
async fn native_owner_opens_a_real_scoped_room_and_rejects_other_callers() {
    let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&db).await.unwrap();
    let store = Arc::new(SqliteStore::new(db.clone()));
    let campaign = CreateCampaign::new(store.clone())
        .execute(CreateCampaignCommand {
            title: "Native host".into(),
            premise: "Road".into(),
            seed: CampaignSeed::Collaborative {
                answers: Default::default(),
            },
        })
        .await
        .unwrap();
    let current=store.commit_character(CommitCampaignCharacter {campaign_id:campaign.id,session_id:campaign.last_session_id,character_id:RoomListenerId::new_v4(),expected_revision:campaign.revision,causation_id:"hero".into(),character:serde_json::json!({"name":"Mira","heroClass":"fighter","level":1,"hp":12,"hpMax":12,"ac":14,"abilities":{"str":15,"dex":14,"con":13,"int":12,"wis":10,"cha":8},"inventory":[]})}).await.unwrap();
    let state = AppState::new(
        Arc::new(app_llm::MockProvider::new(vec![])),
        "fixture".into(),
        db,
    );
    let token = format!(
        "{}{}",
        RoomListenerId::new_v4().simple(),
        RoomListenerId::new_v4().simple()
    );
    let router =
        app_bootstrap::wiring::build_native_room_owner_router(state.clone(), token.clone())
            .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = reqwest::Client::new();
    let url = format!("http://{address}/native/rooms/host");
    let input = NativeHostRoomRequest {
        request_id: RoomListenerId::new_v4(),
        campaign_id: campaign.id,
        session_id: campaign.last_session_id,
        expected_revision: current.campaign.revision,
        bind_addr: "127.0.0.1:0".parse().unwrap(),
    };
    assert_eq!(
        client
            .post(&url)
            .json(&input)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(format!(
                "{}{}",
                RoomListenerId::new_v4().simple(),
                RoomListenerId::new_v4().simple()
            ))
            .json(&input)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(&token)
            .header("authorization", format!("Bearer {token}"))
            .json(&input)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    let mut forged = serde_json::to_value(&input).unwrap();
    forged["model"] = serde_json::json!("forged");
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(&token)
            .json(&forged)
            .send()
            .await
            .unwrap()
            .status(),
        422
    );
    let response = client
        .post(&url)
        .bearer_auth(&token)
        .json(&input)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let grant: NativeHostRoomGrant = response.json().await.unwrap();
    assert!(grant.capability != token);
    assert!(state
        .secrets_repo()
        .get("native:host_room_request")
        .await
        .unwrap()
        .is_some());
    let guest_db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&guest_db).await.unwrap();
    let guest_state = AppState::new(
        Arc::new(app_llm::MockProvider::new(vec![])),
        "fixture".into(),
        guest_db,
    );
    let guest_token = format!(
        "{}{}",
        RoomListenerId::new_v4().simple(),
        RoomListenerId::new_v4().simple()
    );
    let guest_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let guest_address = guest_listener.local_addr().unwrap();
    let guest_router = app_bootstrap::wiring::build_native_room_owner_router(
        guest_state.clone(),
        guest_token.clone(),
    )
    .unwrap();
    let guest_server =
        tokio::spawn(async move { axum::serve(guest_listener, guest_router).await.unwrap() });
    let guest_join = app_application::models::room_client::NativeGuestJoinRequest {
        address: grant.address,
        room_id: grant.room.room_id,
        join_id: RoomListenerId::new_v4(),
        display_name: "Tara".into(),
        invitation: grant.invitation.clone(),
    };
    let guest_url = format!("http://{guest_address}/native/room-client/join");
    assert_eq!(
        client
            .post(&guest_url)
            .json(&guest_join)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    let response = client
        .post(&guest_url)
        .bearer_auth(&guest_token)
        .json(&guest_join)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let joined: app_application::models::room_client::NativeGuestConnection =
        response.json().await.unwrap();
    assert_eq!(joined.room.members.len(), 2);
    let again: app_application::models::room_client::NativeGuestConnection = client
        .post(&guest_url)
        .bearer_auth(&guest_token)
        .json(&guest_join)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(again.member_id, joined.member_id);
    assert_eq!(again.room.members.len(), 2);
    let stored = guest_state
        .secrets_repo()
        .get(&format!("native:guest-join:{}", guest_join.join_id))
        .await
        .unwrap()
        .unwrap();
    let stored: app_application::models::room_client::StoredNativeGuestJoin =
        serde_json::from_str(&stored).unwrap();
    assert!(stored.connection.is_some());
    assert!(app_application::rooms::RoomSecret::from_input(stored.resume_proof).is_ok());
    let restarted = app_server::native_room_client::service(guest_state.clone()).unwrap();
    let resumed = restarted
        .resume(
            app_application::models::room_client::NativeGuestResumeRequest {
                room_id: grant.room.room_id,
                join_id: guest_join.join_id,
            },
        )
        .await
        .unwrap();
    assert_eq!(resumed.member_id, joined.member_id);
    let mut changed = guest_join.clone();
    changed.display_name = "Changed".into();
    assert_eq!(
        client
            .post(&guest_url)
            .bearer_auth(&guest_token)
            .json(&changed)
            .send()
            .await
            .unwrap()
            .status(),
        409
    );
    guest_server.abort();
    let repeated: NativeHostRoomGrant = client
        .post(&url)
        .bearer_auth(&token)
        .json(&input)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(repeated.address, grant.address);
    assert_eq!(repeated.room.room_id, grant.room.room_id);
    let public = format!("http://{}/rooms/{}", grant.address, grant.room.room_id);
    assert_eq!(client.get(&public).send().await.unwrap().status(), 401);
    let current = client
        .get(&public)
        .bearer_auth(&grant.capability)
        .header("x-room-member-id", grant.member_id.to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(current.status(), 200);
    assert_eq!(
        client
            .post(format!("http://{}/native/rooms/host", grant.address))
            .bearer_auth(&token)
            .json(&input)
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    assert_eq!(
        client
            .delete(format!(
                "http://{address}/native/rooms/{}",
                RoomListenerId::new_v4()
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .status(),
        409
    );
    assert_eq!(
        client
            .delete(format!(
                "http://{address}/native/rooms/{}",
                grant.room.room_id
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .status(),
        204
    );
    assert!(tokio::net::TcpStream::connect(grant.address).await.is_err());
    assert!(state
        .secrets_repo()
        .get("native:host_room_request")
        .await
        .unwrap()
        .is_none());
    assert_eq!(
        client
            .delete(format!(
                "http://{address}/native/rooms/{}",
                grant.room.room_id
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .status(),
        204
    );
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(&token)
            .json(&input)
            .send()
            .await
            .unwrap()
            .status(),
        409
    );
    let mut changed = input.clone();
    changed.expected_revision = store
        .load_current(campaign.id)
        .await
        .unwrap()
        .campaign
        .revision;
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(&token)
            .json(&changed)
            .send()
            .await
            .unwrap()
            .status(),
        409
    );
    changed.request_id = RoomListenerId::new_v4();
    let fresh = client
        .post(&url)
        .bearer_auth(&token)
        .json(&changed)
        .send()
        .await
        .unwrap();
    assert_eq!(fresh.status(), 200);
    let fresh: NativeHostRoomGrant = fresh.json().await.unwrap();
    assert_eq!(
        client
            .delete(format!(
                "http://{address}/native/rooms/{}",
                fresh.room.room_id
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .status(),
        204
    );
    server.abort();
}

#[tokio::test]
async fn occupied_port_closes_partial_room_and_does_not_publish_later_solo_chat() {
    use app_application::ports::repositories::MessageRepository;
    let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&db).await.unwrap();
    let store = Arc::new(SqliteStore::new(db.clone()));
    let campaign = CreateCampaign::new(store.clone())
        .execute(CreateCampaignCommand {
            title: "Failed native host".into(),
            premise: "Road".into(),
            seed: CampaignSeed::Collaborative {
                answers: Default::default(),
            },
        })
        .await
        .unwrap();
    let current=store.commit_character(CommitCampaignCharacter {campaign_id:campaign.id,session_id:campaign.last_session_id,character_id:RoomListenerId::new_v4(),expected_revision:campaign.revision,causation_id:"hero".into(),character:serde_json::json!({"name":"Mira","heroClass":"fighter","level":1,"hp":12,"hpMax":12,"ac":14,"abilities":{"str":15,"dex":14,"con":13,"int":12,"wis":10,"cha":8},"inventory":[]})}).await.unwrap();
    let state = AppState::new(
        Arc::new(app_llm::MockProvider::new(vec![])),
        "fixture".into(),
        db,
    );
    let token = format!(
        "{}{}",
        RoomListenerId::new_v4().simple(),
        RoomListenerId::new_v4().simple()
    );
    let owner = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let owner_address = owner.local_addr().unwrap();
    let router =
        app_bootstrap::wiring::build_native_room_owner_router(state.clone(), token.clone())
            .unwrap();
    let server = tokio::spawn(async move { axum::serve(owner, router).await.unwrap() });
    let occupied = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut input = NativeHostRoomRequest {
        request_id: RoomListenerId::new_v4(),
        campaign_id: campaign.id,
        session_id: campaign.last_session_id,
        expected_revision: current.campaign.revision,
        bind_addr: occupied.local_addr().unwrap(),
    };
    let client = reqwest::Client::new();
    let url = format!("http://{owner_address}/native/rooms/host");
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(&token)
            .json(&input)
            .send()
            .await
            .unwrap()
            .status(),
        409
    );
    let active: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM coop_rooms WHERE active=1")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(active, 0);
    store
        .append(
            campaign.last_session_id.0,
            app_application::models::chat::ChatMessage::user_text("PRIVATE_AFTER_FAILED_HOST"),
        )
        .await
        .unwrap();
    let stopped = input.clone();
    drop(occupied);
    input.request_id = RoomListenerId::new_v4();
    input.bind_addr = "127.0.0.1:0".parse().unwrap();
    input.expected_revision = store
        .load_current(campaign.id)
        .await
        .unwrap()
        .campaign
        .revision;
    let response = client
        .post(&url)
        .bearer_auth(&token)
        .json(&input)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let grant: NativeHostRoomGrant = response.json().await.unwrap();
    let snapshot = client
        .get(format!(
            "http://{}/rooms/{}/state",
            grant.address, grant.room.room_id
        ))
        .bearer_auth(&grant.capability)
        .header("x-room-member-id", grant.member_id.to_string())
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(!snapshot.contains("PRIVATE_AFTER_FAILED_HOST"));
    assert_eq!(
        client
            .delete(format!(
                "http://{owner_address}/native/rooms/{}",
                grant.room.room_id
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .status(),
        204
    );
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(&token)
            .json(&stopped)
            .send()
            .await
            .unwrap()
            .status(),
        409
    );
    server.abort();
}
