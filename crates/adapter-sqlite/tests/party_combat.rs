use adapter_sqlite::{pool, SqliteStore};
use app_application::{
    campaigns::{CreateCampaign, CreateCampaignCommand},
    models::{
        campaign_state::CommitCampaignCharacter,
        combat::{CombatProjection, CombatSnapshot, COMBAT_PROJECTION_VERSION},
        party::{EnrollPartyCharacter, InitializeParty},
        session::CampaignSeed,
    },
    ports::repositories::{CampaignStateRepository, CombatRepository, PartyRepository},
};
use app_domain::combat::{combatant::Combatant, initiative::InitiativeEntry, types::CombatantId};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

async fn fixture() -> (
    Arc<SqliteStore>,
    app_application::models::party::PartyState,
    Uuid,
) {
    let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    fixture_with_pool(db).await
}
async fn fixture_with_pool(
    db: sqlx::SqlitePool,
) -> (
    Arc<SqliteStore>,
    app_application::models::party::PartyState,
    Uuid,
) {
    fixture_with_party_size(db, 2).await
}
async fn fixture_with_party_size(
    db: sqlx::SqlitePool,
    players: usize,
) -> (
    Arc<SqliteStore>,
    app_application::models::party::PartyState,
    Uuid,
) {
    pool::migrate(&db).await.unwrap();
    let store = Arc::new(SqliteStore::new(db));
    let campaign = CreateCampaign::new(store.clone())
        .execute(CreateCampaignCommand {
            request_id: None,
            title: "Party combat".into(),
            premise: "Same-name heroes".into(),
            seed: CampaignSeed::Bundled {
                adventure_id: "bell-beneath-water".into(),
            },
        })
        .await
        .unwrap();
    let host_id = Uuid::new_v4();
    let state=store.commit_character(CommitCampaignCharacter {
        campaign_id:campaign.id,session_id:campaign.last_session_id,character_id:host_id,
        expected_revision:campaign.revision,causation_id:"host".into(),
        character:json!({"name":"Mira","heroClass":"fighter","level":1,"hp":10,"hpMax":20,"ac":20,"abilities":{"str":16,"dex":30,"con":14,"int":10,"wis":16,"cha":8},"inventory":[{"id":"fighter-longbow","name":"Longbow","count":1},{"id":"fighter-arrows-20","name":"Arrows","count":1}]}),
    }).await.unwrap();
    let party = store
        .initialize_party(InitializeParty {
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            expected_revision: state.campaign.revision,
            causation_id: "party".into(),
        })
        .await
        .unwrap();
    let guest_id = Uuid::new_v4();
    let mut party=store.enroll_party_character(EnrollPartyCharacter {
        campaign_id:party.campaign_id,session_id:party.session_id,expected_revision:party.revision,
        causation_id:"guest".into(),verified_host_member_id:party.roster.host_member_id,member_id:Uuid::new_v4(),character_id:guest_id,
        character:json!({"name":"Mira","heroClass":"cleric","level":1,"hp":8,"hpMax":20,"ac":20,"abilities":{"str":12,"dex":30,"con":14,"int":10,"wis":16,"cha":8},"inventory":[],"spells":{"cantrips":["sacred-flame"],"level1":["cure-wounds","healing-word"]}}),
    }).await.unwrap();
    for _ in 2..players {
        let character_id = Uuid::new_v4();
        let mut character = party.characters[&guest_id].clone();
        character["characterId"] = json!(character_id);
        party = store
            .enroll_party_character(EnrollPartyCharacter {
                campaign_id: party.campaign_id,
                session_id: party.session_id,
                expected_revision: party.revision,
                causation_id: format!("guest-{character_id}"),
                verified_host_member_id: party.roster.host_member_id,
                member_id: Uuid::new_v4(),
                character_id,
                character,
            })
            .await
            .unwrap();
    }
    let encounter = Uuid::new_v4();
    let npc = CombatantId::new();
    store
        .create(
            party.session_id.0,
            CombatProjection {
                authored: None,
                adventure: None,
                campaign_scope: None,
                schema_version: COMBAT_PROJECTION_VERSION,
                encounter_id: encounter,
                revision: 0,
                snapshot: CombatSnapshot {
                    active: true,
                    round: 1,
                    current_combatant: Some(npc),
                    initiative: vec![InitiativeEntry {
                        id: npc,
                        roll: 0,
                        dex_tiebreak: 0,
                    }],
                    combatants: vec![Combatant::new(npc, "Mira".into(), 100, 100, 10)],
                },
                events: vec![],
            },
        )
        .await
        .unwrap();
    (store, party, encounter)
}

