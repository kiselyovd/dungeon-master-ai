use adapter_media::asset_codec::RasterContentInspector;
use adapter_secrets::InMemorySecretsStore;
use adapter_sqlite::{pool, SqliteStore};
use app_application::agent::operations::{
    AgentOperation, AgentOperationRepository, OperationClaim, ToolStepClaim,
};
use app_application::agent::tool_dispatch::{GeneratedAgentMedia, ToolExecution};
use app_application::models::agent::AgentTurnRequest;
use app_application::models::snapshot::{CreateSnapshotRecord, RestoreSnapshotRecord};
use app_application::ports::repositories::SnapshotRepository;
use app_application::{
    campaigns::{CreateCampaign, CreateCampaignCommand},
    media_assets::MediaAssets,
    models::{
        campaign_state::CommitCampaignCharacter, media_asset::*, room::CreateLocalRoom,
        room_character::*, session::CampaignSeed,
    },
    ports::{media_assets::MediaContentInspector, repositories::CampaignStateRepository},
    rooms::{JoinRoom, RoomSecret, RoomService, SystemRoomClock},
};
use serde_json::json;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::sync::Arc;
use uuid::Uuid;
const PORTRAIT: &[u8] =
    include_bytes!("../../adapter-media/assets/bundled/portraits/portrait-fighter-01-v2.webp");
const HOST_PORTRAIT: &[u8] =
    include_bytes!("../../adapter-media/assets/bundled/portraits/portrait-fighter-legacy.webp");
const MAP: &[u8] = include_bytes!("../../../src/assets/living-tabletop/v2/map-archive-v2.webp");
fn media(store: Arc<SqliteStore>) -> Arc<MediaAssets> {
    Arc::new(MediaAssets::new(store, Arc::new(RasterContentInspector)))
}
fn id(reference: &MediaReference) -> Uuid {
    let MediaReference::Owned { asset_id, .. } = reference else {
        panic!("owned reference expected")
    };
    *asset_id
}
fn hero(name: &str) -> serde_json::Value {
    json!({"name":name,"heroClass":"fighter","level":1,"hp":12,"hpMax":12,"ac":14,"abilities":{"str":15,"dex":14,"con":13,"int":12,"wis":10,"cha":8},"inventory":[],"backstory":"PRIVATE_SHEET_SENTINEL"})
}
async fn campaign(store: Arc<SqliteStore>) -> app_application::models::session::CampaignDetail {
    CreateCampaign::new(store)
        .execute(CreateCampaignCommand {
            title: "Media storage".into(),
            premise: "Bounded proof".into(),
            seed: CampaignSeed::Bundled {
                adventure_id: "bell-beneath-water".into(),
            },
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn real_file_reopen_preserves_bytes_and_hash_without_inline_character_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("media.db");
    let db = SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true),
    )
    .await
    .unwrap();
    pool::migrate(&db).await.unwrap();
    let store = Arc::new(SqliteStore::new(db.clone()));
    let assets = media(store.clone());
    let c = campaign(store.clone()).await;
    let draft = Uuid::new_v4();
    let actor = Uuid::new_v4();
    let reference = assets
        .register(
            MediaOwner::Draft(draft),
            MediaCategory::Portrait,
            MediaOrigin::Uploaded,
            "image/webp",
            PORTRAIT.to_vec(),
        )
        .await
        .unwrap();
    let mut sheet = hero("Mira");
    sheet["portraitMediaRef"] = json!(reference);
    sheet["portraitUrl"] = json!(null);
    store
        .commit_character(CommitCampaignCharacter {
            campaign_id: c.id,
            session_id: c.last_session_id,
            character_id: actor,
            expected_revision: c.revision,
            causation_id: "portrait".into(),
            character: sheet,
        })
        .await
        .unwrap();
    assert_eq!(
        assets
            .read(id(&reference), &MediaReadScope::Campaign(Uuid::new_v4()))
            .await
            .unwrap_err(),
        MediaAssetError::Denied
    );
    drop(assets);
    drop(store);
    db.close().await;
    let reopened = SqlitePool::connect_with(SqliteConnectOptions::new().filename(&path))
        .await
        .unwrap();
    let restored_store = Arc::new(SqliteStore::new(reopened.clone()));
    let restored = media(restored_store.clone())
        .read(id(&reference), &MediaReadScope::Campaign(c.id.0))
        .await
        .unwrap();
    assert_eq!(restored.reference, reference);
    assert_eq!(restored.bytes, PORTRAIT);
    let persisted = restored_store
        .load_current(c.id)
        .await
        .unwrap()
        .projection
        .character;
    assert_eq!(persisted["portraitUrl"], json!(null));
    assert_eq!(persisted["portraitMediaRef"], json!(reference));
    assert!(!persisted.to_string().contains("base64"));
    reopened.close().await;
}

