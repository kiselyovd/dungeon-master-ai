use adapter_secrets::{InMemorySecretsStore, StrongholdSecretsStore};
use adapter_sqlite::{pool, SqliteStore};
use app_application::{
    campaigns::{CreateCampaign, CreateCampaignCommand},
    models::{
        campaign_state::CommitCampaignCharacter,
        party::{EnrollPartyCharacter, InitializeParty},
        room::CreateLocalRoom,
        session::CampaignSeed,
    },
    ports::repositories::{CampaignStateRepository, PartyRepository, RoomRepository},
    rooms::{JoinRoom, RoomClock, RoomOperation, RoomSecret, RoomService},
};
use serde_json::json;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use uuid::Uuid;
struct CountingSecrets {
    inner: InMemorySecretsStore,
    writes: std::sync::atomic::AtomicUsize,
}
#[async_trait::async_trait]
impl app_application::ports::secrets::SecretsStore for CountingSecrets {
    async fn get(
        &self,
        key: &str,
    ) -> Result<Option<String>, app_application::ports::secrets::SecretsError> {
        self.inner.get(key).await
    }
    async fn set(
        &self,
        key: &str,
        value: &str,
    ) -> Result<(), app_application::ports::secrets::SecretsError> {
        self.writes.fetch_add(1, Ordering::SeqCst);
        self.inner.set(key, value).await
    }
    async fn delete(&self, key: &str) -> Result<(), app_application::ports::secrets::SecretsError> {
        self.inner.delete(key).await
    }
}
struct Clock(AtomicU64);
impl RoomClock for Clock {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
async fn fixture(db: sqlx::SqlitePool) -> (Arc<SqliteStore>, CreateLocalRoom) {
    pool::migrate(&db).await.unwrap();
    let store = Arc::new(SqliteStore::new(db));
    let campaign = CreateCampaign::new(store.clone())
        .execute(CreateCampaignCommand {
            title: "Room".into(),
            premise: "Two devices".into(),
            seed: CampaignSeed::Bundled {
                adventure_id: "bell-beneath-water".into(),
            },
        })
        .await
        .unwrap();
    let state=store.commit_character(CommitCampaignCharacter{campaign_id:campaign.id,session_id:campaign.last_session_id,character_id:Uuid::new_v4(),expected_revision:campaign.revision,causation_id:"host".into(),character:json!({"name":"Mira","heroClass":"fighter","level":1,"hp":12,"hpMax":12,"ac":14,"abilities":{"str":15,"dex":14,"con":13,"int":12,"wis":10,"cha":8},"inventory":[]})}).await.unwrap();
    (
        store,
        CreateLocalRoom {
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            expected_revision: state.campaign.revision,
            causation_id: "room".into(),
        },
    )
}
fn secret(value: &RoomSecret) -> RoomSecret {
    RoomSecret::from_input(value.expose().into()).unwrap()
}

#[tokio::test]
async fn actual_room_http_authenticates_identity_and_isolates_desktop_admin_routes() {
    let (store, mut command) =
        fixture(sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap()).await;
    let state = store.load_current(command.campaign_id).await.unwrap();
    let mut host_sheet = state.projection.character;
    host_sheet["backstory"] = json!("host-private-backstory");
    let state = store
        .commit_character(CommitCampaignCharacter {
            campaign_id: command.campaign_id,
            session_id: command.session_id,
            character_id: state.character_id.unwrap(),
            expected_revision: state.campaign.revision,
            causation_id: "host-private-fixture".into(),
            character: host_sheet,
        })
        .await
        .unwrap();
    command.expected_revision = state.campaign.revision;
    let service = Arc::new(RoomService::new(
        store.clone(),
        store.clone(),
        Arc::new(InMemorySecretsStore::default()),
        Arc::new(Clock(AtomicU64::new(1000))),
    ));
    let created = service.create_local_room(command).await.unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let router = adapter_http::rooms::router(service);
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let client = reqwest::Client::new();
    let url = format!("http://{address}/rooms/{}", created.grant.room.room_id);
    let missing = client.get(&url).send().await.unwrap();
    assert_eq!(missing.status(), 401);
    let host = client
        .get(&url)
        .bearer_auth(created.grant.capability.expose())
        .header("x-room-member-id", created.grant.member_id.to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(host.status(), 200);
    assert_eq!(host.headers()["cache-control"], "no-store");
    let host_json: serde_json::Value = host.json().await.unwrap();
    assert!(host_json.get("invitation").is_none());
    let proof = RoomSecret::generate();
    let join_id = Uuid::new_v4();
    let payload = json!({"join_id":join_id,"display_name":"Guest","invitation":created.invitation.expose(),"resume_proof":proof.expose()});
    let joined = client
        .post(format!("{url}/join"))
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(joined.status(), 200);
    assert_eq!(joined.headers()["cache-control"], "no-store");
    let joined: serde_json::Value = joined.json().await.unwrap();
    let member = joined["member_id"].as_str().unwrap();
    let capability = joined["capability"].as_str().unwrap();
    let replay = client
        .post(format!("{url}/join"))
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(replay.status(), 200);
    let replay: serde_json::Value = replay.json().await.unwrap();
    assert!(replay["capability"].as_str().unwrap() == capability);
    let stolen_replay = client.post(format!("{url}/join")).json(&json!({"join_id":join_id,"display_name":"Guest","invitation":created.invitation.expose(),"resume_proof":RoomSecret::generate().expose()})).send().await.unwrap();
    assert_eq!(stolen_replay.status(), 401);
    let pending = client
        .get(format!("{url}/party"))
        .bearer_auth(capability)
        .header("x-room-member-id", member)
        .send()
        .await
        .unwrap();
    assert_eq!(pending.status(), 200);
    assert!(pending.json::<serde_json::Value>().await.unwrap().is_null());
    let party = store
        .party(created.grant.room.campaign_id)
        .await
        .unwrap()
        .unwrap();
    let host_character = &party.characters[&party.roster.members[0].character_id];
    let mut character = host_character.clone();
    let character_id = Uuid::new_v4();
    character["characterId"] = json!(character_id);
    character["name"] = json!("Guest hero");
    character["backstory"] = json!("guest-private-backstory");
    store
        .enroll_party_character(EnrollPartyCharacter {
            campaign_id: party.campaign_id,
            session_id: party.session_id,
            expected_revision: party.revision,
            causation_id: "http-party-acceptance".into(),
            verified_host_member_id: party.roster.host_member_id,
            member_id: Uuid::parse_str(member).unwrap(),
            character_id,
            character,
        })
        .await
        .unwrap();
    let own = client
        .get(format!("{url}/party"))
        .bearer_auth(capability)
        .header("x-room-member-id", member)
        .send()
        .await
        .unwrap();
    assert_eq!(own.status(), 200);
    let own = own.text().await.unwrap();
    assert!(own.contains("guest-private-backstory"));
    assert!(!own.contains("host-private-backstory"));
    let host_view = client
        .get(format!("{url}/party"))
        .bearer_auth(created.grant.capability.expose())
        .header("x-room-member-id", created.grant.member_id.to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(host_view.status(), 200);
    assert!(!host_view
        .text()
        .await
        .unwrap()
        .contains("guest-private-backstory"));
    let oversized = client
        .post(format!("{url}/join"))
        .header("content-type", "application/json")
        .body(" ".repeat(8193))
        .send()
        .await
        .unwrap();
    assert_eq!(oversized.status(), 413);
    let duplicate_identity = client
        .get(&url)
        .bearer_auth(capability)
        .header("x-room-member-id", member)
        .header("x-room-member-id", created.grant.member_id.to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(duplicate_identity.status(), 401);
    let spoof = client
        .get(&url)
        .bearer_auth(capability)
        .header("x-room-member-id", created.grant.member_id.to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(spoof.status(), 401);
    let invitation = client
        .post(format!("{url}/invitation"))
        .bearer_auth(capability)
        .header("x-room-member-id", member)
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(invitation.status(), 401);
    let host_invitation = client
        .post(format!("{url}/invitation"))
        .bearer_auth(created.grant.capability.expose())
        .header("x-room-member-id", created.grant.member_id.to_string())
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(host_invitation.status(), 200);
    for path in [
        "/settings/v2",
        "/providers",
        "/campaigns",
        "/hf/token",
        "/saves",
        "/rooms",
    ] {
        assert_eq!(
            client
                .get(format!("http://{address}{path}"))
                .send()
                .await
                .unwrap()
                .status(),
            404
        );
    }
    let tampered = client.post(format!("{url}/join")).json(&json!({"join_id":join_id,"display_name":"Guest","invitation":created.invitation.expose(),"resume_proof":proof.expose(),"role":"Host"})).send().await.unwrap();
    assert_eq!(tampered.status(), 422);
    let revoked = client
        .delete(format!("{url}/members/{member}"))
        .bearer_auth(created.grant.capability.expose())
        .header("x-room-member-id", created.grant.member_id.to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(revoked.status(), 204);
    let guest = client
        .get(&url)
        .bearer_auth(capability)
        .header("x-room-member-id", member)
        .send()
        .await
        .unwrap();
    assert_eq!(guest.status(), 401);
    let closed = client
        .delete(&url)
        .bearer_auth(created.grant.capability.expose())
        .header("x-room-member-id", created.grant.member_id.to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(closed.status(), 204);
    let old_host = client
        .get(&url)
        .bearer_auth(created.grant.capability.expose())
        .header("x-room-member-id", created.grant.member_id.to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(old_host.status(), 401);
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn concurrent_service_instances_replay_the_same_private_join_binding() {
    let (store, command) =
        fixture(sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap()).await;
    let vault = Arc::new(InMemorySecretsStore::default());
    let clock = Arc::new(Clock(AtomicU64::new(1000)));
    let first = RoomService::new(store.clone(), store.clone(), vault.clone(), clock.clone());
    let second = RoomService::new(store.clone(), store.clone(), vault, clock);
    let created = first.create_local_room(command).await.unwrap();
    let proof = RoomSecret::generate();
    let join_id = Uuid::new_v4();
    let request = || JoinRoom {
        room_id: created.grant.room.room_id,
        join_id,
        display_name: "Guest".into(),
        invitation: secret(&created.invitation),
        resume_proof: secret(&proof),
    };
    let (left, right) = tokio::join!(first.join(request()), second.join(request()));
    let left = left.unwrap();
    let right = right.unwrap();
    assert_eq!(left.member_id, right.member_id);
    assert!(left.capability.expose() == right.capability.expose());
    assert_eq!(
        store
            .room(left.room.room_id)
            .await
            .unwrap()
            .unwrap()
            .members
            .len(),
        2
    );
    assert!(second
        .authenticate(right.room.room_id, right.member_id, &right.capability)
        .await
        .is_ok());
}

#[tokio::test]
async fn full_saved_party_recovers_member_identity_in_each_new_room() {
    let (store, mut command) =
        fixture(sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap()).await;
    let mut party = store
        .initialize_party(InitializeParty {
            campaign_id: command.campaign_id,
            session_id: command.session_id,
            expected_revision: command.expected_revision,
            causation_id: "saved-party".into(),
        })
        .await
        .unwrap();
    let mut guests = Vec::new();
    for index in 0..3 {
        let member_id = Uuid::new_v4();
        let character_id = Uuid::new_v4();
        let mut character = party.characters.values().next().unwrap().clone();
        character["characterId"] = json!(character_id);
        character["name"] = json!(format!("Guest {index}"));
        party = store
            .enroll_party_character(EnrollPartyCharacter {
                campaign_id: command.campaign_id,
                session_id: command.session_id,
                expected_revision: party.revision,
                causation_id: format!("guest-{index}"),
                verified_host_member_id: party.roster.host_member_id,
                member_id,
                character_id,
                character,
            })
            .await
            .unwrap();
        guests.push(member_id);
    }
    command.expected_revision = party.revision;
    let service = RoomService::new(
        store.clone(),
        store.clone(),
        Arc::new(InMemorySecretsStore::default()),
        Arc::new(Clock(AtomicU64::new(1000))),
    );
    for round in 0..2 {
        command.causation_id = format!("room-{round}");
        let created = service.create_local_room(command.clone()).await.unwrap();
        for member_id in &guests {
            let host = service
                .authenticate(
                    created.grant.room.room_id,
                    created.grant.member_id,
                    &created.grant.capability,
                )
                .await
                .unwrap();
            let invitation = service.invite_member(&host, *member_id).await.unwrap();
            let joined = service
                .join(JoinRoom {
                    room_id: created.grant.room.room_id,
                    join_id: Uuid::new_v4(),
                    display_name: "Returning player".into(),
                    invitation,
                    resume_proof: RoomSecret::generate(),
                })
                .await
                .unwrap();
            assert_eq!(joined.member_id, *member_id);
            assert!(service
                .authenticate(joined.room.room_id, *member_id, &joined.capability)
                .await
                .is_ok());
            assert_eq!(joined.room.members.len(), 4);
        }
        let host = service
            .authenticate(
                created.grant.room.room_id,
                created.grant.member_id,
                &created.grant.capability,
            )
            .await
            .unwrap();
        service.close(&host).await.unwrap();
    }
    assert_eq!(
        store
            .party(command.campaign_id)
            .await
            .unwrap()
            .unwrap()
            .roster,
        party.roster
    );
}

#[tokio::test]
async fn invitation_and_member_capabilities_are_distinct_and_public_metadata_has_no_secrets() {
    let (store, command) =
        fixture(sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap()).await;
    let clock = Arc::new(Clock(AtomicU64::new(1000)));
    let service = RoomService::new(
        store.clone(),
        store.clone(),
        Arc::new(InMemorySecretsStore::default()),
        clock,
    );
    let created = service.create_local_room(command.clone()).await.unwrap();
    let host = service
        .authenticate(
            created.grant.room.room_id,
            created.grant.member_id,
            &created.grant.capability,
        )
        .await
        .unwrap();
    let join_id = Uuid::new_v4();
    let proof = RoomSecret::generate();
    let joined = service
        .join(JoinRoom {
            room_id: created.grant.room.room_id,
            join_id,
            display_name: "Mira".into(),
            invitation: secret(&created.invitation),
            resume_proof: secret(&proof),
        })
        .await
        .unwrap();
    let peer = service
        .authenticate(joined.room.room_id, joined.member_id, &joined.capability)
        .await
        .unwrap();
    assert!(peer.permits(RoomOperation::ActOwnCharacter));
    assert!(!peer.permits(RoomOperation::ManageSnapshots));
    assert!(!peer.permits(RoomOperation::ConfigureProvider));
    assert!(host.permits(RoomOperation::ManageSnapshots));
    assert!(service
        .authenticate(
            joined.room.room_id,
            created.grant.member_id,
            &joined.capability
        )
        .await
        .is_err());
    assert!(service
        .join(JoinRoom {
            room_id: joined.room.room_id,
            join_id: Uuid::new_v4(),
            display_name: "Other".into(),
            invitation: secret(&created.invitation),
            resume_proof: RoomSecret::generate()
        })
        .await
        .is_err());
    let replay = service
        .join(JoinRoom {
            room_id: joined.room.room_id,
            join_id,
            display_name: "Mira".into(),
            invitation: RoomSecret::generate(),
            resume_proof: secret(&proof),
        })
        .await
        .unwrap();
    assert!(replay.capability.expose() == joined.capability.expose());
    assert_eq!(replay.member_id, joined.member_id);
    assert!(service
        .join(JoinRoom {
            room_id: joined.room.room_id,
            join_id,
            display_name: "Mira".into(),
            invitation: secret(&created.invitation),
            resume_proof: RoomSecret::generate()
        })
        .await
        .is_err());
    let raw: String = sqlx::query_scalar("SELECT record_json FROM coop_rooms WHERE room_id=?1")
        .bind(joined.room.room_id.to_string())
        .fetch_one(store.pool())
        .await
        .unwrap();
    for value in [
        created.invitation.expose(),
        created.grant.capability.expose(),
        joined.capability.expose(),
        proof.expose(),
    ] {
        assert!(!raw.contains(value));
        assert!(!serde_json::to_string(&joined.room).unwrap().contains(value));
    }
    let recreated = service.create_local_room(command).await.unwrap();
    assert_eq!(recreated.grant.room.room_id, created.grant.room.room_id);
    assert!(recreated.grant.capability.expose() == created.grant.capability.expose());
}

#[tokio::test]
async fn expiration_revocation_and_room_closure_reject_old_credentials() {
    let (store, command) =
        fixture(sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap()).await;
    let clock = Arc::new(Clock(AtomicU64::new(1000)));
    let service = RoomService::new(
        store.clone(),
        store,
        Arc::new(InMemorySecretsStore::default()),
        clock.clone(),
    );
    let created = service.create_local_room(command).await.unwrap();
    clock.0.store(1600, Ordering::SeqCst);
    assert!(service
        .join(JoinRoom {
            room_id: created.grant.room.room_id,
            join_id: Uuid::new_v4(),
            display_name: "Guest".into(),
            invitation: secret(&created.invitation),
            resume_proof: RoomSecret::generate()
        })
        .await
        .is_err());
    let host = service
        .authenticate(
            created.grant.room.room_id,
            created.grant.member_id,
            &created.grant.capability,
        )
        .await
        .unwrap();
    let fresh = service.rotate_invitation(&host).await.unwrap();
    let joined = service
        .join(JoinRoom {
            room_id: created.grant.room.room_id,
            join_id: Uuid::new_v4(),
            display_name: "Guest".into(),
            invitation: fresh,
            resume_proof: RoomSecret::generate(),
        })
        .await
        .unwrap();
    let guest = service
        .authenticate(joined.room.room_id, joined.member_id, &joined.capability)
        .await
        .unwrap();
    assert!(service.rotate_invitation(&guest).await.is_err());
    service.revoke(&host, joined.member_id).await.unwrap();
    assert!(service.current_room(&guest).await.is_err());
    assert!(service.own_party(&guest).await.is_err());
    assert!(service.current_room(&host).await.is_err());
    assert!(service
        .authenticate(joined.room.room_id, joined.member_id, &joined.capability)
        .await
        .is_err());
    assert!(service.rotate_invitation(&host).await.is_err());
    let host = service
        .authenticate(
            created.grant.room.room_id,
            created.grant.member_id,
            &created.grant.capability,
        )
        .await
        .unwrap();
    service.close(&host).await.unwrap();
    assert!(service
        .authenticate(
            created.grant.room.room_id,
            created.grant.member_id,
            &created.grant.capability
        )
        .await
        .is_err());
}

#[tokio::test]
async fn credentials_survive_actual_sqlite_and_stronghold_reopening() {
    let directory = tempfile::tempdir().unwrap();
    let db_path = directory.path().join("room.sqlite");
    let vault_path = directory.path().join("room.hold");
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(true);
    let db = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options.clone())
        .await
        .unwrap();
    let (store, command) = fixture(db).await;
    let clock = Arc::new(Clock(AtomicU64::new(1000)));
    let vault = Arc::new(
        StrongholdSecretsStore::open(vault_path.clone(), b"test-vault-passphrase".to_vec())
            .unwrap(),
    );
    let service = RoomService::new(store.clone(), store.clone(), vault.clone(), clock.clone());
    let created = service.create_local_room(command).await.unwrap();
    let proof = RoomSecret::generate();
    let join_id = Uuid::new_v4();
    let joined = service
        .join(JoinRoom {
            room_id: created.grant.room.room_id,
            join_id,
            display_name: "Guest".into(),
            invitation: secret(&created.invitation),
            resume_proof: secret(&proof),
        })
        .await
        .unwrap();
    drop(service);
    drop(vault);
    store.pool().close().await;
    drop(store);
    let db = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    let store = Arc::new(SqliteStore::new(db));
    let vault = Arc::new(
        StrongholdSecretsStore::open(vault_path, b"test-vault-passphrase".to_vec()).unwrap(),
    );
    let reopened = RoomService::new(store.clone(), store.clone(), vault, clock);
    assert!(reopened
        .authenticate(joined.room.room_id, joined.member_id, &joined.capability)
        .await
        .is_ok());
    let replay = reopened
        .join(JoinRoom {
            room_id: joined.room.room_id,
            join_id,
            display_name: "Guest".into(),
            invitation: RoomSecret::generate(),
            resume_proof: proof,
        })
        .await
        .unwrap();
    assert!(replay.capability.expose() == joined.capability.expose());
    assert_eq!(
        store
            .room(joined.room.room_id)
            .await
            .unwrap()
            .unwrap()
            .members
            .len(),
        2
    );
    drop(reopened);
    store.pool().close().await;
}

#[tokio::test]
async fn denied_invitation_rotation_does_not_write_any_secret() {
    let (store, command) =
        fixture(sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap()).await;
    let secrets = Arc::new(CountingSecrets {
        inner: Default::default(),
        writes: Default::default(),
    });
    let service = RoomService::new(
        store.clone(),
        store,
        secrets.clone(),
        Arc::new(Clock(AtomicU64::new(1000))),
    );
    let created = service.create_local_room(command).await.unwrap();
    let joined = service
        .join(JoinRoom {
            room_id: created.grant.room.room_id,
            join_id: Uuid::new_v4(),
            display_name: "Guest".into(),
            invitation: secret(&created.invitation),
            resume_proof: RoomSecret::generate(),
        })
        .await
        .unwrap();
    let peer = service
        .authenticate(joined.room.room_id, joined.member_id, &joined.capability)
        .await
        .unwrap();
    let before = secrets.writes.load(Ordering::SeqCst);
    assert!(service.rotate_invitation(&peer).await.is_err());
    assert_eq!(secrets.writes.load(Ordering::SeqCst), before);
}