#[tokio::test]
async fn npc_automation_stops_for_each_hero_and_advances_shared_time_once_per_round() {
    use app_application::combat::{
        commands::{CombatActionCommand, ResolveCombatCommand},
        resolve::ResolveCombatAction,
    };
    let (store, party, encounter) = fixture().await;
    let initial = store.get(encounter).await.unwrap().unwrap();
    let first = initial.snapshot.current_combatant.unwrap();
    let step = ResolveCombatAction::new(store.clone())
        .execute(ResolveCombatCommand {
            request_id: "first-player-turn".into(),
            encounter_id: encounter,
            expected_revision: 0,
            rng_seed: 42,
            action: CombatActionCommand::EndTurn { combatant: first },
        })
        .await
        .unwrap();
    let second = step.projection.snapshot.current_combatant.unwrap();
    assert_ne!(first, second);
    assert!(step
        .projection
        .snapshot
        .combatants
        .iter()
        .find(|actor| actor.id == second)
        .unwrap()
        .character_id
        .is_some());
    assert_eq!(step.projection.snapshot.round, 1);
    let next = ResolveCombatAction::new(store.clone())
        .execute(ResolveCombatCommand {
            request_id: "second-player-turn".into(),
            encounter_id: encounter,
            expected_revision: 1,
            rng_seed: 43,
            action: CombatActionCommand::EndTurn { combatant: second },
        })
        .await
        .unwrap();
    assert_eq!(next.projection.snapshot.current_combatant, Some(first));
    assert_eq!(next.projection.snapshot.round, 2);
    assert_eq!(
        store
            .load_current(party.campaign_id)
            .await
            .unwrap()
            .projection
            .adventuring
            .unwrap()
            .elapsed_seconds,
        6
    );
    let saved = store.party(party.campaign_id).await.unwrap().unwrap();
    for actor in next
        .projection
        .snapshot
        .combatants
        .iter()
        .filter(|actor| actor.character_id.is_some())
    {
        assert_eq!(
            saved.characters[&actor.character_id.unwrap().0]["hp"],
            actor.current_hp
        );
    }
}

#[tokio::test]
async fn four_same_name_heroes_each_receive_a_turn_before_the_next_shared_round() {
    use app_application::combat::{
        commands::{CombatActionCommand, ResolveCombatCommand},
        resolve::ResolveCombatAction,
    };
    let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let (store, party, encounter) = fixture_with_party_size(db, 4).await;
    let mut projection = store.get(encounter).await.unwrap().unwrap();
    let first = projection.snapshot.current_combatant.unwrap();
    let mut seen = std::collections::BTreeSet::new();
    for turn in 0..4 {
        let actor = projection.snapshot.current_combatant.unwrap();
        assert!(seen.insert(actor.0));
        assert!(projection
            .snapshot
            .combatants
            .iter()
            .find(|candidate| candidate.id == actor)
            .unwrap()
            .character_id
            .is_some());
        assert_eq!(projection.snapshot.round, 1);
        projection = ResolveCombatAction::new(store.clone())
            .execute(ResolveCombatCommand {
                request_id: format!("four-player-turn-{turn}"),
                encounter_id: encounter,
                expected_revision: projection.revision,
                rng_seed: 42 + turn,
                action: CombatActionCommand::EndTurn { combatant: actor },
            })
            .await
            .unwrap()
            .projection;
    }
    assert_eq!(projection.snapshot.current_combatant, Some(first));
    assert_eq!(projection.snapshot.round, 2);
    let current = store.party(party.campaign_id).await.unwrap().unwrap();
    assert_eq!(current.revision.0, party.revision.0 + 5);
    for actor in projection
        .snapshot
        .combatants
        .iter()
        .filter(|actor| actor.character_id.is_some())
    {
        assert_eq!(
            current.characters[&actor.character_id.unwrap().0]["hp"],
            actor.current_hp
        );
    }
    assert_eq!(
        store
            .load_current(party.campaign_id)
            .await
            .unwrap()
            .projection
            .adventuring
            .unwrap()
            .elapsed_seconds,
        6
    );
}

