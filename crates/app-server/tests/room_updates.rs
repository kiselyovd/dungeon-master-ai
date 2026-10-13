use adapter_secrets::InMemorySecretsStore;
use adapter_sqlite::{pool, SqliteStore};
use app_application::ports::repositories::{CombatRepository, SnapshotRepository};
use app_application::{
    campaigns::{CreateCampaign, CreateCampaignCommand},
    models::{
        campaign_state::CommitCampaignCharacter,
        room::CreateLocalRoom,
        room_character::{ApproveRoomCharacter, SubmitRoomCharacter},
        session::CampaignSeed,
    },
    ports::repositories::{
        CampaignStateRepository, MessageRepository, PartyRepository, RoomRepository,
    },
    rooms::{JoinRoom, RoomSecret, RoomService, SystemRoomClock},
};
use futures::{Stream, StreamExt};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

async fn fixture(
    db: sqlx::SqlitePool,
    secrets: Arc<dyn app_application::ports::secrets::SecretsStore>,
) -> (
    Arc<SqliteStore>,
    Arc<RoomService>,
    app_application::rooms::CreatedRoom,
    Uuid,
) {
    use app_application::campaigns::{CreateSnapshot, CreateSnapshotCommand};
    pool::migrate(&db).await.unwrap();
    let store = Arc::new(SqliteStore::new(db));
    let campaign = CreateCampaign::new(store.clone())
        .execute(CreateCampaignCommand {
            request_id: None,
            title: "Updates".into(),
            premise: "Ferry".into(),
            seed: CampaignSeed::Bundled {
                adventure_id: "bell-beneath-water".into(),
            },
        })
        .await
        .unwrap();
    let current=store.commit_character(CommitCampaignCharacter {campaign_id:campaign.id,session_id:campaign.last_session_id,character_id:Uuid::new_v4(),expected_revision:campaign.revision,causation_id:"host".into(),character:json!({"name":"Mira","heroClass":"fighter","level":1,"hp":12,"hpMax":12,"ac":14,"abilities":{"str":15,"dex":14,"con":13,"int":12,"wis":10,"cha":8},"inventory":[],"backstory":"OWN_PRIVATE"})}).await.unwrap();
    store
        .append(
            campaign.last_session_id.0,
            app_application::models::chat::ChatMessage::user_text("SOLO_PRIVATE_CHAT"),
        )
        .await
        .unwrap();
    let saved = CreateSnapshot::new(store.clone())
        .execute(CreateSnapshotCommand {
            id: Uuid::new_v4(),
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            expected_revision: current.campaign.revision,
            title: "Solo".into(),
            kind: "manual".into(),
            summary: "".into(),
            tag: "".into(),
            causation_id: "solo-save".into(),
            character: current.projection.character,
        })
        .await
        .unwrap();
    let current = store.load_current(campaign.id).await.unwrap();
    let service = Arc::new(RoomService::new(
        store.clone(),
        store.clone(),
        secrets,
        Arc::new(SystemRoomClock),
    ));
    let created = service
        .create_local_room(CreateLocalRoom {
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            expected_revision: current.campaign.revision,
            causation_id: "room".into(),
        })
        .await
        .unwrap();
    (store, service, created, saved.save_id)
}