#[tokio::test]
async fn deduplication_is_immutable_and_registration_retry_retains_ownership() {
    let db = SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&db).await.unwrap();
    let store = Arc::new(SqliteStore::new(db.clone()));
    let assets = media(store);
    let first_draft = Uuid::new_v4();
    let second_draft = Uuid::new_v4();
    let asset_id = Uuid::new_v4();
    let a = assets
        .register_as(
            asset_id,
            MediaOwner::Draft(first_draft),
            MediaCategory::Portrait,
            MediaOrigin::Uploaded,
            "image/webp",
            PORTRAIT.to_vec(),
        )
        .await
        .unwrap();
    let replay = assets
        .register_as(
            asset_id,
            MediaOwner::Draft(first_draft),
            MediaCategory::Portrait,
            MediaOrigin::Uploaded,
            "image/webp",
            PORTRAIT.to_vec(),
        )
        .await
        .unwrap();
    assert_eq!(a, replay);
    assert_eq!(
        assets
            .register_as(
                asset_id,
                MediaOwner::Draft(second_draft),
                MediaCategory::Portrait,
                MediaOrigin::Uploaded,
                "image/webp",
                PORTRAIT.to_vec()
            )
            .await
            .unwrap_err(),
        MediaAssetError::Denied
    );
    let b = assets
        .register(
            MediaOwner::Draft(second_draft),
            MediaCategory::Portrait,
            MediaOrigin::Uploaded,
            "image/webp",
            PORTRAIT.to_vec(),
        )
        .await
        .unwrap();
    assert_ne!(id(&a), id(&b));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM media_content")
            .fetch_one(&db)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        assets
            .read(id(&a), &MediaReadScope::Draft(second_draft))
            .await
            .unwrap_err(),
        MediaAssetError::Denied
    );
    let package = assets
        .export_asset(&a, &MediaReadScope::Draft(first_draft))
        .await
        .unwrap();
    let imported = assets
        .import_asset(MediaOwner::Draft(second_draft), package.clone())
        .await
        .unwrap();
    assert_ne!(id(&imported), id(&a));
    assert_eq!(
        assets
            .read(id(&imported), &MediaReadScope::Draft(second_draft))
            .await
            .unwrap()
            .bytes,
        PORTRAIT
    );
    let mut corrupt = package;
    corrupt.bytes[0] ^= 1;
    assert!(assets
        .import_asset(MediaOwner::Draft(second_draft), corrupt)
        .await
        .is_err());
    sqlx::query("UPDATE media_content SET payload=zeroblob(length(payload))")
        .execute(&db)
        .await
        .unwrap();
    assert_eq!(
        assets
            .read(id(&a), &MediaReadScope::Draft(first_draft))
            .await
            .unwrap_err(),
        MediaAssetError::Corrupt
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM media_assets")
            .fetch_one(&db)
            .await
            .unwrap(),
        3
    );
    assert_eq!(
        assets
            .read(Uuid::new_v4(), &MediaReadScope::Draft(first_draft))
            .await
            .unwrap_err(),
        MediaAssetError::Missing
    );
}