#[tokio::test]
async fn missing_duplicate_and_foreign_bindings_cannot_change_any_party_sheet() {
    let (store, party, encounter) = fixture().await;
    let original = store.get(encounter).await.unwrap().unwrap();
    let before = store.party(party.campaign_id).await.unwrap().unwrap();
    for kind in 0..3 {
        let mut changed = original.clone();
        changed.revision = 1;
        let guest = changed
            .snapshot
            .combatants
            .iter()
            .position(|actor| {
                actor
                    .character_id
                    .is_some_and(|id| id.0 == party.roster.members[1].character_id)
            })
            .unwrap();
        if kind == 0 {
            changed.snapshot.combatants.remove(guest);
        } else {
            changed.snapshot.combatants[guest].character_id =
                Some(app_domain::combat::types::CharacterId(if kind == 1 {
                    party.roster.members[0].character_id
                } else {
                    Uuid::new_v4()
                }));
        }
        assert!(store
            .commit_action(0, changed, &format!("bad-binding-{kind}"), "{}")
            .await
            .is_err());
        assert!(store.party(party.campaign_id).await.unwrap().unwrap() == before);
        assert_eq!(store.get(encounter).await.unwrap().unwrap().revision, 0);
    }
}

#[tokio::test]
async fn real_reopen_and_legacy_profile_restore_use_every_saved_party_sheet() {
    use app_application::{
        campaigns::{CreateSnapshot, CreateSnapshotCommand},
        models::snapshot::RestoreSnapshotRecord,
        ports::repositories::SnapshotRepository,
    };
    let path = std::env::temp_dir().join(format!("dmai-party-combat-{}.sqlite", Uuid::new_v4()));
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(&path)
        .create_if_missing(true);
    let db = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options.clone())
        .await
        .unwrap();
    let (store, party, encounter) = fixture_with_pool(db).await;
    let mut projection = store.get(encounter).await.unwrap().unwrap();
    let tokens = projection
        .snapshot
        .combatants
        .iter()
        .filter_map(|actor| actor.character_id.map(|id| (id.0, actor.id)))
        .collect::<std::collections::BTreeMap<_, _>>();
    for actor in &mut projection.snapshot.combatants {
        if let Some(id) = actor.character_id {
            actor.apply_raw_damage(3);
            if id.0 == party.roster.members[0].character_id {
                actor.loadout.as_mut().unwrap().arrows -= 1;
            } else {
                actor
                    .casting
                    .as_mut()
                    .unwrap()
                    .slots
                    .get_mut(&1)
                    .unwrap()
                    .remaining -= 1;
            }
        }
    }
    projection.revision = 1;
    projection.snapshot.round = 2;
    projection = store
        .commit_action(0, projection, "before-save", "{}")
        .await
        .unwrap()
        .projection;
    let saved_party = store.party(party.campaign_id).await.unwrap().unwrap();
    let saved = CreateSnapshot::new(store.clone())
        .execute(CreateSnapshotCommand {
            id: Uuid::new_v4(),
            campaign_id: party.campaign_id,
            session_id: party.session_id,
            expected_revision: saved_party.revision,
            title: "Party checkpoint".into(),
            kind: "manual".into(),
            summary: "".into(),
            tag: "combat".into(),
            causation_id: "party-save".into(),
            character: saved_party.characters[&party.roster.members[0].character_id].clone(),
        })
        .await
        .unwrap();
    let mut legacy = serde_json::to_value(&saved.snapshot.projection).unwrap();
    for actor in legacy["combat"]["snapshot"]["combatants"]
        .as_array_mut()
        .unwrap()
    {
        if actor["character_id"].is_string() {
            actor.as_object_mut().unwrap().remove("loadout");
            actor.as_object_mut().unwrap().remove("casting");
        }
    }
    sqlx::query("UPDATE campaign_snapshots SET projection_json=?2 WHERE id=?1")
        .bind(saved.snapshot.id.to_string())
        .bind(legacy.to_string())
        .execute(store.pool())
        .await
        .unwrap();
    for actor in &mut projection.snapshot.combatants {
        if let Some(id) = actor.character_id {
            actor.apply_raw_damage(1);
            if id.0 == party.roster.members[0].character_id {
                actor.loadout.as_mut().unwrap().arrows -= 1;
            } else {
                actor
                    .casting
                    .as_mut()
                    .unwrap()
                    .slots
                    .get_mut(&1)
                    .unwrap()
                    .remaining -= 1;
            }
        }
    }
    projection.revision = 2;
    projection.snapshot.round = 3;
    store
        .commit_action(1, projection, "after-save", "{}")
        .await
        .unwrap();
    let latest = store.party(party.campaign_id).await.unwrap().unwrap();
    store.pool().close().await;
    drop(store);
    let db = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    let reopened = SqliteStore::new(db);
    assert!(reopened.party(party.campaign_id).await.unwrap().unwrap() == latest);
    reopened
        .restore_snapshot(RestoreSnapshotRecord {
            save_id: saved.save_id,
            campaign_id: party.campaign_id,
            session_id: party.session_id,
            expected_revision: latest.revision,
            causation_id: "party-restore".into(),
        })
        .await
        .unwrap();
    let restored = reopened.get(encounter).await.unwrap().unwrap();
    let after = reopened.party(party.campaign_id).await.unwrap().unwrap();
    assert!(after.characters == saved_party.characters);
    assert_eq!(restored.snapshot.round, 2);
    for actor in restored
        .snapshot
        .combatants
        .iter()
        .filter(|actor| actor.character_id.is_some())
    {
        let id = actor.character_id.unwrap().0;
        assert_eq!(actor.id, tokens[&id]);
        if id == party.roster.members[0].character_id {
            assert_eq!(actor.loadout.as_ref().unwrap().arrows, 19);
        } else {
            assert_eq!(actor.casting.as_ref().unwrap().slots[&1].remaining, 1);
        }
    }
    assert_eq!(
        reopened
            .load_current(party.campaign_id)
            .await
            .unwrap()
            .projection
            .adventuring
            .unwrap()
            .elapsed_seconds,
        6
    );
    reopened.pool().close().await;
    drop(reopened);
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn every_party_hero_gets_a_distinct_token_and_resources_commit_once_for_the_group() {
    let (store, party, encounter) = fixture().await;
    let mut projection = store.get(encounter).await.unwrap().unwrap();
    let heroes = projection
        .snapshot
        .combatants
        .iter()
        .filter(|actor| actor.character_id.is_some())
        .collect::<Vec<_>>();
    assert_eq!(heroes.len(), 2);
    assert_ne!(heroes[0].id, heroes[1].id);
    assert_ne!(heroes[0].position, heroes[1].position);
    assert!(heroes
        .iter()
        .all(|hero| hero.name == "Mira" && hero.id.0 != hero.character_id.unwrap().0));
    for actor in &mut projection.snapshot.combatants {
        if let Some(id) = actor.character_id {
            actor.apply_raw_damage(3);
            if id.0 == party.roster.members[0].character_id {
                actor.loadout.as_mut().unwrap().arrows -= 1;
            } else {
                actor
                    .casting
                    .as_mut()
                    .unwrap()
                    .slots
                    .get_mut(&1)
                    .unwrap()
                    .remaining -= 1;
            }
        }
    }
    projection.snapshot.round = 2;
    projection.revision = 1;
    let committed = store
        .commit_action(0, projection.clone(), "party-damage", "{}")
        .await
        .unwrap();
    assert!(!committed.duplicate);
    let after = store.party(party.campaign_id).await.unwrap().unwrap();
    assert_eq!(after.revision.0, party.revision.0 + 2);
    assert_eq!(
        after.characters[&party.roster.members[0].character_id]["hp"],
        7
    );
    assert_eq!(
        after.characters[&party.roster.members[0].character_id]["combatResources"]["arrows"],
        19
    );
    assert_eq!(
        after.characters[&party.roster.members[1].character_id]["hp"],
        5
    );
    assert_eq!(
        after.characters[&party.roster.members[1].character_id]["combatResources"]["spellSlots"]
            ["1"],
        1
    );
    let current = store.load_current(party.campaign_id).await.unwrap();
    assert_eq!(current.projection.adventuring.unwrap().elapsed_seconds, 6);
    assert!(
        store
            .commit_action(0, projection, "party-damage", "{}")
            .await
            .unwrap()
            .duplicate
    );
    assert!(store.party(party.campaign_id).await.unwrap().unwrap() == after);
}