#[tokio::test]
async fn restoration_resets_delivery_without_publishing_old_solo_chat() {
    use app_application::{
        campaigns::{CreateSnapshot, CreateSnapshotCommand},
        models::snapshot::RestoreSnapshotRecord,
    };
    let (store, service, created, solo_save) = fixture(
        sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap(),
        Arc::new(InMemorySecretsStore::default()),
    )
    .await;
    let host = service
        .authenticate(
            created.grant.room.room_id,
            created.grant.member_id,
            &created.grant.capability,
        )
        .await
        .unwrap();
    let campaign = created.grant.room.campaign_id;
    let session = created.grant.room.session_id;
    store
        .append_room_agent_message(
            host.lease(),
            Uuid::new_v4(),
            app_application::models::chat::ChatMessage::Assistant {
                content: "PUBLIC_A".into(),
            },
        )
        .await
        .unwrap();
    let current = store.load_current(campaign).await.unwrap();
    let saved = CreateSnapshot::new(store.clone())
        .execute(CreateSnapshotCommand {
            id: Uuid::new_v4(),
            campaign_id: campaign,
            session_id: session,
            expected_revision: current.campaign.revision,
            title: "Public".into(),
            kind: "manual".into(),
            summary: "".into(),
            tag: "".into(),
            causation_id: "public-save".into(),
            character: current.projection.character,
        })
        .await
        .unwrap();
    assert_eq!(saved.snapshot.projection.shared_message_indices.len(), 1);
    store
        .append_room_agent_message(
            host.lease(),
            Uuid::new_v4(),
            app_application::models::chat::ChatMessage::Assistant {
                content: "PUBLIC_B".into(),
            },
        )
        .await
        .unwrap();
    let before = service.updates(&host, None).await.unwrap().unwrap();
    let current = store.load_current(campaign).await.unwrap();
    store
        .restore_snapshot(RestoreSnapshotRecord {
            save_id: saved.save_id,
            campaign_id: campaign,
            session_id: session,
            expected_revision: current.campaign.revision,
            causation_id: "restore-public".into(),
        })
        .await
        .unwrap();
    let restored = service
        .updates(&host, Some(before.cursor))
        .await
        .unwrap()
        .unwrap();
    assert!(restored.reset);
    assert!(restored.cursor > before.cursor);
    let body = serde_json::to_string(&restored).unwrap();
    assert!(body.contains("PUBLIC_A"));
    assert!(!body.contains("PUBLIC_B"));
    assert!(!body.contains("SOLO_PRIVATE_CHAT"));
    assert!(restored
        .events
        .iter()
        .all(|event| event.sequence > before.cursor));
    let current = store.load_current(campaign).await.unwrap();
    store
        .restore_snapshot(RestoreSnapshotRecord {
            save_id: solo_save,
            campaign_id: campaign,
            session_id: session,
            expected_revision: current.campaign.revision,
            causation_id: "restore-private-solo".into(),
        })
        .await
        .unwrap();
    let private = service
        .updates(&host, Some(restored.cursor))
        .await
        .unwrap()
        .unwrap();
    assert!(private.reset);
    assert!(private.timeline_reset_cursor > restored.timeline_reset_cursor);
    for index in 0..140 {
        store
            .append_room_agent_message(
                host.lease(),
                Uuid::new_v4(),
                app_application::models::chat::ChatMessage::Assistant {
                    content: format!("AFTER_RESTORE_{index}"),
                },
            )
            .await
            .unwrap();
    }
    let delayed = service
        .updates(&host, Some(before.cursor))
        .await
        .unwrap()
        .unwrap();
    assert!(delayed.reset);
    assert_eq!(delayed.timeline_reset_cursor, private.timeline_reset_cursor);
    assert!(delayed
        .events
        .iter()
        .all(|event| event.kind != "timeline_reset"));
    assert!(private.state.messages.is_empty());
    assert!(!serde_json::to_string(&private)
        .unwrap()
        .contains("SOLO_PRIVATE_CHAT"));
}

#[tokio::test]
async fn a_large_backlog_is_delivered_in_strict_cursor_order_without_missing_changes() {
    let (store, service, created, _) = fixture(
        sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap(),
        Arc::new(InMemorySecretsStore::default()),
    )
    .await;
    let host = service
        .authenticate(
            created.grant.room.room_id,
            created.grant.member_id,
            &created.grant.capability,
        )
        .await
        .unwrap();
    let initial = service.updates(&host, None).await.unwrap().unwrap();
    for index in 0..300 {
        store
            .append_room_agent_message(
                host.lease(),
                Uuid::new_v4(),
                app_application::models::chat::ChatMessage::Assistant {
                    content: format!("Shared narration {index}"),
                },
            )
            .await
            .unwrap();
    }
    let mut cursor = initial.cursor;
    let mut count = 0;
    let mut pages = 0;
    while let Some(update) = service.updates(&host, Some(cursor)).await.unwrap() {
        assert!(!update.reset);
        assert!(update.events.len() <= 128);
        assert!(update.head >= update.cursor);
        for event in &update.events {
            assert!(event.sequence > cursor);
            cursor = event.sequence;
            count += 1;
        }
        assert_eq!(update.cursor, cursor);
        pages += 1;
    }
    assert_eq!(count, 300);
    assert_eq!(pages, 3);
}