#[tokio::test]
async fn room_peer_reads_only_current_public_actor_art_and_revoked_scope_is_denied() {
    let db = SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&db).await.unwrap();
    let store = Arc::new(SqliteStore::new(db));
    let assets = media(store.clone());
    let c = campaign(store.clone()).await;
    let host_art = assets
        .register(
            MediaOwner::Draft(Uuid::new_v4()),
            MediaCategory::Portrait,
            MediaOrigin::Uploaded,
            "image/webp",
            HOST_PORTRAIT.to_vec(),
        )
        .await
        .unwrap();
    let mut host_sheet = hero("Host");
    host_sheet["portraitMediaRef"] = json!(host_art);
    let current = store
        .commit_character(CommitCampaignCharacter {
            campaign_id: c.id,
            session_id: c.last_session_id,
            character_id: Uuid::new_v4(),
            expected_revision: c.revision,
            causation_id: "host".into(),
            character: host_sheet,
        })
        .await
        .unwrap();
    let rooms = RoomService::new(
        store.clone(),
        store.clone(),
        Arc::new(InMemorySecretsStore::default()),
        Arc::new(SystemRoomClock),
    )
    .with_media_assets(assets.clone());
    let created = rooms
        .create_local_room(CreateLocalRoom {
            campaign_id: c.id,
            session_id: c.last_session_id,
            expected_revision: current.campaign.revision,
            causation_id: "room".into(),
        })
        .await
        .unwrap();
    let room = created.grant.room.room_id;
    let guest = rooms
        .join(JoinRoom {
            room_id: room,
            join_id: Uuid::new_v4(),
            display_name: "Guest".into(),
            invitation: RoomSecret::from_input(created.invitation.expose().into()).unwrap(),
            resume_proof: RoomSecret::generate(),
        })
        .await
        .unwrap();
    let host_subject = rooms
        .authenticate(room, created.grant.member_id, &created.grant.capability)
        .await
        .unwrap();
    let guest_subject = rooms
        .authenticate(room, guest.member_id, &guest.capability)
        .await
        .unwrap();
    assert_eq!(
        rooms
            .public_asset(&guest_subject, id(&host_art))
            .await
            .unwrap()
            .bytes,
        HOST_PORTRAIT
    );
    let public = rooms
        .upload_portrait(
            &guest_subject,
            Uuid::new_v4(),
            "image/webp",
            PORTRAIT.to_vec(),
        )
        .await
        .unwrap();
    let private = rooms
        .upload_portrait(
            &guest_subject,
            Uuid::new_v4(),
            "image/webp",
            PORTRAIT.to_vec(),
        )
        .await
        .unwrap();
    assert_eq!(
        rooms
            .public_asset(&host_subject, id(&private))
            .await
            .unwrap_err(),
        MediaAssetError::Denied
    );
    let guest_character = Uuid::new_v4();
    let mut sheet = hero("Guest");
    sheet["portraitMediaRef"] = json!(public);
    let proposal = rooms
        .submit_character(
            &guest_subject,
            SubmitRoomCharacter {
                proposal_id: Uuid::new_v4(),
                character_id: guest_character,
                character: sheet,
            },
        )
        .await
        .unwrap();
    let current = rooms.own_party(&host_subject).await.unwrap().unwrap();
    rooms
        .approve_character(
            &host_subject,
            ApproveRoomCharacter {
                proposal_id: proposal.proposal_id,
                expected_revision: current.revision,
                causation_id: "approve-media".into(),
            },
        )
        .await
        .unwrap();
    let view = rooms.own_party(&host_subject).await.unwrap().unwrap();
    let public_json = serde_json::to_string(&view.actors).unwrap();
    assert!(public_json.contains(&id(&public).to_string()));
    assert!(!public_json.contains("PRIVATE_SHEET_SENTINEL"));
    assert!(!public_json.contains("data_b64"));
    assert_eq!(
        rooms
            .public_asset(&host_subject, id(&public))
            .await
            .unwrap()
            .bytes,
        PORTRAIT
    );
    assert_eq!(
        rooms
            .public_asset(&host_subject, id(&private))
            .await
            .unwrap_err(),
        MediaAssetError::Denied
    );
    rooms.revoke(&host_subject, guest.member_id).await.unwrap();
    let current_host = rooms
        .authenticate(room, created.grant.member_id, &created.grant.capability)
        .await
        .unwrap();
    assert_eq!(
        rooms
            .public_asset(&current_host, id(&public))
            .await
            .unwrap_err(),
        MediaAssetError::Denied
    );
    assert_eq!(
        rooms
            .public_asset(&guest_subject, id(&public))
            .await
            .unwrap_err(),
        MediaAssetError::Denied
    );
    assert_eq!(
        rooms
            .public_asset(&host_subject, id(&public))
            .await
            .unwrap_err(),
        MediaAssetError::Denied
    );
}