#[tokio::test]
async fn guest_write_failure_rolls_back_host_guest_clock_encounter_and_receipt() {
    let (store, party, encounter) = fixture().await;
    let before = store.party(party.campaign_id).await.unwrap().unwrap();
    let mut projection = store.get(encounter).await.unwrap().unwrap();
    assert_eq!(
        projection
            .snapshot
            .combatants
            .iter()
            .filter(|actor| actor.character_id.is_some())
            .count(),
        2
    );
    for actor in &mut projection.snapshot.combatants {
        if actor.character_id.is_some() {
            actor.apply_raw_damage(3);
        }
    }
    projection.revision = 1;
    projection.snapshot.round = 2;
    sqlx::raw_sql("CREATE TRIGGER fail_guest_combat BEFORE UPDATE OF character_json ON party_characters BEGIN SELECT RAISE(ABORT,'synthetic_guest_failure'); END;").execute(store.pool()).await.unwrap();
    assert!(store
        .commit_action(0, projection, "failed-party", "{}")
        .await
        .is_err());
    assert!(store.party(party.campaign_id).await.unwrap().unwrap() == before);
    assert_eq!(store.get(encounter).await.unwrap().unwrap().revision, 0);
    assert!(store
        .find_action(encounter, "failed-party", "{}")
        .await
        .unwrap()
        .is_none());
    assert_eq!(
        store
            .load_current(party.campaign_id)
            .await
            .unwrap()
            .projection
            .adventuring
            .unwrap_or_default()
            .elapsed_seconds,
        0
    );
}

