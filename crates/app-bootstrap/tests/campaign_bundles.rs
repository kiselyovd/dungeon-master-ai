use adapter_media::bundle_codec::DecodedRasterInspector;
use adapter_sqlite::{pool, SqliteStore};
use app_application::{
    campaign_bundles::CampaignBundles,
    campaigns::{CreateCampaign, CreateCampaignCommand},
    models::{
        campaign_bundle::*, campaign_state::CommitCampaignCharacter, chat::*, media_asset::*,
        session::CampaignSeed,
    },
    ports::{
        media_assets::MediaContentInspector,
        repositories::{CampaignStateRepository, MessageRepository},
    },
};
use serde_json::json;
use std::{io::Cursor, sync::Arc};
use uuid::Uuid;
async fn fixture() -> (
    Arc<SqliteStore>,
    app_application::models::session::CampaignDetail,
    Uuid,
) {
    let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&db).await.unwrap();
    let store = Arc::new(SqliteStore::new(db));
    let campaign = CreateCampaign::new(store.clone())
        .execute(CreateCampaignCommand {
            request_id: None,
            title: "Portable campaign".into(),
            premise: "Private seed premise".into(),
            seed: CampaignSeed::Bundled {
                adventure_id: app_application::models::adventure::BELL_ADVENTURE_ID.into(),
            },
        })
        .await
        .unwrap();
    let actor = Uuid::new_v4();
    store.commit_character(CommitCampaignCharacter{campaign_id:campaign.id,session_id:campaign.last_session_id,character_id:actor,expected_revision:campaign.revision,causation_id:"fixture-hero".into(),character:json!({"name":"Mira","heroClass":"fighter","level":1,"hp":9,"hpMax":12,"ac":14,"abilities":{"str":15,"dex":14,"con":13,"int":12,"wis":10,"cha":8},"inventory":[{"id":"rope","name":"Rope","count":2}],"backstory":"Private owned biography","spells":{"cantrips":[],"level1":[]},"provider_token":"must-not-export","reasoning":"must-not-export"})}).await.unwrap();
    (store, campaign, actor)
}
fn service(store: Arc<SqliteStore>) -> CampaignBundles<SqliteStore, DecodedRasterInspector> {
    CampaignBundles::new(store, Arc::new(DecodedRasterInspector))
}
async fn persisted_room_id(
    store: &SqliteStore,
    creation: &app_application::models::room::CreateLocalRoom,
) -> Uuid {
    let signature = serde_json::to_string(creation).unwrap();
    app_application::ports::repositories::RoomRepository::find_creation(
        store,
        creation.campaign_id,
        &creation.causation_id,
        &signature,
    )
    .await
    .unwrap()
    .expect("created public room record")
    .room_id
}
fn principal(actor: Uuid) -> ExportPrincipal {
    ExportPrincipal::SoloStorageOwner {
        own_actor: Some(actor),
    }
}
#[tokio::test]
async fn authoritative_backup_roundtrips_into_new_ids_without_changing_source_or_exporting_infrastructure(
) {
    let (store, campaign, actor) = fixture().await;
    use app_application::{
        models::adventure::{Action, AdventureCommand, Location},
        ports::repositories::AdventureRepository,
    };
    for (index, action) in [Action::Move(Location::Shrine), Action::ConsultLeon]
        .into_iter()
        .enumerate()
    {
        let current = store.load_current(campaign.id).await.unwrap();
        store
            .act(AdventureCommand {
                campaign_id: campaign.id,
                session_id: campaign.last_session_id,
                character_id: actor,
                expected_revision: current.campaign.revision,
                causation_id: format!("portable-world-{index}"),
                action,
            })
            .await
            .unwrap();
    }
    store
        .append(
            campaign.last_session_id.0,
            ChatMessage::System {
                content: "PRIVATE SYSTEM INSTRUCTIONS".into(),
            },
        )
        .await
        .unwrap();
    store
        .append(
            campaign.last_session_id.0,
            ChatMessage::user_text("Confirmed action"),
        )
        .await
        .unwrap();
    store
        .append(
            campaign.last_session_id.0,
            ChatMessage::ToolResult(ToolResult {
                tool_call_id: "private-tool".into(),
                content: "PRIVATE TOOL PAYLOAD".into(),
                is_error: false,
            }),
        )
        .await
        .unwrap();
    store
        .append(
            campaign.last_session_id.0,
            ChatMessage::Assistant {
                content: "Confirmed narration".into(),
            },
        )
        .await
        .unwrap();
    let before = store.load_current(campaign.id).await.unwrap();
    let bundle = service(store.clone())
        .export(
            campaign.id,
            principal(actor),
            BundleKind::FullCampaignBackup,
            None,
            1,
        )
        .await
        .unwrap();
    let bytes = bundle.encode().unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    for forbidden in [
        "PRIVATE SYSTEM",
        "PRIVATE TOOL",
        "provider_token",
        "must-not-export",
        "reasoning",
    ] {
        assert!(!text.contains(forbidden), "{forbidden}");
    }
    assert!(text.contains("Private owned biography"));
    assert_eq!(store.load_current(campaign.id).await.unwrap(), before);
    let imported = service(store.clone()).import_new(&bytes).await.unwrap();
    assert_ne!(imported.campaign_id, campaign.id);
    assert_ne!(imported.session_id, campaign.last_session_id);
    let current = store.load_current(imported.campaign_id).await.unwrap();
    assert_eq!(current.projection.character["hp"], 9);
    assert_eq!(current.projection.character["inventory"][0]["count"], 2);
    assert_eq!(
        current.projection.character["characterId"],
        imported.remapped_ids[&actor].to_string()
    );
    assert_eq!(current.projection.adventure, before.projection.adventure);
    assert_eq!(
        current.projection.adventuring,
        before.projection.adventuring
    );
    assert!(
        current
            .projection
            .adventuring
            .as_ref()
            .unwrap()
            .elapsed_seconds
            >= 900
    );
    assert_eq!(current.projection.messages.len(), 2);
    assert_eq!(store.load_current(campaign.id).await.unwrap(), before);
}
#[tokio::test]
async fn dossier_excludes_private_canon_and_unpublished_messages_and_cannot_import() {
    let (store, campaign, actor) = fixture().await;
    let private = store
        .append(
            campaign.last_session_id.0,
            ChatMessage::Assistant {
                content: "PRIVATE SOLO MESSAGE".into(),
            },
        )
        .await
        .unwrap();
    let public = store
        .append(
            campaign.last_session_id.0,
            ChatMessage::Assistant {
                content: "Published room history".into(),
            },
        )
        .await
        .unwrap();
    sqlx::query("INSERT INTO shared_messages(message_id,session_id) VALUES(?1,?2)")
        .bind(public.id.to_string())
        .bind(campaign.last_session_id.0.to_string())
        .execute(store.pool())
        .await
        .unwrap();
    let bundle = service(store.clone())
        .export(
            campaign.id,
            ExportPrincipal::Player { actor_id: actor },
            BundleKind::PlayerDossier,
            Some(actor),
            1,
        )
        .await
        .unwrap();
    let bytes = bundle.encode().unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.contains("Published room history"));
    assert!(!text.contains("PRIVATE SOLO MESSAGE"));
    assert!(!text.contains("Private seed premise"));
    assert!(!text.contains("package_id"));
    assert_ne!(private.id, public.id);
    assert!(matches!(
        service(store.clone()).import_new(&bytes).await,
        Err(BundleError::NotRestorable)
    ));
    assert!(matches!(
        service(store)
            .export(
                campaign.id,
                ExportPrincipal::Player { actor_id: actor },
                BundleKind::FullCampaignBackup,
                None,
                1
            )
            .await,
        Err(BundleError::ExportForbidden)
    ));
}
#[tokio::test]
async fn required_real_media_is_rebound_and_missing_corrupt_or_partial_writes_rollback() {
    let (store, campaign, actor) = fixture().await;
    let mut encoded = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image::RgbImage::new(2, 1))
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let pixels = encoded.into_inner();
    let descriptor = DecodedRasterInspector
        .inspect(&pixels, "image/png")
        .unwrap();
    let asset = Uuid::new_v4();
    let reference = MediaReference::Owned {
        asset_id: asset,
        category: MediaCategory::Portrait,
        origin: MediaOrigin::Uploaded,
        descriptor: descriptor.clone(),
    };
    let mut current = store.load_current(campaign.id).await.unwrap();
    current.projection.character["portraitMediaRef"] = serde_json::to_value(&reference).unwrap();
    sqlx::query("UPDATE campaign_characters SET character_json=?1 WHERE campaign_id=?2")
        .bind(current.projection.character.to_string())
        .bind(campaign.id.0.to_string())
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::query("INSERT INTO media_content(sha256,descriptor_json,payload) VALUES(?1,?2,?3)")
        .bind(&descriptor.sha256)
        .bind(serde_json::to_string(&descriptor).unwrap())
        .bind(&pixels)
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::query("INSERT INTO media_assets(asset_id,sha256,reference_json,owner_kind,owner_id,actor_id) VALUES(?1,?2,?3,'campaign',?4,NULL)").bind(asset.to_string()).bind(&descriptor.sha256).bind(serde_json::to_string(&reference).unwrap()).bind(campaign.id.0.to_string()).execute(store.pool()).await.unwrap();
    let bundle = service(store.clone())
        .export(
            campaign.id,
            principal(actor),
            BundleKind::FullCampaignBackup,
            None,
            1,
        )
        .await
        .unwrap();
    let mut invalid = bundle.clone();
    invalid.media.clear();
    assert!(matches!(invalid.encode(), Err(BundleError::MissingMedia)));
    invalid = bundle.clone();
    invalid.media[0].bytes[0] ^= 1;
    assert!(service(store.clone())
        .import_new(&invalid.encode().unwrap())
        .await
        .is_err());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM campaigns")
        .fetch_one(store.pool())
        .await
        .unwrap();
    sqlx::query("CREATE TRIGGER fail_bundle_import BEFORE INSERT ON campaign_snapshots BEGIN SELECT RAISE(ABORT,'fixture failure'); END").execute(store.pool()).await.unwrap();
    assert!(service(store.clone())
        .import_new(&bundle.encode().unwrap())
        .await
        .is_err());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM campaigns")
            .fetch_one(store.pool())
            .await
            .unwrap(),
        count
    );
    sqlx::query("DROP TRIGGER fail_bundle_import")
        .execute(store.pool())
        .await
        .unwrap();
    let imported = service(store.clone())
        .import_new(&bundle.encode().unwrap())
        .await
        .unwrap();
    let current = store.load_current(imported.campaign_id).await.unwrap();
    assert_eq!(
        current.projection.character["portraitMediaRef"]["asset_id"],
        imported.remapped_ids[&asset].to_string()
    );
    assert_eq!(sqlx::query_scalar::<_,Vec<u8>>("SELECT c.payload FROM media_assets a JOIN media_content c ON c.sha256=a.sha256 WHERE a.asset_id=?1").bind(imported.remapped_ids[&asset].to_string()).fetch_one(store.pool()).await.unwrap(),pixels);
}
async fn serve(router: axum::Router) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (format!("http://{address}"), task)
}
#[tokio::test]
async fn native_http_requires_owner_token_and_never_authorizes_a_body_role() {
    let (store, campaign, _) = fixture().await;
    let token = "c".repeat(64);
    let router = adapter_http::native_bundles::router(
        Arc::new(app_bootstrap::campaign_bundles::NativeBundles::new(
            store.clone(),
        )),
        token.clone(),
    )
    .unwrap();
    let (base, task) = serve(router).await;
    let client = reqwest::Client::new();
    let path = format!("{base}/native/campaigns/{}/bundle/export", campaign.id.0);
    assert_eq!(
        client
            .post(&path)
            .json(&json!({"kind":"full_campaign_backup","role":"host"}))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        client
            .post(&path)
            .bearer_auth("d".repeat(64))
            .json(&json!({"kind":"full_campaign_backup"}))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        client
            .post(&path)
            .bearer_auth(&token)
            .json(&json!({"kind":"full_campaign_backup","role":"host"}))
            .send()
            .await
            .unwrap()
            .status(),
        422
    );
    let response = client
        .post(&path)
        .bearer_auth(&token)
        .json(&json!({"kind":"full_campaign_backup"}))
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    assert_eq!(response.headers()["cache-control"], "no-store");
    let bytes = response.bytes().await.unwrap();
    let response = client
        .post(format!("{base}/native/campaigns/bundle/import"))
        .bearer_auth(&token)
        .body(bytes)
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    task.abort();
}
#[tokio::test]
async fn authenticated_guest_dossier_contains_only_own_sheet_and_is_denied_after_revocation() {
    use app_application::{
        models::{
            room::CreateLocalRoom,
            room_character::{ApproveRoomCharacter, SubmitRoomCharacter},
        },
        rooms::{JoinRoom, RoomSecret, RoomService, SystemRoomClock},
    };
    let (store, campaign, host_actor) = fixture().await;
    let rooms = Arc::new(
        RoomService::new(
            store.clone(),
            store.clone(),
            Arc::new(adapter_secrets::InMemorySecretsStore::default()),
            Arc::new(SystemRoomClock),
        )
        .with_dossiers(Arc::new(service(store.clone()))),
    );
    let current = store.load_current(campaign.id).await.unwrap();
    let creation = CreateLocalRoom {
        campaign_id: campaign.id,
        session_id: campaign.last_session_id,
        expected_revision: current.campaign.revision,
        causation_id: "fixture-room".into(),
    };
    let created = rooms.create_local_room(creation.clone()).await.unwrap();
    let room_id = persisted_room_id(&store, &creation).await;
    assert!(room_id == created.grant.room.room_id);
    let guest = rooms
        .join(JoinRoom {
            room_id: created.grant.room.room_id,
            join_id: Uuid::new_v4(),
            display_name: "Guest".into(),
            invitation: RoomSecret::from_input(created.invitation.expose().to_owned()).unwrap(),
            resume_proof: RoomSecret::generate(),
        })
        .await
        .unwrap();
    let subject = rooms
        .authenticate(guest.room.room_id, guest.member_id, &guest.capability)
        .await
        .unwrap();
    let host = rooms
        .authenticate(
            created.grant.room.room_id,
            created.grant.member_id,
            &created.grant.capability,
        )
        .await
        .unwrap();
    let actor = Uuid::new_v4();
    let proposal = Uuid::new_v4();
    let mut sheet = current.projection.character.clone();
    sheet["characterId"] = json!(actor);
    sheet["name"] = json!("Own guest");
    sheet["backstory"] = json!("Guest private biography");
    rooms
        .submit_character(
            &subject,
            SubmitRoomCharacter {
                proposal_id: proposal,
                character_id: actor,
                character: sheet,
            },
        )
        .await
        .unwrap();
    let latest = store.load_current(campaign.id).await.unwrap();
    rooms
        .approve_character(
            &host,
            ApproveRoomCharacter {
                proposal_id: proposal,
                expected_revision: latest.campaign.revision,
                causation_id: "accept-guest".into(),
            },
        )
        .await
        .unwrap();
    let (base, task) = serve(adapter_http::rooms::router(rooms.clone())).await;
    let client = reqwest::Client::new();
    let path = format!("{base}/rooms/{room_id}/dossier");
    let response = client
        .get(&path)
        .header("x-room-member-id", guest.member_id.to_string())
        .bearer_auth(guest.capability.expose())
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    let bytes = response.bytes().await.unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(text.contains("Guest private biography"));
    assert!(!text.contains("Private owned biography"));
    assert!(!text.contains("Private seed premise"));
    assert!(!text.contains("host_member_id"));
    assert!(!text.contains(&host_actor.to_string()));
    assert!(matches!(
        service(store.clone()).import_new(&bytes).await,
        Err(BundleError::NotRestorable)
    ));
    assert_eq!(
        client
            .post(format!(
                "{base}/native/campaigns/{}/bundle/export",
                campaign.id.0
            ))
            .header("x-room-member-id", guest.member_id.to_string())
            .bearer_auth(guest.capability.expose())
            .json(&json!({"kind":"full_campaign_backup"}))
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    let backup = service(store.clone())
        .export(
            campaign.id,
            principal(host_actor),
            BundleKind::FullCampaignBackup,
            None,
            1,
        )
        .await
        .unwrap();
    let imported = service(store.clone())
        .import_new(&backup.encode().unwrap())
        .await
        .unwrap();
    let imported_state = store.load_current(imported.campaign_id).await.unwrap();
    let party = imported_state.projection.party.unwrap();
    assert_eq!(party.members.len(), 1);
    assert_eq!(
        party.members[0].character["backstory"],
        "Guest private biography"
    );
    assert_eq!(party.members[0].character_id, imported.remapped_ids[&actor]);
    rooms.revoke(&host, guest.member_id).await.unwrap();
    assert_eq!(
        client
            .get(&path)
            .header("x-room-member-id", guest.member_id.to_string())
            .bearer_auth(guest.capability.expose())
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    task.abort();
}
#[tokio::test]
async fn combat_clock_geometry_and_spent_budgets_survive_with_remapped_scope() {
    use app_application::models::combat::CombatCampaignScope;
    use app_domain::combat::{
        combatant::Combatant,
        initiative::InitiativeEntry,
        types::{CharacterId, CombatantId, Position},
    };
    let (store, campaign, actor) = fixture().await;
    let before = store.load_current(campaign.id).await.unwrap();
    let encounter = Uuid::new_v4();
    let mut combat = app_application::combat::authored::bell_defense(encounter);
    let token = Uuid::new_v4();
    let mut hero = Combatant::new(CombatantId(token), "Mira".into(), 12, 9, 14);
    hero.current_hp = 9;
    hero.character_id = Some(CharacterId(actor));
    hero.position = Position { x: 2, y: 3 };
    hero.budget.action = false;
    hero.budget.movement_ft = 15;
    combat.snapshot.combatants.push(hero);
    combat.snapshot.initiative.insert(
        0,
        InitiativeEntry {
            id: CombatantId(token),
            roll: 20,
            dex_tiebreak: 2,
        },
    );
    combat.snapshot.current_combatant = Some(CombatantId(token));
    combat.snapshot.round = 3;
    combat.campaign_scope = Some(CombatCampaignScope {
        campaign_id: campaign.id.0,
        session_id: campaign.last_session_id.0,
        character_id: actor,
        revision: before.campaign.revision.0,
    });
    sqlx::query("INSERT INTO combat_encounters(id,session_id,started_at,initiative,projection,revision) VALUES(?1,?2,'2026','[]',?3,?4)").bind(encounter.to_string()).bind(campaign.last_session_id.0.to_string()).bind(serde_json::to_string(&combat).unwrap()).bind(combat.revision as i64).execute(store.pool()).await.unwrap();
    let backup = service(store.clone())
        .export(
            campaign.id,
            principal(actor),
            BundleKind::FullCampaignBackup,
            None,
            1,
        )
        .await
        .unwrap();
    let imported = service(store.clone())
        .import_new(&backup.encode().unwrap())
        .await
        .unwrap();
    let state = store.load_current(imported.campaign_id).await.unwrap();
    let restored: app_application::models::combat::CombatProjection =
        serde_json::from_value(state.projection.combat.unwrap()).unwrap();
    let scope = restored.campaign_scope.unwrap();
    assert_eq!(scope.campaign_id, imported.campaign_id.0);
    assert_eq!(scope.session_id, imported.session_id.0);
    assert_eq!(scope.character_id, imported.remapped_ids[&actor]);
    assert_eq!(scope.revision, state.campaign.revision.0);
    assert_eq!(restored.encounter_id, imported.remapped_ids[&encounter]);
    assert_eq!(restored.snapshot.round, 3);
    assert_eq!(
        restored.snapshot.current_combatant,
        Some(CombatantId(imported.remapped_ids[&token]))
    );
    let hero = restored
        .snapshot
        .combatants
        .iter()
        .find(|hero| hero.id.0 == imported.remapped_ids[&token])
        .unwrap();
    assert_eq!(hero.position, Position { x: 2, y: 3 });
    assert_eq!(hero.current_hp, 9);
    assert!(!hero.budget.action);
    assert_eq!(hero.budget.movement_ft, 15);
    assert_eq!(restored.authored.unwrap().map, combat.authored.unwrap().map);
}
#[tokio::test]
async fn forged_bundle_metadata_and_duplicate_identities_do_not_create_campaigns() {
    let (store, campaign, actor) = fixture().await;
    let original = store.load_current(campaign.id).await.unwrap();
    let bundle = service(store.clone())
        .export(
            campaign.id,
            principal(actor),
            BundleKind::FullCampaignBackup,
            None,
            1,
        )
        .await
        .unwrap();
    let value = serde_json::to_value(&bundle).unwrap();
    for (path, replacement) in [
        ("schema_version", json!(99)),
        ("ruleset_id", json!("wrong")),
        ("source_campaign_id", json!(Uuid::nil())),
    ] {
        let mut altered = value.clone();
        altered["header"][path] = replacement;
        assert!(service(store.clone())
            .import_new(&serde_json::to_vec(&altered).unwrap())
            .await
            .is_err());
    }
    let mut altered = value.clone();
    let row = json!({"id":Uuid::new_v4(),"entry_html":"Journal","created_at":"2026"});
    altered["payload"]["payload"]["projection"]["journal"] = json!([row.clone(), row]);
    assert!(service(store.clone())
        .import_new(&serde_json::to_vec(&altered).unwrap())
        .await
        .is_err());
    altered = value.clone();
    altered["payload"]["payload"]["projection"]["character"]["portraitMediaRef"] =
        json!({"source":"bundled","asset_id":"asset.nonexistent.v1","version":1});
    let mut unknown_asset: app_application::models::portable_campaign::PortableCampaignBundle =
        serde_json::from_value(altered).unwrap();
    unknown_asset.seal().unwrap();
    assert!(matches!(
        service(store.clone())
            .import_new(&unknown_asset.encode().unwrap())
            .await,
        Err(BundleError::MissingMedia)
    ));
    let mut tampered = value.clone();
    tampered["payload"]["payload"]["projection"]["character"]["hp"] = json!(8);
    assert!(matches!(
        service(store.clone())
            .import_new(&serde_json::to_vec(&tampered).unwrap())
            .await,
        Err(BundleError::IntegrityMismatch)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM campaigns")
            .fetch_one(store.pool())
            .await
            .unwrap(),
        1
    );
    assert_eq!(store.load_current(campaign.id).await.unwrap(), original);
}
#[tokio::test]
async fn same_import_operation_replays_one_result_and_changed_bytes_are_rejected() {
    let (store, campaign, actor) = fixture().await;
    let operation = Uuid::new_v4();
    let bundle = service(store.clone())
        .export(
            campaign.id,
            principal(actor),
            BundleKind::FullCampaignBackup,
            None,
            1,
        )
        .await
        .unwrap();
    let bytes = bundle.encode().unwrap();
    let first = service(store.clone())
        .import_with_operation(&bytes, operation)
        .await
        .unwrap();
    let second = service(store.clone())
        .import_with_operation(&bytes, operation)
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(first).unwrap(),
        serde_json::to_value(second).unwrap()
    );
    let mut changed = bundle;
    changed.header.exported_at_epoch_ms = 2;
    changed.seal().unwrap();
    assert!(matches!(
        service(store.clone())
            .import_with_operation(&changed.encode().unwrap(), operation)
            .await,
        Err(BundleError::ImportConflict)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM campaigns")
            .fetch_one(store.pool())
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM campaign_bundle_imports")
            .fetch_one(store.pool())
            .await
            .unwrap(),
        1
    );
}
#[tokio::test]
async fn attached_images_remain_typed_media_and_roundtrip_the_confirmed_history() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let (store, campaign, actor) = fixture().await;
    let mut encoded = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image::RgbImage::new(2, 1))
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let bytes = encoded.into_inner();
    store
        .append(
            campaign.last_session_id.0,
            ChatMessage::User {
                parts: vec![
                    MessagePart::Text {
                        text: "Look at the map".into(),
                    },
                    MessagePart::Image {
                        mime: "image/png".into(),
                        data_b64: STANDARD.encode(&bytes),
                        name: Some("map.png".into()),
                    },
                ],
            },
        )
        .await
        .unwrap();
    let original = store.load_current(campaign.id).await.unwrap();
    let bundle = service(store.clone())
        .export(
            campaign.id,
            principal(actor),
            BundleKind::FullCampaignBackup,
            None,
            1,
        )
        .await
        .unwrap();
    assert_eq!(bundle.media.len(), 1);
    let encoded = bundle.encode().unwrap();
    let imported = service(store.clone()).import_new(&encoded).await.unwrap();
    let restored = store.load_current(imported.campaign_id).await.unwrap();
    assert_eq!(restored.projection.messages, original.projection.messages);
}
#[tokio::test]
async fn unstarted_campaign_can_be_backed_up_without_inventing_a_character() {
    let (store, _, _) = fixture().await;
    let campaign = CreateCampaign::new(store.clone())
        .execute(CreateCampaignCommand {
            request_id: None,
            title: "Unstarted".into(),
            premise: "Empty".into(),
            seed: CampaignSeed::Collaborative {
                answers: Default::default(),
            },
        })
        .await
        .unwrap();
    let bundle = service(store.clone())
        .export(
            campaign.id,
            ExportPrincipal::SoloStorageOwner { own_actor: None },
            BundleKind::FullCampaignBackup,
            None,
            1,
        )
        .await
        .unwrap();
    let imported = service(store.clone())
        .import_new(&bundle.encode().unwrap())
        .await
        .unwrap();
    let current = store.load_current(imported.campaign_id).await.unwrap();
    assert!(current.character_id.is_none());
    assert_eq!(current.projection.character, json!({}));
}