#[test]
fn inspector_rejects_spoofed_mime_truncated_container_and_oversized_input() {
    let inspector = RasterContentInspector;
    assert!(inspector.inspect(PORTRAIT, "image/webp").unwrap().valid());
    assert_eq!(
        inspector.inspect(PORTRAIT, "image/png"),
        Err(MediaAssetError::Invalid)
    );
    assert_eq!(
        inspector.inspect(&PORTRAIT[..PORTRAIT.len() - 1], "image/webp"),
        Err(MediaAssetError::Invalid)
    );
    assert_eq!(
        inspector.inspect(&vec![0; MAX_MEDIA_ASSET_BYTES + 1], "image/png"),
        Err(MediaAssetError::TooLarge)
    );
    assert_eq!(
        inspector.inspect(b"<svg/>", "image/svg+xml"),
        Err(MediaAssetError::Invalid)
    );
}

#[tokio::test]
async fn near_legacy_eight_mib_envelope_is_preserved_without_truncation() {
    let db = SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&db).await.unwrap();
    let assets = media(Arc::new(SqliteStore::new(db)));
    // An ignorable RIFF chunk preserves the existing valid pixels and exact bytes.
    // The resulting data URL fits just below the legacy 8MiB envelope.
    let target = 6 * 1024 * 1024 - 128;
    let extra = target - PORTRAIT.len() - 8;
    assert_eq!(extra % 2, 0);
    let mut bytes = PORTRAIT.to_vec();
    bytes.extend_from_slice(b"JUNK");
    bytes.extend_from_slice(&(extra as u32).to_le_bytes());
    bytes.resize(target, b'x');
    bytes[4..8].copy_from_slice(&((target - 8) as u32).to_le_bytes());
    let draft = Uuid::new_v4();
    let reference = assets
        .register(
            MediaOwner::Draft(draft),
            MediaCategory::Portrait,
            MediaOrigin::Migrated,
            "image/webp",
            bytes.clone(),
        )
        .await
        .unwrap();
    let restored = assets
        .read(id(&reference), &MediaReadScope::Draft(draft))
        .await
        .unwrap();
    assert_eq!(restored.bytes, bytes);
    assert_eq!(restored.reference, reference);
}