#[tokio::test]
async fn ended_combat_vitality_and_recovery_deadline_survive_snapshot_and_real_reopen() {
    use app_application::vitality;
    use app_application::{
        campaigns::{CreateSnapshot, CreateSnapshotCommand},
        models::snapshot::RestoreSnapshotRecord,
        ports::repositories::SnapshotRepository,
    };
    use app_domain::combat::conditions::Condition;
    let path = std::env::temp_dir().join(format!("dmai-vitality-{}.sqlite", Uuid::new_v4()));
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(&path)
        .create_if_missing(true);
    let db = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options.clone())
        .await
        .unwrap();
    let (store, party, encounter) = fixture_with_pool(db).await;
    let host = party.roster.members[0].character_id;
    let guest = party.roster.members[1].character_id;
    let mut projection = store.get(encounter).await.unwrap().unwrap();
    for actor in &mut projection.snapshot.combatants {
        if let Some(character) = actor.character_id {
            actor.current_hp = 0;
            actor.add_condition(Condition::Unconscious);
            if character.0 == host {
                actor.is_dead = true;
                actor.death_saves.failures = 3;
            } else {
                actor.death_saves.stabilize();
            }
        }
    }
    projection.snapshot.active = false;
    projection.revision = 1;
    store
        .commit_action(0, projection.clone(), "ended-vitality", "{}")
        .await
        .unwrap();
    let saved_party = store.party(party.campaign_id).await.unwrap().unwrap();
    let deadline = vitality::read(&saved_party.characters[&guest], guest)
        .unwrap()
        .unwrap()
        .natural_recovery
        .unwrap();
    assert_eq!(deadline.started_at_seconds, 6);
    let saved = CreateSnapshot::new(store.clone())
        .execute(CreateSnapshotCommand {
            id: Uuid::new_v4(),
            campaign_id: party.campaign_id,
            session_id: party.session_id,
            expected_revision: saved_party.revision,
            title: "Vitality".into(),
            kind: "manual".into(),
            summary: "".into(),
            tag: "exploration".into(),
            causation_id: "vitality-save".into(),
            character: saved_party.characters[&host].clone(),
        })
        .await
        .unwrap();
    let mut healed = saved_party.characters[&guest].clone();
    vitality::clear_after_healing(&mut healed, 3).unwrap();
    healed["hp"] = json!(3);
    sqlx::query(
        "UPDATE party_characters SET character_json=?1 WHERE campaign_id=?2 AND character_id=?3",
    )
    .bind(healed.to_string())
    .bind(party.campaign_id.0.to_string())
    .bind(guest.to_string())
    .execute(store.pool())
    .await
    .unwrap();
    let revision = store
        .load_current(party.campaign_id)
        .await
        .unwrap()
        .campaign
        .revision;
    store.pool().close().await;
    drop(store);
    let db = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    let reopened = SqliteStore::new(db);
    reopened
        .restore_snapshot(RestoreSnapshotRecord {
            save_id: saved.save_id,
            campaign_id: party.campaign_id,
            session_id: party.session_id,
            expected_revision: revision,
            causation_id: "vitality-restore".into(),
        })
        .await
        .unwrap();
    let restored = reopened.party(party.campaign_id).await.unwrap().unwrap();
    let root = vitality::read(&restored.characters[&host], host)
        .unwrap()
        .unwrap();
    assert!(root.is_dead);
    assert_eq!(root.death_saves.failures, 3);
    assert_eq!(restored.characters[&host]["hp"], 0);
    let state = vitality::read(&restored.characters[&guest], guest)
        .unwrap()
        .unwrap();
    assert!(state.death_saves.stable);
    assert_eq!(state.natural_recovery, Some(deadline));
    assert_eq!(restored.characters[&guest]["hp"], 0);
    assert!(reopened
        .load_current(party.campaign_id)
        .await
        .unwrap()
        .projection
        .combat
        .is_none());
}