#[tokio::test]
async fn cursor_survives_real_database_and_vault_reopening_and_failed_commits_publish_nothing() {
    use app_application::{
        agent::hero_intentions::HeroCombatIntent, models::room_combat::RoomCombatCommand,
    };
    let db_path = std::env::temp_dir().join(format!("dmai-updates-{}.sqlite", Uuid::new_v4()));
    let vault_path = db_path.with_extension("hold");
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(true);
    let db = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options.clone())
        .await
        .unwrap();
    let vault = Arc::new(
        adapter_secrets::StrongholdSecretsStore::open(
            vault_path.clone(),
            b"test-vault-passphrase".to_vec(),
        )
        .unwrap(),
    );
    let (store, service, created, _) = fixture(db, vault.clone()).await;
    let host = service
        .authenticate(
            created.grant.room.room_id,
            created.grant.member_id,
            &created.grant.capability,
        )
        .await
        .unwrap();
    let npc = app_domain::combat::types::CombatantId::new();
    let encounter = Uuid::new_v4();
    let projection = store
        .create(
            created.grant.room.session_id.0,
            app_application::models::combat::CombatProjection {
                authored: None,
                adventure: None,
                campaign_scope: None,
                appearances: Default::default(),
                schema_version: app_application::models::combat::COMBAT_PROJECTION_VERSION,
                encounter_id: encounter,
                revision: 0,
                snapshot: app_application::models::combat::CombatSnapshot {
                    active: true,
                    round: 1,
                    current_combatant: Some(npc),
                    initiative: vec![app_domain::combat::initiative::InitiativeEntry {
                        id: npc,
                        roll: -99,
                        dex_tiebreak: 0,
                    }],
                    combatants: vec![app_domain::combat::combatant::Combatant::new(
                        npc,
                        "Foe".into(),
                        100,
                        100,
                        10,
                    )],
                },
                events: vec![],
            },
        )
        .await
        .unwrap();
    let before = service.updates(&host, None).await.unwrap().unwrap();
    let command = RoomCombatCommand {
        request_id: "turn".into(),
        expected_revision: projection.revision,
        action: HeroCombatIntent::EndTurn,
    };
    sqlx::raw_sql("CREATE TRIGGER fail_delivery_receipt BEFORE INSERT ON combat_action_receipts BEGIN SELECT RAISE(ABORT,'synthetic_receipt_failure'); END;").execute(store.pool()).await.unwrap();
    assert!(service
        .act_combat(&host, encounter, command.clone())
        .await
        .is_err());
    assert!(service
        .updates(&host, Some(before.cursor))
        .await
        .unwrap()
        .is_none());
    sqlx::raw_sql("DROP TRIGGER fail_delivery_receipt;")
        .execute(store.pool())
        .await
        .unwrap();
    service
        .act_combat(&host, encounter, command.clone())
        .await
        .unwrap();
    let updated = service
        .updates(&host, Some(before.cursor))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        updated
            .events
            .iter()
            .filter(|event| event.kind == "combat")
            .count(),
        1
    );
    assert!(!serde_json::to_string(&updated.events)
        .unwrap()
        .contains("loadout"));
    service.act_combat(&host, encounter, command).await.unwrap();
    assert!(service
        .updates(&host, Some(updated.cursor))
        .await
        .unwrap()
        .is_none());
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
        adapter_secrets::StrongholdSecretsStore::open(
            vault_path.clone(),
            b"test-vault-passphrase".to_vec(),
        )
        .unwrap(),
    );
    let reopened = RoomService::new(
        store.clone(),
        store.clone(),
        vault,
        Arc::new(SystemRoomClock),
    );
    let host = reopened
        .authenticate(
            created.grant.room.room_id,
            created.grant.member_id,
            &created.grant.capability,
        )
        .await
        .unwrap();
    let recovered = reopened
        .updates(&host, Some(before.cursor))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(recovered.cursor, updated.cursor);
    assert_eq!(
        recovered
            .events
            .iter()
            .filter(|event| event.kind == "combat")
            .count(),
        1
    );
    let future = reopened
        .updates(&host, Some(u64::MAX))
        .await
        .unwrap()
        .unwrap();
    assert!(future.reset);
    assert_eq!(future.cursor, updated.cursor);
    drop(reopened);
    store.pool().close().await;
    drop(store);
    std::fs::remove_file(db_path).unwrap();
    std::fs::remove_file(vault_path).unwrap();
}