#[tokio::test]
async fn map_reference_commits_with_tool_receipt_and_restores_without_repeating_bytes() {
    let db = SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&db).await.unwrap();
    let store = Arc::new(SqliteStore::new(db.clone()));
    let assets = media(store.clone());
    let c = campaign(store.clone()).await;
    let current = store
        .commit_character(CommitCampaignCharacter {
            campaign_id: c.id,
            session_id: c.last_session_id,
            character_id: Uuid::new_v4(),
            expected_revision: c.revision,
            causation_id: "map-host".into(),
            character: hero("Map host"),
        })
        .await
        .unwrap();
    let saved = store
        .create_snapshot(CreateSnapshotRecord {
            id: Uuid::new_v4(),
            campaign_id: c.id,
            session_id: c.last_session_id,
            expected_revision: current.campaign.revision,
            schema_version: 1,
            ruleset_id: "srd-5.2.1".into(),
            title: "Before map".into(),
            kind: "manual".into(),
            summary: String::new(),
            tag: "exploration".into(),
            causation_id: "before-map".into(),
            character: json!({}),
        })
        .await
        .unwrap();
    let request = Uuid::new_v4();
    let operation = AgentOperation {
        request_id: request,
        request: AgentTurnRequest {
            campaign_id: c.id.0,
            session_id: c.last_session_id.0,
            player_message: "Show a map".into(),
            history: vec![],
            images: vec![],
            board: None,
        },
        model: Some("fixture".into()),
    };
    assert!(matches!(
        store.claim(operation, Uuid::new_v4()).await.unwrap(),
        OperationClaim::Started
    ));
    let reference = assets
        .register(
            MediaOwner::Campaign {
                campaign_id: c.id.0,
                actor_id: None,
            },
            MediaCategory::Map,
            MediaOrigin::Generated,
            "image/webp",
            MAP.to_vec(),
        )
        .await
        .unwrap();
    store
        .claim_tool_step(
            request,
            "map",
            json!({"tool_name":"generate_map","args":{"prompt":"Fixture map"}}),
        )
        .await
        .unwrap();
    let execution = ToolExecution {
        result: json!({"status":"generated","media_ref":reference}),
        is_error: false,
        handled_by: "image-provider".into(),
        media: vec![GeneratedAgentMedia::Image {
            media_ref: Some(Box::new(reference.clone())),
            mime_type: "image/webp".into(),
            data_b64: String::new(),
            kind: "map".into(),
            source: app_application::ports::media::ImageSource::Generated,
        }],
    };
    store
        .finish_tool_step(request, "map", &execution)
        .await
        .unwrap();
    let selected = store.load_current(c.id).await.unwrap();
    assert_eq!(selected.projection.map_media_ref, Some(reference.clone()));
    assert!(matches!(
        store
            .claim_tool_step(
                request,
                "map",
                json!({"tool_name":"generate_map","args":{"prompt":"Fixture map"}})
            )
            .await
            .unwrap(),
        ToolStepClaim::Completed(_)
    ));
    assert_eq!(
        store.load_current(c.id).await.unwrap().campaign.revision,
        selected.campaign.revision
    );
    let raw: String = sqlx::query_scalar(
        "SELECT result_json FROM agent_tool_steps WHERE request_id=?1 AND tool_call_id='map'",
    )
    .bind(request.to_string())
    .fetch_one(&db)
    .await
    .unwrap();
    assert!(raw.len() < 2048);
    assert!(!raw.contains("data:image/"));
    store
        .claim_tool_step(
            request,
            "late-map",
            json!({"tool_name":"generate_map","args":{"prompt":"Late map"}}),
        )
        .await
        .unwrap();
    store
        .restore_snapshot(RestoreSnapshotRecord {
            save_id: saved.save_id,
            campaign_id: c.id,
            session_id: c.last_session_id,
            expected_revision: selected.campaign.revision,
            causation_id: "restore-before-map".into(),
        })
        .await
        .unwrap();
    assert!(store
        .load_current(c.id)
        .await
        .unwrap()
        .projection
        .map_media_ref
        .is_none());
    assert!(store
        .finish_tool_step(request, "late-map", &execution)
        .await
        .is_err());
    assert!(store
        .load_current(c.id)
        .await
        .unwrap()
        .projection
        .map_media_ref
        .is_none());
    assert_eq!(
        assets
            .read(id(&reference), &MediaReadScope::Campaign(c.id.0))
            .await
            .unwrap()
            .bytes,
        MAP
    );
}