#[tokio::test]
async fn combat_stabilization_schedules_once_and_replays_without_rebasing_the_deadline() {
    use app_application::combat::{
        commands::{CombatActionCommand, ResolveCombatCommand},
        resolve::ResolveCombatAction,
    };
    use app_domain::{
        combat::conditions::Condition,
        dice::{roll_one, Die},
        rng::SeededRng,
    };
    let (store, party, encounter) = fixture().await;
    let mut projection = store.get(encounter).await.unwrap().unwrap();
    let heroes: Vec<_> = projection
        .snapshot
        .combatants
        .iter()
        .filter(|a| a.character_id.is_some())
        .map(|a| a.id)
        .collect();
    let (first, dying) = (heroes[0], heroes[1]);
    let id = projection
        .snapshot
        .combatants
        .iter()
        .find(|a| a.id == dying)
        .unwrap()
        .character_id
        .unwrap()
        .0;
    let actor = projection
        .snapshot
        .combatants
        .iter_mut()
        .find(|a| a.id == dying)
        .unwrap();
    actor.current_hp = 0;
    actor.add_condition(Condition::Unconscious);
    actor.death_saves.successes = 2;
    let npc = projection
        .snapshot
        .combatants
        .iter()
        .find(|a| a.character_id.is_none())
        .unwrap()
        .id;
    projection.snapshot.initiative = [first, dying, npc]
        .into_iter()
        .enumerate()
        .map(|(n, id)| InitiativeEntry {
            id,
            roll: 30 - n as i32,
            dex_tiebreak: 0,
        })
        .collect();
    projection.snapshot.current_combatant = Some(first);
    projection.revision = 1;
    store
        .commit_action(0, projection, "setup-dying", "{}")
        .await
        .unwrap();
    let before = store.party(party.campaign_id).await.unwrap().unwrap();
    let before_projection = store.get(encounter).await.unwrap().unwrap();
    let seed = (0..10000)
        .find(|s| roll_one(Die::D20, &mut SeededRng::from_seed(*s)) == 10)
        .unwrap();
    let command = ResolveCombatCommand {
        request_id: "stabilize-turn".into(),
        encounter_id: encounter,
        expected_revision: 1,
        rng_seed: seed,
        action: CombatActionCommand::EndTurn { combatant: first },
    };
    // A failed final receipt must undo the mandatory roll, deadline and clock together.
    sqlx::query("CREATE TRIGGER fail_death_receipt BEFORE INSERT ON combat_action_receipts BEGIN SELECT RAISE(ABORT,'test'); END").execute(store.pool()).await.unwrap();
    assert!(ResolveCombatAction::new(store.clone())
        .execute(command.clone())
        .await
        .is_err());
    assert_eq!(
        store.party(party.campaign_id).await.unwrap().unwrap(),
        before
    );
    assert_eq!(
        serde_json::to_value(store.get(encounter).await.unwrap().unwrap()).unwrap(),
        serde_json::to_value(before_projection).unwrap()
    );
    sqlx::query("DROP TRIGGER fail_death_receipt")
        .execute(store.pool())
        .await
        .unwrap();
    let result = ResolveCombatAction::new(store.clone())
        .execute(command.clone())
        .await
        .unwrap();
    let saves: Vec<_> = result
        .projection
        .events
        .iter()
        .flat_map(|e| &e.death_saves)
        .collect();
    assert_eq!(saves.len(), 1);
    assert!(saves[0].stable);
    let saved = store.party(party.campaign_id).await.unwrap().unwrap();
    let vitality = app_application::vitality::read(&saved.characters[&id], id)
        .unwrap()
        .unwrap();
    let deadline = vitality.natural_recovery.unwrap();
    assert_eq!(deadline.started_at_seconds, 6);
    assert!([3600, 7200, 10800, 14400]
        .contains(&(deadline.ready_at_seconds - deadline.started_at_seconds)));
    let mut replay = command;
    replay.rng_seed = seed + 1;
    let duplicate = ResolveCombatAction::new(Arc::new(SqliteStore::new(store.pool().clone())))
        .execute(replay)
        .await
        .unwrap();
    assert!(duplicate.duplicate);
    assert_eq!(
        serde_json::to_value(&duplicate.projection).unwrap(),
        serde_json::to_value(&result.projection).unwrap()
    );
    assert_eq!(
        store.party(party.campaign_id).await.unwrap().unwrap(),
        saved
    );
    // Additional combat rounds and combat ending preserve the original timer.
    let mut later = result.projection;
    later.revision += 1;
    later.snapshot.round += 5;
    later.snapshot.active = false;
    later.snapshot.current_combatant = None;
    store
        .commit_action(2, later, "finish-after-stable", "{}")
        .await
        .unwrap();
    let ended = store.party(party.campaign_id).await.unwrap().unwrap();
    let final_state = app_application::vitality::read(&ended.characters[&id], id)
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_value(final_state.natural_recovery.unwrap()).unwrap(),
        serde_json::to_value(deadline).unwrap()
    );
    assert_eq!(
        store
            .load_current(party.campaign_id)
            .await
            .unwrap()
            .projection
            .adventuring
            .unwrap()
            .elapsed_seconds,
        36
    );
}

