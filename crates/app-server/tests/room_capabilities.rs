use adapter_secrets::{InMemorySecretsStore, StrongholdSecretsStore};
use adapter_sqlite::{pool, SqliteStore};
use app_application::{
    campaigns::{CreateCampaign, CreateCampaignCommand},
    models::{
        campaign_state::CommitCampaignCharacter,
        party::{EnrollPartyCharacter, InitializeParty},
        room::CreateLocalRoom,
        room_character::{ApproveRoomCharacter, ProposalStatus, SubmitRoomCharacter},
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

async fn guest_proposal(
    store: &SqliteStore,
    service: &RoomService,
    created: &app_application::rooms::CreatedRoom,
) -> (
    app_application::rooms::RoomGrant,
    app_application::rooms::RoomSubject,
    SubmitRoomCharacter,
) {
    let grant = service
        .join(JoinRoom {
            room_id: created.grant.room.room_id,
            join_id: Uuid::new_v4(),
            display_name: "Guest".into(),
            invitation: secret(&created.invitation),
            resume_proof: RoomSecret::generate(),
        })
        .await
        .unwrap();
    let subject = service
        .authenticate(grant.room.room_id, grant.member_id, &grant.capability)
        .await
        .unwrap();
    let party = store
        .party(created.grant.room.campaign_id)
        .await
        .unwrap()
        .unwrap();
    let character_id = Uuid::new_v4();
    let mut character = party.characters[&party.roster.members[0].character_id].clone();
    character["characterId"] = json!(character_id);
    character["name"] = json!("Guest hero");
    character["backstory"] = json!("private-proposal-story");
    (
        grant,
        subject,
        SubmitRoomCharacter {
            proposal_id: Uuid::new_v4(),
            character_id,
            character,
        },
    )
}

async fn group_fight(
    store: &SqliteStore,
    service: &RoomService,
    created: &app_application::rooms::CreatedRoom,
) -> (
    app_application::rooms::RoomGrant,
    app_application::rooms::RoomSubject,
    app_application::rooms::RoomSubject,
    Uuid,
    app_application::models::combat::CombatProjection,
) {
    let (grant, guest, mut submission) = guest_proposal(store, service, created).await;
    submission.character["heroClass"] = json!("cleric");
    submission.character["abilities"]["wis"] = json!(16);
    submission.character["spells"] =
        json!({"cantrips":["sacred-flame"],"level1":["cure-wounds","healing-word"]});
    service
        .submit_character(&guest, submission.clone())
        .await
        .unwrap();
    let party = store
        .party(created.grant.room.campaign_id)
        .await
        .unwrap()
        .unwrap();
    let host = service
        .authenticate(
            created.grant.room.room_id,
            created.grant.member_id,
            &created.grant.capability,
        )
        .await
        .unwrap();
    service
        .approve_character(
            &host,
            ApproveRoomCharacter {
                proposal_id: submission.proposal_id,
                expected_revision: party.revision,
                causation_id: "group-fight".into(),
            },
        )
        .await
        .unwrap();
    let encounter = Uuid::new_v4();
    let npc = app_domain::combat::types::CombatantId::new();
    let projection = app_application::ports::repositories::CombatRepository::create(
        store,
        party.session_id.0,
        app_application::models::combat::CombatProjection {
            authored: None,
            adventure: None,
            campaign_scope: None,
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
    (grant, guest, host, encounter, projection)
}

#[tokio::test]
async fn room_spell_rolls_resources_and_all_party_effects_commit_or_rollback_together() {
    use app_application::{
        agent::hero_intentions::HeroCombatIntent, models::room_combat::RoomCombatCommand,
        ports::repositories::CombatRepository,
    };
    let (store, command) =
        fixture(sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap()).await;
    let service = RoomService::new(
        store.clone(),
        store.clone(),
        Arc::new(InMemorySecretsStore::default()),
        Arc::new(Clock(AtomicU64::new(1000))),
    );
    let created = service.create_local_room(command).await.unwrap();
    let (grant, guest, host, encounter, mut projection) =
        group_fight(&store, &service, &created).await;
    let party = store
        .party(created.grant.room.campaign_id)
        .await
        .unwrap()
        .unwrap();
    let guest_id = party
        .roster
        .members
        .iter()
        .find(|member| member.member_id == grant.member_id)
        .unwrap()
        .character_id;
    let root_id = party.roster.members[0].character_id;
    let root = projection
        .snapshot
        .combatants
        .iter_mut()
        .find(|actor| actor.character_id.is_some_and(|id| id.0 == root_id))
        .unwrap();
    root.apply_raw_damage(3);
    let root_token = root.id;
    projection.revision = 1;
    store.compare_and_set(0, projection.clone()).await.unwrap();
    let current = projection.snapshot.current_combatant.unwrap();
    if projection
        .snapshot
        .combatants
        .iter()
        .find(|actor| actor.id == current)
        .unwrap()
        .character_id
        .unwrap()
        .0
        != guest_id
    {
        projection = service
            .act_combat(
                &host,
                encounter,
                RoomCombatCommand {
                    request_id: "yield-to-cleric".into(),
                    expected_revision: 1,
                    action: HeroCombatIntent::EndTurn,
                },
            )
            .await
            .unwrap()
            .projection;
    }
    let before = store.party(party.campaign_id).await.unwrap().unwrap();
    let intent = RoomCombatCommand {
        request_id: "guest-heal".into(),
        expected_revision: projection.revision,
        action: HeroCombatIntent::Spell {
            spell_id: "cure-wounds".into(),
            targets: vec![root_token.0],
            slot_level: 1,
        },
    };
    sqlx::raw_sql("CREATE TRIGGER fail_room_receipt BEFORE INSERT ON combat_action_receipts BEGIN SELECT RAISE(ABORT,'synthetic_room_receipt_failure'); END;").execute(store.pool()).await.unwrap();
    assert!(service
        .act_combat(&guest, encounter, intent.clone())
        .await
        .is_err());
    assert!(store.party(party.campaign_id).await.unwrap().unwrap() == before);
    assert_eq!(
        store.get(encounter).await.unwrap().unwrap().revision,
        projection.revision
    );
    sqlx::raw_sql("DROP TRIGGER fail_room_receipt;")
        .execute(store.pool())
        .await
        .unwrap();
    let first = service
        .act_combat(&guest, encounter, intent.clone())
        .await
        .unwrap();
    let after = store.party(party.campaign_id).await.unwrap().unwrap();
    assert_eq!(after.characters[&root_id]["hp"], 12);
    assert_eq!(
        after.characters[&guest_id]["combatResources"]["spellSlots"]["1"],
        1
    );
    let replay = service.act_combat(&guest, encounter, intent).await.unwrap();
    assert!(replay.duplicate);
    assert_eq!(replay.receipt_revision, first.receipt_revision);
    assert!(
        serde_json::to_value(replay.receipt_events).unwrap()
            == serde_json::to_value(first.receipt_events).unwrap()
    );
    assert!(store.party(party.campaign_id).await.unwrap().unwrap() == after);
}

#[tokio::test]
async fn revoked_leases_and_foreign_encounters_cannot_read_or_commit_room_combat() {
    use app_application::{
        agent::hero_intentions::HeroCombatIntent, models::room_combat::RoomCombatCommand,
    };
    let (store, command) =
        fixture(sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap()).await;
    let service = RoomService::new(
        store.clone(),
        store.clone(),
        Arc::new(InMemorySecretsStore::default()),
        Arc::new(Clock(AtomicU64::new(1000))),
    );
    let created = service.create_local_room(command).await.unwrap();
    let (grant, guest, host, encounter, _) = group_fight(&store, &service, &created).await;
    let other = CreateCampaign::new(store.clone())
        .execute(CreateCampaignCommand {
            title: "Other".into(),
            premise: "Private other room".into(),
            seed: CampaignSeed::Bundled {
                adventure_id: "bell-beneath-water".into(),
            },
        })
        .await
        .unwrap();
    let foreign = Uuid::new_v4();
    let npc = app_domain::combat::types::CombatantId::new();
    app_application::ports::repositories::CombatRepository::create(
        &*store,
        other.last_session_id.0,
        app_application::models::combat::CombatProjection {
            authored: None,
            adventure: None,
            campaign_scope: None,
            schema_version: app_application::models::combat::COMBAT_PROJECTION_VERSION,
            encounter_id: foreign,
            revision: 0,
            snapshot: app_application::models::combat::CombatSnapshot {
                active: true,
                round: 1,
                current_combatant: Some(npc),
                initiative: vec![app_domain::combat::initiative::InitiativeEntry {
                    id: npc,
                    roll: 10,
                    dex_tiebreak: 0,
                }],
                combatants: vec![app_domain::combat::combatant::Combatant::new(
                    npc,
                    "Private foe".into(),
                    10,
                    10,
                    10,
                )],
            },
            events: vec![],
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        service.room_combat(&guest, foreign).await,
        Err(app_application::rooms::RoomError::Denied)
    ));
    let before = store
        .party(created.grant.room.campaign_id)
        .await
        .unwrap()
        .unwrap();
    service.revoke(&host, grant.member_id).await.unwrap();
    let stale = store
        .act_room_combat(
            host.lease(),
            encounter,
            RoomCombatCommand {
                request_id: "stale-role".into(),
                expected_revision: 0,
                action: HeroCombatIntent::EndTurn,
            },
            42,
        )
        .await;
    assert!(matches!(
        stale,
        Err(
            app_application::ports::repositories::RepositoryError::Operation {
                code: "room_access_denied",
                ..
            }
        )
    ));
    assert!(service.room_combat(&guest, encounter).await.is_err());
    assert!(store.party(before.campaign_id).await.unwrap().unwrap() == before);
    let receipts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM combat_action_receipts")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(receipts, 0);
}

#[tokio::test]
async fn restoring_no_combat_or_another_encounter_never_exposes_the_superseded_live_battle() {
    use app_application::{
        agent::hero_intentions::HeroCombatIntent,
        campaigns::{CreateSnapshot, CreateSnapshotCommand},
        models::{room_combat::RoomCombatCommand, snapshot::RestoreSnapshotRecord},
        ports::repositories::{CombatRepository, SnapshotRepository},
    };
    let (store, command) =
        fixture(sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap()).await;
    let service = RoomService::new(
        store.clone(),
        store.clone(),
        Arc::new(InMemorySecretsStore::default()),
        Arc::new(Clock(AtomicU64::new(1000))),
    );
    let created = service.create_local_room(command).await.unwrap();
    let (_, guest, _, a, initial) = group_fight(&store, &service, &created).await;
    let party = store
        .party(created.grant.room.campaign_id)
        .await
        .unwrap()
        .unwrap();
    let saved_a = CreateSnapshot::new(store.clone())
        .execute(CreateSnapshotCommand {
            id: Uuid::new_v4(),
            campaign_id: party.campaign_id,
            session_id: party.session_id,
            expected_revision: party.revision,
            title: "A".into(),
            kind: "manual".into(),
            summary: "".into(),
            tag: "".into(),
            causation_id: "save-a".into(),
            character: party.characters[&party.roster.members[0].character_id].clone(),
        })
        .await
        .unwrap();
    store.end(a).await.unwrap();
    let idle = store.party(party.campaign_id).await.unwrap().unwrap();
    let saved_idle = CreateSnapshot::new(store.clone())
        .execute(CreateSnapshotCommand {
            id: Uuid::new_v4(),
            campaign_id: party.campaign_id,
            session_id: party.session_id,
            expected_revision: idle.revision,
            title: "Idle".into(),
            kind: "manual".into(),
            summary: "".into(),
            tag: "".into(),
            causation_id: "save-idle".into(),
            character: idle.characters[&idle.roster.members[0].character_id].clone(),
        })
        .await
        .unwrap();
    let mut projection = initial;
    let b = Uuid::new_v4();
    let npc = app_domain::combat::types::CombatantId::new();
    projection.encounter_id = b;
    projection.campaign_scope = None;
    projection.events.clear();
    projection
        .snapshot
        .combatants
        .retain(|actor| actor.character_id.is_none());
    projection.snapshot.combatants[0].id = npc;
    projection.snapshot.current_combatant = Some(npc);
    projection.snapshot.initiative = vec![app_domain::combat::initiative::InitiativeEntry {
        id: npc,
        roll: -99,
        dex_tiebreak: 0,
    }];
    store.create(party.session_id.0, projection).await.unwrap();
    for (save_id, causation) in [
        (saved_idle.save_id, "restore-idle"),
        (saved_a.save_id, "restore-a"),
    ] {
        let current = store.party(party.campaign_id).await.unwrap().unwrap();
        store
            .restore_snapshot(RestoreSnapshotRecord {
                save_id,
                campaign_id: party.campaign_id,
                session_id: party.session_id,
                expected_revision: current.revision,
                causation_id: causation.into(),
            })
            .await
            .unwrap();
        assert!(matches!(
            service.room_combat(&guest, b).await,
            Err(app_application::rooms::RoomError::Conflict)
        ));
        assert!(matches!(
            service
                .act_combat(
                    &guest,
                    b,
                    RoomCombatCommand {
                        request_id: format!("old-{causation}"),
                        expected_revision: 0,
                        action: HeroCombatIntent::EndTurn
                    }
                )
                .await,
            Err(app_application::rooms::RoomError::Conflict)
        ));
    }
    assert!(
        service
            .room_combat(&guest, a)
            .await
            .unwrap()
            .snapshot
            .active
    );
}

#[tokio::test]
async fn party_long_rest_updates_all_heroes_once_and_receipt_failure_rolls_back_every_effect() {
    use app_application::{
        models::{rest::RestIntent, room_world::RoomRestCommand},
        ports::repositories::CombatRepository,
    };
    let (store, command) =
        fixture(sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap()).await;
    let service = RoomService::new(
        store.clone(),
        store.clone(),
        Arc::new(InMemorySecretsStore::default()),
        Arc::new(Clock(AtomicU64::new(1000))),
    );
    let created = service.create_local_room(command).await.unwrap();
    let (grant, guest, host, encounter, mut projection) =
        group_fight(&store, &service, &created).await;
    for actor in &mut projection.snapshot.combatants {
        if actor.character_id.is_some() {
            actor.apply_raw_damage(4);
            if let Some(slots) = actor.casting.as_mut().unwrap().slots.get_mut(&1) {
                slots.remaining = 0;
            }
        }
    }
    projection.revision = 1;
    store.compare_and_set(0, projection).await.unwrap();
    store.end(encounter).await.unwrap();
    let before_short = store
        .party(created.grant.room.campaign_id)
        .await
        .unwrap()
        .unwrap();
    let guest_id = before_short
        .roster
        .members
        .iter()
        .find(|member| member.member_id == grant.member_id)
        .unwrap()
        .character_id;
    service
        .rest(
            &guest,
            RoomRestCommand {
                request_id: "guest-hit-die".into(),
                expected_revision: before_short.revision,
                intent: RestIntent::Short { hit_dice: 1 },
            },
        )
        .await
        .unwrap();
    let after_short = store
        .party(created.grant.room.campaign_id)
        .await
        .unwrap()
        .unwrap();
    let root = before_short.roster.members[0].character_id;
    assert!(after_short.characters[&root] == before_short.characters[&root]);
    assert_eq!(
        after_short.characters[&guest_id]["combatResources"]["hitDiceRemaining"],
        0
    );
    assert_eq!(
        after_short.characters[&guest_id]["combatResources"]["spellSlots"]["1"],
        0
    );
    let before = store
        .party(created.grant.room.campaign_id)
        .await
        .unwrap()
        .unwrap();
    let before_seconds = service.rest_status(&host).await.unwrap().elapsed_seconds;
    let cursor = service.updates(&host, None).await.unwrap().unwrap().cursor;
    let command = RoomRestCommand {
        request_id: "long-rest".into(),
        expected_revision: before.revision,
        intent: RestIntent::Long,
    };
    assert!(matches!(
        service.rest(&guest, command.clone()).await,
        Err(app_application::rooms::RoomError::Denied)
    ));
    sqlx::raw_sql("CREATE TRIGGER fail_world_receipt BEFORE INSERT ON room_world_receipts BEGIN SELECT RAISE(ABORT,'synthetic_world_failure'); END;").execute(store.pool()).await.unwrap();
    assert!(service.rest(&host, command.clone()).await.is_err());
    assert!(store.party(before.campaign_id).await.unwrap().unwrap() == before);
    assert!(service
        .updates(&host, Some(cursor))
        .await
        .unwrap()
        .is_none());
    sqlx::raw_sql("DROP TRIGGER fail_world_receipt;")
        .execute(store.pool())
        .await
        .unwrap();
    let first = service.rest(&host, command.clone()).await.unwrap();
    assert_eq!(
        first.rest_outcome.as_ref().unwrap().elapsed_seconds,
        before_seconds + 8 * 3600
    );
    let after = store.party(before.campaign_id).await.unwrap().unwrap();
    assert_eq!(after.revision.0, before.revision.0 + 1);
    for sheet in after.characters.values() {
        assert_eq!(sheet["hp"], 12);
    }
    let guest_id = after
        .roster
        .members
        .iter()
        .find(|member| member.member_id == grant.member_id)
        .unwrap()
        .character_id;
    assert_eq!(
        after.characters[&guest_id]["combatResources"]["spellSlots"]["1"],
        2
    );
    assert!(service.rest(&host, command).await.unwrap().duplicate);
    assert!(store.party(before.campaign_id).await.unwrap().unwrap() == after);
    let repeat = RoomRestCommand {
        request_id: "too-soon".into(),
        expected_revision: after.revision,
        intent: RestIntent::Long,
    };
    assert!(matches!(
        service.rest(&host, repeat).await,
        Err(app_application::rooms::RoomError::ActionRejected)
    ));
    let status = service.rest_status(&guest).await.unwrap();
    assert_eq!(status.character_id, guest_id);
    assert_eq!(status.elapsed_seconds, before_seconds + 8 * 3600);
}

#[tokio::test]
async fn guest_reward_failure_rolls_back_shared_flags_inventory_clock_and_changes() {
    use app_application::{
        models::{
            adventure::{Action, Location},
            room_world::RoomAdventureCommand,
        },
        ports::repositories::CombatRepository,
    };
    let (store, command) =
        fixture(sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap()).await;
    let service = RoomService::new(
        store.clone(),
        store.clone(),
        Arc::new(InMemorySecretsStore::default()),
        Arc::new(Clock(AtomicU64::new(1000))),
    );
    let created = service.create_local_room(command).await.unwrap();
    let (_, guest, host, encounter, _) = group_fight(&store, &service, &created).await;
    store.end(encounter).await.unwrap();
    for (index, action) in [
        Action::Move(Location::Quay),
        Action::Move(Location::Archive),
        Action::LocateTikhon,
    ]
    .into_iter()
    .enumerate()
    {
        let current = store
            .party(created.grant.room.campaign_id)
            .await
            .unwrap()
            .unwrap();
        service
            .adventure(
                &guest,
                RoomAdventureCommand {
                    request_id: format!("prepare-{index}"),
                    expected_revision: current.revision,
                    action,
                },
            )
            .await
            .unwrap();
    }
    let before = store
        .party(created.grant.room.campaign_id)
        .await
        .unwrap()
        .unwrap();
    let frame = service.updates(&host, None).await.unwrap().unwrap();
    let rescue = RoomAdventureCommand {
        request_id: "rescue".into(),
        expected_revision: before.revision,
        action: Action::RescueTikhon,
    };
    sqlx::raw_sql("CREATE TRIGGER fail_reward_receipt BEFORE INSERT ON room_world_receipts BEGIN SELECT RAISE(ABORT,'synthetic_reward_failure'); END;").execute(store.pool()).await.unwrap();
    assert!(service.adventure(&guest, rescue.clone()).await.is_err());
    assert!(store.party(before.campaign_id).await.unwrap().unwrap() == before);
    assert!(service
        .updates(&host, Some(frame.cursor))
        .await
        .unwrap()
        .is_none());
    sqlx::raw_sql("DROP TRIGGER fail_reward_receipt;")
        .execute(store.pool())
        .await
        .unwrap();
    service.adventure(&guest, rescue.clone()).await.unwrap();
    let after = store.party(before.campaign_id).await.unwrap().unwrap();
    let status = service.rest_status(&guest).await.unwrap();
    assert_eq!(
        after.characters[&status.character_id]["inventory"][0]["count"],
        10
    );
    assert!(service.adventure(&guest, rescue).await.unwrap().duplicate);
    assert!(store.party(before.campaign_id).await.unwrap().unwrap() == after);
    let stale = guest.lease().clone();
    service
        .revoke(&host, guest.lease().member_id())
        .await
        .unwrap();
    assert!(store
        .act_room_adventure(
            &stale,
            RoomAdventureCommand {
                request_id: "stale".into(),
                expected_revision: after.revision,
                action: Action::RecoverLedger
            }
        )
        .await
        .is_err());
}

#[tokio::test]
async fn a_legacy_active_encounter_blocks_room_rest_status_and_world_changes() {
    use app_application::models::{
        adventure::{Action, Location},
        rest::RestIntent,
        room_world::{RoomAdventureCommand, RoomRestCommand},
    };
    let (store, command) =
        fixture(sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap()).await;
    let service = RoomService::new(
        store.clone(),
        store.clone(),
        Arc::new(InMemorySecretsStore::default()),
        Arc::new(Clock(AtomicU64::new(1000))),
    );
    let created = service.create_local_room(command).await.unwrap();
    let host = service
        .authenticate(
            created.grant.room.room_id,
            created.grant.member_id,
            &created.grant.capability,
        )
        .await
        .unwrap();
    let before = store
        .party(created.grant.room.campaign_id)
        .await
        .unwrap()
        .unwrap();
    sqlx::query("INSERT INTO combat_encounters(id,session_id,initiative,started_at,projection) VALUES(?1,?2,'[]','legacy',NULL)").bind(Uuid::new_v4().to_string()).bind(before.session_id.0.to_string()).execute(store.pool()).await.unwrap();
    assert!(service.rest_status(&host).await.unwrap().combat_active);
    for (index, intent) in [
        RestIntent::Short { hit_dice: 0 },
        RestIntent::Long,
        RestIntent::Wait { minutes: 60 },
    ]
    .into_iter()
    .enumerate()
    {
        assert!(matches!(
            service
                .rest(
                    &host,
                    RoomRestCommand {
                        request_id: format!("legacy-rest-{index}"),
                        expected_revision: before.revision,
                        intent
                    }
                )
                .await,
            Err(app_application::rooms::RoomError::ActionRejected)
        ));
    }
    assert!(matches!(
        service
            .adventure(
                &host,
                RoomAdventureCommand {
                    request_id: "legacy-travel".into(),
                    expected_revision: before.revision,
                    action: Action::Move(Location::Quay)
                }
            )
            .await,
        Err(app_application::rooms::RoomError::ActionRejected)
    ));
    assert!(store.party(before.campaign_id).await.unwrap().unwrap() == before);
}

#[tokio::test]
async fn a_near_budget_guest_sheet_cannot_be_corrupted_by_derived_rest_resources() {
    use app_application::models::{
        campaign_state::{within_character_budget, MAX_CHARACTER_METADATA_BYTES},
        rest::RestIntent,
        room_world::RoomRestCommand,
    };
    let (store, command) =
        fixture(sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap()).await;
    let service = RoomService::new(
        store.clone(),
        store.clone(),
        Arc::new(InMemorySecretsStore::default()),
        Arc::new(Clock(AtomicU64::new(1000))),
    );
    let created = service.create_local_room(command).await.unwrap();
    let (_, guest, mut proposal) = guest_proposal(&store, &service, &created).await;
    proposal.character["backstory"] = json!("");
    let base = serde_json::to_vec(&proposal.character).unwrap().len();
    proposal.character["backstory"] = json!("A".repeat(MAX_CHARACTER_METADATA_BYTES - base));
    assert!(within_character_budget(&proposal.character));
    service
        .submit_character(&guest, proposal.clone())
        .await
        .unwrap();
    let current = store
        .party(created.grant.room.campaign_id)
        .await
        .unwrap()
        .unwrap();
    let host = service
        .authenticate(
            created.grant.room.room_id,
            created.grant.member_id,
            &created.grant.capability,
        )
        .await
        .unwrap();
    service
        .approve_character(
            &host,
            ApproveRoomCharacter {
                proposal_id: proposal.proposal_id,
                expected_revision: current.revision,
                causation_id: "accept-near-budget".into(),
            },
        )
        .await
        .unwrap();
    let before = store.party(current.campaign_id).await.unwrap().unwrap();
    let frame = service.updates(&host, None).await.unwrap().unwrap();
    assert!(service
        .rest(
            &guest,
            RoomRestCommand {
                request_id: "oversize-rest".into(),
                expected_revision: before.revision,
                intent: RestIntent::Short { hit_dice: 0 }
            }
        )
        .await
        .is_err());
    assert!(store.party(current.campaign_id).await.unwrap().unwrap() == before);
    assert!(service
        .updates(&host, Some(frame.cursor))
        .await
        .unwrap()
        .is_none());
    let npc = app_domain::combat::types::CombatantId::new();
    let projection = app_application::models::combat::CombatProjection {
        authored: None,
        adventure: None,
        campaign_scope: None,
        schema_version: app_application::models::combat::COMBAT_PROJECTION_VERSION,
        encounter_id: Uuid::new_v4(),
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
    };
    assert!(
        app_application::ports::repositories::CombatRepository::create(
            &*store,
            before.session_id.0,
            projection
        )
        .await
        .is_err()
    );
    assert!(store.party(current.campaign_id).await.unwrap().unwrap() == before);
    assert!(service
        .updates(&host, Some(frame.cursor))
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn approval_failure_rolls_back_hero_revision_status_and_both_receipts() {
    let (store, command) =
        fixture(sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap()).await;
    let service = RoomService::new(
        store.clone(),
        store.clone(),
        Arc::new(InMemorySecretsStore::default()),
        Arc::new(Clock(AtomicU64::new(1000))),
    );
    let created = service.create_local_room(command).await.unwrap();
    let (_, guest, submission) = guest_proposal(&store, &service, &created).await;
    service
        .submit_character(&guest, submission.clone())
        .await
        .unwrap();
    let before = store
        .party(created.grant.room.campaign_id)
        .await
        .unwrap()
        .unwrap();
    let host = service
        .authenticate(
            created.grant.room.room_id,
            created.grant.member_id,
            &created.grant.capability,
        )
        .await
        .unwrap();
    let approval = ApproveRoomCharacter {
        proposal_id: submission.proposal_id,
        expected_revision: before.revision,
        causation_id: "approve-once".into(),
    };
    sqlx::raw_sql("CREATE TRIGGER reject_approval_receipt BEFORE INSERT ON room_character_approvals BEGIN SELECT RAISE(ABORT,'synthetic_approval_failure'); END;").execute(store.pool()).await.unwrap();
    assert!(service
        .approve_character(&host, approval.clone())
        .await
        .is_err());
    let after = store.party(before.campaign_id).await.unwrap().unwrap();
    assert!(before == after);
    assert_eq!(
        service
            .own_proposal(&guest)
            .await
            .unwrap()
            .unwrap()
            .summary
            .status,
        ProposalStatus::Pending
    );
    let receipts:i64=sqlx::query_scalar("SELECT (SELECT COUNT(*) FROM room_character_approvals)+(SELECT COUNT(*) FROM party_operation_receipts WHERE causation_id LIKE 'room-approve:%')").fetch_one(store.pool()).await.unwrap();
    assert_eq!(receipts, 0);
    sqlx::raw_sql("DROP TRIGGER reject_approval_receipt;")
        .execute(store.pool())
        .await
        .unwrap();
    let accepted = service
        .approve_character(&host, approval.clone())
        .await
        .unwrap();
    assert_eq!(accepted.revision.0, before.revision.0 + 1);
    assert_eq!(
        service.approve_character(&host, approval).await.unwrap(),
        accepted
    );
    assert_eq!(
        store
            .party(before.campaign_id)
            .await
            .unwrap()
            .unwrap()
            .roster
            .members
            .len(),
        2
    );
}

#[tokio::test]
async fn superseded_proposals_and_revoked_members_cannot_be_approved() {
    let (store, command) =
        fixture(sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap()).await;
    let service = RoomService::new(
        store.clone(),
        store.clone(),
        Arc::new(InMemorySecretsStore::default()),
        Arc::new(Clock(AtomicU64::new(1000))),
    );
    let created = service.create_local_room(command).await.unwrap();
    let (grant, guest, submission) = guest_proposal(&store, &service, &created).await;
    service
        .submit_character(&guest, submission.clone())
        .await
        .unwrap();
    let mut changed = submission.clone();
    changed.character["name"] = json!("Changed under old UUID");
    assert!(matches!(
        service.submit_character(&guest, changed).await,
        Err(app_application::rooms::RoomError::Conflict)
    ));
    let mut replacement = submission.clone();
    replacement.proposal_id = Uuid::new_v4();
    replacement.character["name"] = json!("Revised hero");
    service
        .submit_character(&guest, replacement.clone())
        .await
        .unwrap();
    assert_eq!(
        service
            .own_proposal(&guest)
            .await
            .unwrap()
            .unwrap()
            .summary
            .proposal_id,
        replacement.proposal_id
    );
    let host = service
        .authenticate(
            created.grant.room.room_id,
            created.grant.member_id,
            &created.grant.capability,
        )
        .await
        .unwrap();
    let before = store
        .party(created.grant.room.campaign_id)
        .await
        .unwrap()
        .unwrap();
    let mut approval = ApproveRoomCharacter {
        proposal_id: submission.proposal_id,
        expected_revision: before.revision,
        causation_id: "approve-superseded".into(),
    };
    assert!(service
        .approve_character(&host, approval.clone())
        .await
        .is_err());
    approval.proposal_id = replacement.proposal_id;
    approval.causation_id = "approve-revoked".into();
    service.revoke(&host, grant.member_id).await.unwrap();
    assert!(store
        .approve_character_proposal(host.lease(), approval.clone())
        .await
        .is_err());
    let fresh_host = service
        .authenticate(
            created.grant.room.room_id,
            created.grant.member_id,
            &created.grant.capability,
        )
        .await
        .unwrap();
    assert!(service
        .approve_character(&fresh_host, approval)
        .await
        .is_err());
    assert!(service.own_proposal(&guest).await.is_err());
    assert!(service
        .pending_proposals(&fresh_host)
        .await
        .unwrap()
        .is_empty());
    assert!(store.party(before.campaign_id).await.unwrap().unwrap() == before);
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
    character["backstory"] = json!(format!("guest-private-backstory{}", "A".repeat(9 * 1024)));
    let proposal_id = Uuid::new_v4();
    let submission =
        json!({"proposal_id":proposal_id,"character_id":character_id,"character":character});
    let submitted = client
        .post(format!("{url}/character-proposals"))
        .bearer_auth(capability)
        .header("x-room-member-id", member)
        .json(&submission)
        .send()
        .await
        .unwrap();
    assert_eq!(submitted.status(), 200);
    let submitted = submitted.text().await.unwrap();
    assert!(!submitted.contains("guest-private-backstory"));
    let replay = client
        .post(format!("{url}/character-proposals"))
        .bearer_auth(capability)
        .header("x-room-member-id", member)
        .json(&submission)
        .send()
        .await
        .unwrap();
    assert_eq!(replay.status(), 200);
    let private_draft = client
        .get(format!("{url}/character-proposals/own"))
        .bearer_auth(capability)
        .header("x-room-member-id", member)
        .send()
        .await
        .unwrap();
    assert_eq!(private_draft.status(), 200);
    assert!(private_draft
        .text()
        .await
        .unwrap()
        .contains("guest-private-backstory"));
    let guest_list = client
        .get(format!("{url}/character-proposals"))
        .bearer_auth(capability)
        .header("x-room-member-id", member)
        .send()
        .await
        .unwrap();
    assert_eq!(guest_list.status(), 401);
    let host_list = client
        .get(format!("{url}/character-proposals"))
        .bearer_auth(created.grant.capability.expose())
        .header("x-room-member-id", created.grant.member_id.to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(host_list.status(), 200);
    let host_list = host_list.text().await.unwrap();
    assert!(host_list.contains("Guest hero"));
    assert!(!host_list.contains("guest-private-backstory"));
    let approval =
        json!({"expected_revision":party.revision.0,"causation_id":"http-party-acceptance"});
    let approval_url = format!("{url}/character-proposals/{proposal_id}/approve");
    let guest_approval = client
        .post(&approval_url)
        .bearer_auth(capability)
        .header("x-room-member-id", member)
        .json(&approval)
        .send()
        .await
        .unwrap();
    assert_eq!(guest_approval.status(), 401);
    for _ in 0..2 {
        let accepted = client
            .post(&approval_url)
            .bearer_auth(created.grant.capability.expose())
            .header("x-room-member-id", created.grant.member_id.to_string())
            .json(&approval)
            .send()
            .await
            .unwrap();
        assert_eq!(accepted.status(), 200);
        let accepted: serde_json::Value = accepted.json().await.unwrap();
        assert_eq!(accepted["revision"], party.revision.0 + 1);
    }
    let changed_approval = client.post(&approval_url).bearer_auth(created.grant.capability.expose()).header("x-room-member-id", created.grant.member_id.to_string()).json(&json!({"expected_revision":party.revision.0 + 1,"causation_id":"http-party-acceptance"})).send().await.unwrap();
    assert_eq!(changed_approval.status(), 409);
    let accepted_party = store.party(party.campaign_id).await.unwrap().unwrap();
    assert_eq!(accepted_party.roster.members.len(), 2);
    assert_eq!(
        accepted_party.roster.members[1].member_id,
        Uuid::parse_str(member).unwrap()
    );
    let mut latest = accepted_party.clone();
    let mut rescue_request = None;
    for (index, action) in [
        app_application::models::adventure::Action::Move(
            app_application::models::adventure::Location::Quay,
        ),
        app_application::models::adventure::Action::Move(
            app_application::models::adventure::Location::Archive,
        ),
        app_application::models::adventure::Action::LocateTikhon,
        app_application::models::adventure::Action::RescueTikhon,
        app_application::models::adventure::Action::Move(
            app_application::models::adventure::Location::Quay,
        ),
        app_application::models::adventure::Action::Move(
            app_application::models::adventure::Location::Town,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let request = json!({"request_id":format!("guest-adventure-{index}"),"expected_revision":latest.revision.0,"action":action});
        if index == 3 {
            rescue_request = Some(request.clone());
        }
        let applied = client
            .post(format!("{url}/adventure/actions"))
            .bearer_auth(capability)
            .header("x-room-member-id", member)
            .json(&request)
            .send()
            .await
            .unwrap();
        assert_eq!(applied.status(), 200);
        assert!(!applied
            .text()
            .await
            .unwrap()
            .contains("host-private-backstory"));
        latest = store.party(party.campaign_id).await.unwrap().unwrap();
    }
    assert!(
        latest.characters[&party.roster.members[0].character_id]["inventory"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        latest.characters[&character_id]["inventory"][0]["count"],
        10
    );
    let rescue_retry = client
        .post(format!("{url}/adventure/actions"))
        .bearer_auth(capability)
        .header("x-room-member-id", member)
        .json(&rescue_request.unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(rescue_retry.status(), 200);
    assert!(store.party(party.campaign_id).await.unwrap().unwrap() == latest);
    let short = json!({"request_id":"guest-short-rest","expected_revision":latest.revision.0,"intent":{"kind":"short","hit_dice":0}});
    let rested = client
        .post(format!("{url}/rest"))
        .bearer_auth(capability)
        .header("x-room-member-id", member)
        .json(&short)
        .send()
        .await
        .unwrap();
    assert_eq!(rested.status(), 200);
    let after_short = store.party(party.campaign_id).await.unwrap().unwrap();
    let replay = client
        .post(format!("{url}/rest"))
        .bearer_auth(capability)
        .header("x-room-member-id", member)
        .json(&short)
        .send()
        .await
        .unwrap();
    assert_eq!(replay.status(), 200);
    assert!(store.party(party.campaign_id).await.unwrap().unwrap() == after_short);
    let guest_long=client.post(format!("{url}/rest")).bearer_auth(capability).header("x-room-member-id",member).json(&json!({"request_id":"guest-long","expected_revision":after_short.revision.0,"intent":{"kind":"long"}})).send().await.unwrap();
    assert_eq!(guest_long.status(), 401);
    let encounter = Uuid::new_v4();
    let npc = app_domain::combat::types::CombatantId::new();
    let fight = app_application::ports::repositories::CombatRepository::create(
        &*store,
        party.session_id.0,
        app_application::models::combat::CombatProjection {
            authored: None,
            adventure: None,
            campaign_scope: None,
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
    let combat_url = format!("{url}/combat/{encounter}");
    let guest_combat = client
        .get(&combat_url)
        .bearer_auth(capability)
        .header("x-room-member-id", member)
        .send()
        .await
        .unwrap();
    assert_eq!(guest_combat.status(), 200);
    let guest_combat: serde_json::Value = guest_combat.json().await.unwrap();
    for actor in guest_combat["snapshot"]["combatants"].as_array().unwrap() {
        if actor["character_id"] == json!(character_id) {
            assert!(actor["loadout"].is_object());
        } else if actor["character_id"].is_string() {
            assert!(actor["loadout"].is_null());
            assert!(actor["casting"].is_null());
        }
    }
    let current = fight.snapshot.current_combatant.unwrap();
    let current_character = fight
        .snapshot
        .combatants
        .iter()
        .find(|actor| actor.id == current)
        .unwrap()
        .character_id
        .unwrap()
        .0;
    let host_member = created.grant.member_id.to_string();
    let (first_cap, first_member, second_cap, second_member) = if current_character == character_id
    {
        (
            capability,
            member,
            created.grant.capability.expose(),
            created.grant.member_id.to_string(),
        )
    } else {
        (
            created.grant.capability.expose(),
            host_member.as_str(),
            capability,
            member.to_string(),
        )
    };
    let wrong_turn = client
        .post(format!("{combat_url}/actions"))
        .bearer_auth(second_cap)
        .header("x-room-member-id", &second_member)
        .json(
            &json!({"request_id":"wrong-turn","expected_revision":0,"action":{"kind":"end_turn"}}),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(wrong_turn.status(), 422);
    let forged=client.post(format!("{combat_url}/actions")).bearer_auth(second_cap).header("x-room-member-id",&second_member).json(&json!({"request_id":"forged","expected_revision":0,"action":{"kind":"end_turn","combatant":current.0}})).send().await.unwrap();
    assert_eq!(forged.status(), 422);
    let intent = json!({"request_id":"same-client-nonce","expected_revision":0,"action":{"kind":"end_turn"}});
    for duplicate in [false, true] {
        let applied = client
            .post(format!("{combat_url}/actions"))
            .bearer_auth(first_cap)
            .header("x-room-member-id", first_member)
            .json(&intent)
            .send()
            .await
            .unwrap();
        assert_eq!(applied.status(), 200);
        let applied: serde_json::Value = applied.json().await.unwrap();
        assert_eq!(applied["duplicate"], duplicate);
        assert_eq!(applied["projection"]["revision"], 1);
    }
    let changed=client.post(format!("{combat_url}/actions")).bearer_auth(first_cap).header("x-room-member-id",first_member).json(&json!({"request_id":"same-client-nonce","expected_revision":1,"action":{"kind":"end_turn"}})).send().await.unwrap();
    assert_eq!(changed.status(), 409);
    let second=client.post(format!("{combat_url}/actions")).bearer_auth(second_cap).header("x-room-member-id",&second_member).json(&json!({"request_id":"same-client-nonce","expected_revision":1,"action":{"kind":"end_turn"}})).send().await.unwrap();
    assert_eq!(second.status(), 200);
    let second: serde_json::Value = second.json().await.unwrap();
    assert_eq!(second["projection"]["revision"], 2);
    let late = client
        .post(format!("{combat_url}/actions"))
        .bearer_auth(first_cap)
        .header("x-room-member-id", first_member)
        .json(&intent)
        .send()
        .await
        .unwrap();
    assert_eq!(late.status(), 200);
    let late: serde_json::Value = late.json().await.unwrap();
    assert_eq!(late["projection"]["revision"], 2);
    assert_eq!(late["receipt_revision"], 1);
    let mut finished =
        app_application::ports::repositories::CombatRepository::get(&*store, encounter)
            .await
            .unwrap()
            .unwrap();
    for actor in &mut finished.snapshot.combatants {
        if actor.character_id.is_none() {
            actor.apply_raw_damage(actor.max_hp);
        }
    }
    finished.snapshot.active = false;
    finished.snapshot.current_combatant = None;
    finished.revision = 3;
    app_application::ports::repositories::CombatRepository::compare_and_set(&*store, 2, finished)
        .await
        .unwrap();
    let terminal = client
        .post(format!("{combat_url}/actions"))
        .bearer_auth(first_cap)
        .header("x-room-member-id", first_member)
        .json(&intent)
        .send()
        .await
        .unwrap();
    assert_eq!(terminal.status(), 200);
    let terminal: serde_json::Value = terminal.json().await.unwrap();
    assert_eq!(terminal["projection"]["revision"], 3);
    assert_eq!(terminal["projection"]["snapshot"]["active"], false);
    assert_eq!(terminal["receipt_revision"], 1);
    let fresh_invite = client
        .post(format!("{url}/invitation"))
        .bearer_auth(created.grant.capability.expose())
        .header("x-room-member-id", created.grant.member_id.to_string())
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    let third=client.post(format!("{url}/join")).json(&json!({"join_id":Uuid::new_v4(),"display_name":"Third","invitation":fresh_invite["invitation"],"resume_proof":RoomSecret::generate().expose()})).send().await.unwrap();
    assert_eq!(third.status(), 200);
    let third: serde_json::Value = third.json().await.unwrap();
    let third_member = third["member_id"].as_str().unwrap();
    let third_cap = third["capability"].as_str().unwrap();
    let third_character = Uuid::new_v4();
    let third_proposal = Uuid::new_v4();
    let mut third_sheet = submission["character"].clone();
    third_sheet["characterId"] = json!(third_character);
    third_sheet["backstory"] = json!("third-private-story");
    let third_submission=client.post(format!("{url}/character-proposals")).bearer_auth(third_cap).header("x-room-member-id",third_member).json(&json!({"proposal_id":third_proposal,"character_id":third_character,"character":third_sheet})).send().await.unwrap();
    assert_eq!(third_submission.status(), 200);
    let before_third = store.party(party.campaign_id).await.unwrap().unwrap();
    let third_approval=client.post(format!("{url}/character-proposals/{third_proposal}/approve")).bearer_auth(created.grant.capability.expose()).header("x-room-member-id",created.grant.member_id.to_string()).json(&json!({"expected_revision":before_third.revision.0,"causation_id":"accept-third-after-combat"})).send().await.unwrap();
    assert_eq!(third_approval.status(), 200);
    let historical = client
        .post(format!("{combat_url}/actions"))
        .bearer_auth(first_cap)
        .header("x-room-member-id", first_member)
        .json(&intent)
        .send()
        .await
        .unwrap();
    assert_eq!(historical.status(), 200);
    let historical: serde_json::Value = historical.json().await.unwrap();
    assert_eq!(historical["projection"]["snapshot"]["active"], false);
    assert_eq!(historical["receipt_revision"], 1);
    let not_in_old_encounter = client
        .get(&combat_url)
        .bearer_auth(third_cap)
        .header("x-room-member-id", third_member)
        .send()
        .await
        .unwrap();
    assert_eq!(not_in_old_encounter.status(), 401);
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
    let guest = service
        .authenticate(joined.room.room_id, joined.member_id, &joined.capability)
        .await
        .unwrap();
    let party = store
        .party(created.grant.room.campaign_id)
        .await
        .unwrap()
        .unwrap();
    let character_id = Uuid::new_v4();
    let mut character = party.characters[&party.roster.members[0].character_id].clone();
    character["characterId"] = json!(character_id);
    character["backstory"] = json!("durable-private-proposal");
    let draft = SubmitRoomCharacter {
        proposal_id: Uuid::new_v4(),
        character_id,
        character,
    };
    service
        .submit_character(&guest, draft.clone())
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
    let guest = reopened
        .authenticate(joined.room.room_id, joined.member_id, &joined.capability)
        .await
        .unwrap();
    let recovered = reopened.own_proposal(&guest).await.unwrap().unwrap();
    assert_eq!(recovered.summary.proposal_id, draft.proposal_id);
    assert_eq!(recovered.summary.status, ProposalStatus::Pending);
    assert!(recovered.character == draft.character);
    let host = reopened
        .authenticate(
            created.grant.room.room_id,
            created.grant.member_id,
            &created.grant.capability,
        )
        .await
        .unwrap();
    let approval = ApproveRoomCharacter {
        proposal_id: draft.proposal_id,
        expected_revision: party.revision,
        causation_id: "approve-after-reopen".into(),
    };
    let approved = reopened
        .approve_character(&host, approval.clone())
        .await
        .unwrap();
    assert_eq!(approved.proposal.status, ProposalStatus::Accepted);
    assert_eq!(
        reopened.approve_character(&host, approval).await.unwrap(),
        approved
    );
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

#[tokio::test]
async fn retained_room_heroes_do_not_enter_new_solo_fights_after_revocation_or_close() {
    use app_application::ports::repositories::CombatRepository;
    for close in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(directory.path().join("room.db"))
            .create_if_missing(true);
        let (store, command) = fixture(
            sqlx::SqlitePool::connect_with(options.clone())
                .await
                .unwrap(),
        )
        .await;
        let service = RoomService::new(
            store.clone(),
            store.clone(),
            Arc::new(InMemorySecretsStore::default()),
            Arc::new(Clock(AtomicU64::new(1000))),
        );
        let created = service.create_local_room(command).await.unwrap();
        let (grant, _, host, encounter, mut old) = group_fight(&store, &service, &created).await;
        let party = store
            .party(created.grant.room.campaign_id)
            .await
            .unwrap()
            .unwrap();
        let host_id = party
            .roster
            .members
            .iter()
            .find(|m| m.member_id == party.roster.host_member_id)
            .unwrap()
            .character_id;
        let guest_id = party
            .roster
            .members
            .iter()
            .find(|m| m.member_id == grant.member_id)
            .unwrap()
            .character_id;
        old.snapshot.active = false;
        old.revision = 1;
        store.compare_and_set(0, old.clone()).await.unwrap();
        if close {
            service.close(&host).await.unwrap();
        } else {
            service.revoke(&host, grant.member_id).await.unwrap();
        }
        let archived = store.party(party.campaign_id).await.unwrap().unwrap();
        let guest_before = archived.characters[&guest_id].clone();
        let historical: String =
            sqlx::query_scalar("SELECT projection FROM combat_encounters WHERE id=?1")
                .bind(encounter.to_string())
                .fetch_one(store.pool())
                .await
                .unwrap();
        let historical: app_application::models::combat::CombatProjection =
            serde_json::from_str(&historical).unwrap();
        assert_eq!(
            historical.snapshot.combatants.len(),
            old.snapshot.combatants.len()
        );
        let npc = app_domain::combat::types::CombatantId::new();
        let next = Uuid::new_v4();
        let mut projection = app_application::models::combat::CombatProjection::empty(next, 0);
        projection.snapshot = app_application::models::combat::CombatSnapshot {
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
        };
        let created_fight = store.create(party.session_id.0, projection).await.unwrap();
        let ids = created_fight
            .snapshot
            .combatants
            .iter()
            .filter_map(|a| a.character_id.map(|id| id.0))
            .collect::<Vec<_>>();
        assert_eq!(ids, vec![host_id], "closed={close}");
        let mut forged = created_fight.clone();
        forged
            .snapshot
            .combatants
            .retain(|a| a.character_id.is_none());
        forged.campaign_scope = None;
        forged.revision = 1;
        assert!(store.compare_and_set(0, forged).await.is_err());
        assert_eq!(store.get(next).await.unwrap().unwrap().revision, 0);

        assert_eq!(
            store
                .get(next)
                .await
                .unwrap()
                .unwrap()
                .snapshot
                .combatants
                .len(),
            2
        );
        assert_eq!(
            store
                .party(party.campaign_id)
                .await
                .unwrap()
                .unwrap()
                .characters[&guest_id],
            guest_before
        );
        use app_application::{
            campaigns::{CreateSnapshot, CreateSnapshotCommand},
            models::snapshot::RestoreSnapshotRecord,
            ports::repositories::SnapshotRepository,
        };
        let current = store.load_current(party.campaign_id).await.unwrap();
        let saved = CreateSnapshot::new(store.clone())
            .execute(CreateSnapshotCommand {
                id: Uuid::new_v4(),
                campaign_id: party.campaign_id,
                session_id: party.session_id,
                expected_revision: current.campaign.revision,
                title: "Solo after coop".into(),
                kind: "manual".into(),
                summary: "".into(),
                tag: "".into(),
                causation_id: "solo-after-coop-save".into(),
                character: current.projection.character,
            })
            .await
            .unwrap();
        let current = store.load_current(party.campaign_id).await.unwrap();
        store
            .restore_snapshot(RestoreSnapshotRecord {
                save_id: saved.save_id,
                campaign_id: party.campaign_id,
                session_id: party.session_id,
                expected_revision: current.campaign.revision,
                causation_id: "solo-after-coop-restore".into(),
            })
            .await
            .unwrap();
        assert_eq!(
            store
                .get(next)
                .await
                .unwrap()
                .unwrap()
                .snapshot
                .combatants
                .len(),
            2
        );
        store.pool().close().await;
        let reopened = SqliteStore::new(sqlx::SqlitePool::connect_with(options).await.unwrap());
        assert_eq!(
            reopened
                .get(next)
                .await
                .unwrap()
                .unwrap()
                .snapshot
                .combatants
                .len(),
            2
        );
        assert_eq!(
            reopened
                .party(party.campaign_id)
                .await
                .unwrap()
                .unwrap()
                .characters[&guest_id],
            guest_before
        );
        reopened.pool().close().await;
    }
}

#[tokio::test]
async fn group_rest_ignores_retained_incapacitated_revoked_hero_and_replays_once() {
    use app_application::models::{rest::RestIntent, room_world::RoomRestCommand};
    use app_application::ports::repositories::CombatRepository;
    let (store, command) =
        fixture(sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap()).await;
    let service = RoomService::new(
        store.clone(),
        store.clone(),
        Arc::new(InMemorySecretsStore::default()),
        Arc::new(Clock(AtomicU64::new(1000))),
    );
    let created = service.create_local_room(command).await.unwrap();
    let (grant, _, host, _, mut old) = group_fight(&store, &service, &created).await;
    old.snapshot.active = false;
    old.revision = 1;
    store.compare_and_set(0, old).await.unwrap();
    let party = store
        .party(created.grant.room.campaign_id)
        .await
        .unwrap()
        .unwrap();
    let guest_id = party
        .roster
        .members
        .iter()
        .find(|m| m.member_id == grant.member_id)
        .unwrap()
        .character_id;
    let mut guest = party.characters[&guest_id].clone();
    guest["hp"] = json!(0);
    sqlx::query("UPDATE party_characters SET character_json=?1 WHERE character_id=?2")
        .bind(guest.to_string())
        .bind(guest_id.to_string())
        .execute(store.pool())
        .await
        .unwrap();
    service.revoke(&host, grant.member_id).await.unwrap();
    let host = service
        .authenticate(
            created.grant.room.room_id,
            created.grant.member_id,
            &created.grant.capability,
        )
        .await
        .unwrap();
    let current = store.load_current(party.campaign_id).await.unwrap();
    let command = RoomRestCommand {
        request_id: "present-only-rest".into(),
        expected_revision: current.campaign.revision,
        intent: RestIntent::Long,
    };
    let receipt = service.rest(&host, command.clone()).await.unwrap();
    assert!(!receipt.duplicate);
    let after = store.party(party.campaign_id).await.unwrap().unwrap();
    assert_eq!(after.characters[&guest_id], guest);
    assert!(service.rest(&host, command).await.unwrap().duplicate);
    assert_eq!(
        store.party(party.campaign_id).await.unwrap().unwrap(),
        after
    );
}