#[tokio::test]
async fn legacy_solo_migration_keeps_original_until_commit_and_retries_exact_bytes() {
    use app_application::media_assets::migrate_portrait::{
        MigrateCampaignPortrait, MigratePortrait,
    };
    use base64::{engine::general_purpose::STANDARD, Engine};
    let db = SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&db).await.unwrap();
    let store = Arc::new(SqliteStore::new(db.clone()));
    let assets = media(store.clone());
    let c = campaign(store.clone()).await;
    let actor = Uuid::new_v4();
    let original = format!("data:image/webp;base64,{}", STANDARD.encode(PORTRAIT));
    let mut sheet = hero("Legacy hero");
    sheet["portraitUrl"] = json!(original);
    let before = store
        .commit_character(CommitCampaignCharacter {
            campaign_id: c.id,
            session_id: c.last_session_id,
            character_id: actor,
            expected_revision: c.revision,
            causation_id: "legacy-hero".into(),
            character: sheet,
        })
        .await
        .unwrap();
    let migration = MigrateCampaignPortrait::new(assets.clone(), store.clone());
    let command = MigratePortrait {
        campaign_id: c.id,
        character_id: actor,
        expected_revision: before.campaign.revision,
        request_id: Uuid::new_v4(),
    };
    sqlx::query("CREATE TRIGGER reject_media_commit BEFORE UPDATE ON campaign_characters WHEN json_extract(NEW.character_json,'$.portraitMediaRef') IS NOT NULL BEGIN SELECT RAISE(ABORT,'fixture write failed'); END").execute(&db).await.unwrap();
    assert!(migration.execute(command.clone()).await.is_err());
    assert_eq!(
        store.load_current(c.id).await.unwrap().projection.character["portraitUrl"],
        json!(original)
    );
    sqlx::query("DROP TRIGGER reject_media_commit")
        .execute(&db)
        .await
        .unwrap();
    let migrated = migration.execute(command.clone()).await.unwrap();
    assert!(migrated.changed);
    assert!(!migrated.retained);
    assert!(migrated.state.projection.character["portraitUrl"].is_null());
    assert_eq!(
        migrated.state.projection.character["hp"],
        before.projection.character["hp"]
    );
    assert_eq!(
        migrated.state.projection.character["backstory"],
        before.projection.character["backstory"]
    );
    let reference: MediaReference =
        serde_json::from_value(migrated.state.projection.character["portraitMediaRef"].clone())
            .unwrap();
    assert_eq!(
        assets
            .read(id(&reference), &MediaReadScope::Campaign(c.id.0))
            .await
            .unwrap()
            .bytes,
        PORTRAIT
    );
    let repeated = migration.execute(command).await.unwrap();
    assert!(!repeated.changed);
    assert_eq!(
        repeated.state.campaign.revision,
        migrated.state.campaign.revision
    );
    let unsupported = campaign(store.clone()).await;
    let unknown = Uuid::new_v4();
    let mut retained = hero("Retained hero");
    retained["portraitUrl"] = json!("https://example.org/unresolved.png");
    let state = store
        .commit_character(CommitCampaignCharacter {
            campaign_id: unsupported.id,
            session_id: unsupported.last_session_id,
            character_id: unknown,
            expected_revision: unsupported.revision,
            causation_id: "unsupported".into(),
            character: retained,
        })
        .await
        .unwrap();
    let result = migration
        .execute(MigratePortrait {
            campaign_id: unsupported.id,
            character_id: unknown,
            expected_revision: state.campaign.revision,
            request_id: Uuid::new_v4(),
        })
        .await
        .unwrap();
    assert!(result.retained);
    assert!(!result.changed);
    assert_eq!(
        result.state.projection.character["portraitUrl"],
        json!("https://example.org/unresolved.png")
    );
    assert_eq!(result.state.campaign.revision, state.campaign.revision);
}