#[tokio::test]
async fn damage_at_zero_clears_the_saved_recovery_timer_and_death_commits_atomically() {
    let (store, party, encounter) = fixture().await;
    let mut projection = store.get(encounter).await.unwrap().unwrap();
    let index = projection
        .snapshot
        .combatants
        .iter()
        .position(|a| a.character_id.is_some())
        .unwrap();
    let id = projection.snapshot.combatants[index]
        .character_id
        .unwrap()
        .0;
    let actor = &mut projection.snapshot.combatants[index];
    actor.current_hp = 0;
    actor.death_saves.stabilize();
    projection.revision = 1;
    let mut projection = store
        .commit_action(0, projection, "stable-fixture", "{}")
        .await
        .unwrap()
        .projection;
    let stable = store.party(party.campaign_id).await.unwrap().unwrap();
    assert!(app_application::vitality::read(&stable.characters[&id], id)
        .unwrap()
        .unwrap()
        .natural_recovery
        .is_some());
    projection.snapshot.combatants[index].apply_raw_damage_with_critical(1, true);
    projection.revision = 2;
    let mut projection = store
        .commit_action(1, projection, "critical-on-stable", "{}")
        .await
        .unwrap()
        .projection;
    let dying = store.party(party.campaign_id).await.unwrap().unwrap();
    let vitality = app_application::vitality::read(&dying.characters[&id], id)
        .unwrap()
        .unwrap();
    assert!(!vitality.death_saves.stable);
    assert_eq!(vitality.death_saves.failures, 2);
    assert!(vitality.natural_recovery.is_none());
    projection.snapshot.combatants[index].apply_raw_damage(1);
    projection.revision = 3;
    sqlx::query("CREATE TRIGGER fail_death_write BEFORE INSERT ON combat_action_receipts BEGIN SELECT RAISE(ABORT,'test'); END").execute(store.pool()).await.unwrap();
    assert!(store
        .commit_action(2, projection.clone(), "third-failure", "{}")
        .await
        .is_err());
    assert_eq!(
        store.party(party.campaign_id).await.unwrap().unwrap(),
        dying
    );
    assert_eq!(store.get(encounter).await.unwrap().unwrap().revision, 2);
    sqlx::query("DROP TRIGGER fail_death_write")
        .execute(store.pool())
        .await
        .unwrap();
    store
        .commit_action(2, projection, "third-failure", "{}")
        .await
        .unwrap();
    let dead = store.party(party.campaign_id).await.unwrap().unwrap();
    let vitality = app_application::vitality::read(&dead.characters[&id], id)
        .unwrap()
        .unwrap();
    assert!(vitality.is_dead);
    assert_eq!(vitality.death_saves.failures, 3);
    assert!(vitality.natural_recovery.is_none());
    assert_eq!(dead.characters[&id]["hp"], 0);
}

#[tokio::test]
async fn legacy_guest_at_the_metadata_limit_remains_readable_after_armor_rules_update() {
    use app_application::models::campaign_state::{
        within_character_budget, MAX_CHARACTER_METADATA_BYTES,
    };
    let (store, party, _) = fixture().await;
    let guest = party.roster.members[1].character_id;
    let mut sheet = party.characters[&guest].clone();
    sheet.as_object_mut().unwrap().remove("defense");
    sheet["inventory"] = json!([{"id":"chain-shirt","name":"Chain Shirt","count":1}]);
    sheet["backstory"] = json!("");
    let used = serde_json::to_vec(&sheet).unwrap().len();
    sheet["backstory"] = json!("x".repeat(MAX_CHARACTER_METADATA_BYTES - used));
    assert!(within_character_budget(&sheet));
    sqlx::query(
        "UPDATE party_characters SET character_json=?1 WHERE campaign_id=?2 AND character_id=?3",
    )
    .bind(sheet.to_string())
    .bind(party.campaign_id.0.to_string())
    .bind(guest.to_string())
    .execute(store.pool())
    .await
    .unwrap();
    let loaded = store.party(party.campaign_id).await.unwrap().unwrap();
    assert_eq!(loaded.characters[&guest], sheet);
}