async fn read_event<S, B, E>(stream: &mut S, buffer: &mut Vec<u8>) -> serde_json::Value
where
    S: Stream<Item = Result<B, E>> + Unpin,
    B: AsRef<[u8]>,
    E: std::fmt::Debug,
{
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            if let Some(end) = buffer.windows(2).position(|pair| pair == b"\n\n") {
                let frame = String::from_utf8(buffer.drain(..end + 2).collect()).unwrap();
                if let Some(data) = frame.lines().find_map(|line| {
                    line.strip_prefix("data: ")
                        .or_else(|| line.strip_prefix("data:"))
                }) {
                    return serde_json::from_str(data).unwrap();
                }
            } else {
                let chunk = stream.next().await.expect("live stream").unwrap();
                buffer.extend_from_slice(chunk.as_ref());
            }
        }
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn shared_state_and_durable_updates_are_scoped_and_private() {
    let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&db).await.unwrap();
    let store = Arc::new(SqliteStore::new(db));
    let campaign = CreateCampaign::new(store.clone())
        .execute(CreateCampaignCommand {
            request_id: None,
            title: "Shared".into(),
            premise: "Ferry".into(),
            seed: CampaignSeed::Bundled {
                adventure_id: "bell-beneath-water".into(),
            },
        })
        .await
        .unwrap();
    let sheet = json!({"name":"Mira","heroClass":"fighter","level":1,"hp":12,"hpMax":12,"ac":14,"abilities":{"str":15,"dex":14,"con":13,"int":12,"wis":10,"cha":8},"inventory":[],"backstory":"HOST_PRIVATE_STORY"});
    let committed = store
        .commit_character(CommitCampaignCharacter {
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            character_id: Uuid::new_v4(),
            expected_revision: campaign.revision,
            causation_id: "host".into(),
            character: sheet.clone(),
        })
        .await
        .unwrap();
    store
        .append(
            campaign.last_session_id.0,
            app_application::models::chat::ChatMessage::user_text("SOLO_PRIVATE_CHAT"),
        )
        .await
        .unwrap();
    let service = Arc::new(RoomService::new(
        store.clone(),
        store.clone(),
        Arc::new(InMemorySecretsStore::default()),
        Arc::new(SystemRoomClock),
    ));
    let created = service
        .create_local_room(CreateLocalRoom {
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            expected_revision: committed.campaign.revision,
            causation_id: "room".into(),
        })
        .await
        .unwrap();
    let guest = service
        .join(JoinRoom {
            room_id: created.grant.room.room_id,
            join_id: Uuid::new_v4(),
            display_name: "Guest".into(),
            invitation: RoomSecret::from_input(created.invitation.expose().into()).unwrap(),
            resume_proof: RoomSecret::generate(),
        })
        .await
        .unwrap();
    let guest_subject = service
        .authenticate(guest.room.room_id, guest.member_id, &guest.capability)
        .await
        .unwrap();
    let host = service
        .authenticate(
            created.grant.room.room_id,
            created.grant.member_id,
            &created.grant.capability,
        )
        .await
        .unwrap();
    let character_id = Uuid::new_v4();
    let proposal_id = Uuid::new_v4();
    let mut guest_sheet = sheet;
    guest_sheet["characterId"] = json!(character_id);
    guest_sheet["backstory"] = json!("GUEST_PRIVATE_STORY");
    service
        .submit_character(
            &guest_subject,
            SubmitRoomCharacter {
                proposal_id,
                character_id,
                character: guest_sheet,
            },
        )
        .await
        .unwrap();
    let party = store.party(campaign.id).await.unwrap().unwrap();
    service
        .approve_character(
            &host,
            ApproveRoomCharacter {
                proposal_id,
                expected_revision: party.revision,
                causation_id: "accept".into(),
            },
        )
        .await
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let router = adapter_http::rooms::router(service.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let client = reqwest::Client::new();
    let url = format!("http://{address}/rooms/{}", guest.room.room_id);
    let response = client
        .get(format!("{url}/state"))
        .bearer_auth(guest.capability.expose())
        .header("x-room-member-id", guest.member_id.to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let state: serde_json::Value = response.json().await.unwrap();
    let body = state.to_string();
    assert!(body.contains("GUEST_PRIVATE_STORY"));
    assert!(!body.contains("HOST_PRIVATE_STORY"));
    assert!(!body.contains("SOLO_PRIVATE_CHAT"));
    let cursor = state["cursor"].as_u64().unwrap();
    store
        .append(
            campaign.last_session_id.0,
            app_application::models::chat::ChatMessage::System {
                content: "SYSTEM_PRIVATE_CONTEXT".into(),
            },
        )
        .await
        .unwrap();
    store
        .append_room_agent_message(
            host.lease(),
            Uuid::new_v4(),
            app_application::models::chat::ChatMessage::Assistant {
                content: "Переправа затихла.".into(),
            },
        )
        .await
        .unwrap();
    let response = client
        .get(format!("{url}/updates?after={cursor}"))
        .bearer_auth(guest.capability.expose())
        .header("x-room-member-id", guest.member_id.to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let update: serde_json::Value = response.json().await.unwrap();
    assert!(update["cursor"].as_u64().unwrap() > cursor);
    let body = update.to_string();
    assert!(body.contains("Переправа затихла."));
    assert!(!body.contains("SYSTEM_PRIVATE_CONTEXT"));
    assert!(!body.contains("HOST_PRIVATE_STORY"));
    let cursor = update["cursor"].as_u64().unwrap();
    let empty = client
        .get(format!("{url}/updates?after={cursor}"))
        .bearer_auth(guest.capability.expose())
        .header("x-room-member-id", guest.member_id.to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(empty.status(), 204);
    let host_state = client
        .get(format!("{url}/state"))
        .bearer_auth(created.grant.capability.expose())
        .header("x-room-member-id", created.grant.member_id.to_string())
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(!host_state.contains("GUEST_PRIVATE_STORY"));
    assert!(host_state.contains("HOST_PRIVATE_STORY"));
    let host_stream = client
        .get(format!("{url}/events"))
        .bearer_auth(created.grant.capability.expose())
        .header("x-room-member-id", created.grant.member_id.to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(host_stream.status(), 200);
    let mut host_stream = host_stream.bytes_stream();
    let mut host_buffer = Vec::new();
    let guest_stream = client
        .get(format!("{url}/events"))
        .bearer_auth(guest.capability.expose())
        .header("x-room-member-id", guest.member_id.to_string())
        .header("last-event-id", cursor.to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(guest_stream.status(), 200);
    let mut guest_stream = guest_stream.bytes_stream();
    let mut guest_buffer = Vec::new();
    let initial_host = read_event(&mut host_stream, &mut host_buffer).await;
    let initial_guest = read_event(&mut guest_stream, &mut guest_buffer).await;
    assert_eq!(initial_host["head"], initial_guest["head"]);
    assert_eq!(initial_guest["reset"], false);
    assert!(initial_guest["events"].as_array().unwrap().is_empty());
    assert!(!initial_host.to_string().contains("GUEST_PRIVATE_STORY"));
    assert!(!initial_guest.to_string().contains("HOST_PRIVATE_STORY"));
    store
        .append_room_agent_message(
            host.lease(),
            Uuid::new_v4(),
            app_application::models::chat::ChatMessage::Assistant {
                content: "Колокол прозвучал.".into(),
            },
        )
        .await
        .unwrap();
    let changed_host = read_event(&mut host_stream, &mut host_buffer).await;
    let changed_guest = read_event(&mut guest_stream, &mut guest_buffer).await;
    assert_eq!(changed_host["cursor"], changed_guest["cursor"]);
    assert!(changed_host.to_string().contains("Колокол прозвучал."));
    assert!(changed_guest.to_string().contains("Колокол прозвучал."));
    service.revoke(&host, guest.member_id).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while guest_stream.next().await.is_some() {}
    })
    .await
    .unwrap();
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn local_chat_stays_private_while_a_durable_room_is_active() {
    let (store, service, created, _) = fixture(
        sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap(),
        Arc::new(InMemorySecretsStore::default()),
    )
    .await;
    let host = service
        .authenticate(
            created.grant.room.room_id,
            created.grant.member_id,
            &created.grant.capability,
        )
        .await
        .unwrap();
    let initial = service.initial_updates(&host, None).await.unwrap();
    let local = store
        .append(
            created.grant.room.session_id.0,
            app_application::models::chat::ChatMessage::user_text("DETACHED_SOLO_PRIVATE"),
        )
        .await
        .unwrap();
    let leaked: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM shared_messages WHERE message_id=?1)")
            .bind(local.id.to_string())
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert!(!leaked, "unscoped local message must never become shared");
    assert!(service
        .updates(&host, Some(initial.cursor))
        .await
        .unwrap()
        .is_none());
    let current = service.initial_updates(&host, None).await.unwrap();
    assert!(!serde_json::to_string(&current)
        .unwrap()
        .contains("DETACHED_SOLO_PRIVATE"));
}

#[tokio::test]
async fn scoped_publication_excludes_tool_context_and_rolls_back_with_delivery_failure() {
    use app_application::models::chat::ChatMessage;
    let (store, service, created, _) = fixture(
        sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap(),
        Arc::new(InMemorySecretsStore::default()),
    )
    .await;
    let host = service
        .authenticate(
            created.grant.room.room_id,
            created.grant.member_id,
            &created.grant.capability,
        )
        .await
        .unwrap();
    let turn = Uuid::new_v4();
    store
        .append_room_agent_message(
            host.lease(),
            turn,
            ChatMessage::user_text("PUBLIC_INTENTION"),
        )
        .await
        .unwrap();
    store
        .append_room_agent_message(
            host.lease(),
            turn,
            ChatMessage::AssistantWithToolCalls {
                content: Some("PRIVATE_TOOL_CONTEXT".into()),
                tool_calls: vec![],
            },
        )
        .await
        .unwrap();
    let before = service.initial_updates(&host, None).await.unwrap();
    assert!(serde_json::to_string(&before)
        .unwrap()
        .contains("PUBLIC_INTENTION"));
    assert!(!serde_json::to_string(&before)
        .unwrap()
        .contains("PRIVATE_TOOL_CONTEXT"));
    sqlx::raw_sql("CREATE TRIGGER fail_publication BEFORE INSERT ON room_changes WHEN NEW.kind='message' BEGIN SELECT RAISE(ABORT,'fixture_delivery_failure'); END;")
        .execute(store.pool()).await.unwrap();
    assert!(store
        .append_room_agent_message(
            host.lease(),
            turn,
            ChatMessage::Assistant {
                content: "ROLLED_BACK_NARRATION".into(),
            }
        )
        .await
        .is_err());
    let stored = store.list(created.grant.room.session_id.0).await.unwrap();
    assert!(!serde_json::to_string(&stored)
        .unwrap()
        .contains("ROLLED_BACK_NARRATION"));
    assert_eq!(
        store
            .room_agent_checkpoint(host.lease(), turn)
            .await
            .unwrap()
            .len(),
        2
    );
    assert!(service
        .updates(&host, Some(before.cursor))
        .await
        .unwrap()
        .is_none());
    sqlx::raw_sql("DROP TRIGGER fail_publication")
        .execute(store.pool())
        .await
        .unwrap();
    store
        .append_room_agent_message(
            host.lease(),
            turn,
            ChatMessage::Assistant {
                content: "PUBLIC_CONFIRMED_NARRATION".into(),
            },
        )
        .await
        .unwrap();
    let current = service
        .updates(&host, Some(before.cursor))
        .await
        .unwrap()
        .unwrap();
    assert!(serde_json::to_string(&current)
        .unwrap()
        .contains("PUBLIC_CONFIRMED_NARRATION"));
}
