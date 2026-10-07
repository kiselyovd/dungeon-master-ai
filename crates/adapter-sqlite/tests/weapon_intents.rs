use adapter_sqlite::{pool, SqliteStore};
use app_application::{
    campaigns::{CreateCampaign, CreateCampaignCommand, CreateSnapshot, CreateSnapshotCommand},
    combat::{
        commands::{CombatActionCommand, ResolveCombatCommand},
        resolve::ResolveCombatAction,
    },
    models::{
        campaign_state::CommitCampaignCharacter,
        combat::{CombatProjection, CombatSnapshot, COMBAT_PROJECTION_VERSION},
        session::CampaignSeed,
    },
    ports::repositories::{CampaignStateRepository, CombatRepository},
};
use app_domain::combat::{
    combatant::Combatant, initiative::InitiativeEntry, loadout::WeaponMode, types::CombatantId,
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

async fn fixture() -> (
    Arc<SqliteStore>,
    CommitCampaignCharacter,
    Uuid,
    CombatantId,
    CombatantId,
) {
    let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    fixture_with_pool(db).await
}

async fn fixture_with_pool(
    db: sqlx::SqlitePool,
) -> (
    Arc<SqliteStore>,
    CommitCampaignCharacter,
    Uuid,
    CombatantId,
    CombatantId,
) {
    pool::migrate(&db).await.unwrap();
    let store = Arc::new(SqliteStore::new(db));
    let campaign = CreateCampaign::new(store.clone())
        .execute(CreateCampaignCommand {
            title: "Weapons".into(),
            premise: "Bridge".into(),
            seed: CampaignSeed::Collaborative {
                answers: std::collections::BTreeMap::from([(
                    "world".into(),
                    vec!["Bridge".into()],
                )]),
            },
        })
        .await
        .unwrap();
    let character = CommitCampaignCharacter {
        campaign_id: campaign.id,
        session_id: campaign.last_session_id,
        character_id: Uuid::new_v4(),
        expected_revision: campaign.revision,
        causation_id: "hero".into(),
        character: json!({"name":"Mira","heroClass":"fighter","level":1,"hp":20,"hpMax":20,"ac":15,"abilities":{"str":16,"dex":16,"con":14,"int":10,"wis":12,"cha":8},"inventory":[{"id":"shortbow","name":"Shortbow","count":1},{"id":"arrows-20","name":"Arrows (20)","count":1}]}),
    };
    store.commit_character(character.clone()).await.unwrap();
    let encounter = Uuid::new_v4();
    let npc = CombatantId::new();
    store
        .create(
            character.session_id.0,
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
                    combatants: vec![Combatant::new(npc, "Enemy".into(), 100, 100, 10)],
                },
                events: vec![],
            },
        )
        .await
        .unwrap();
    let hero = store
        .get(encounter)
        .await
        .unwrap()
        .unwrap()
        .snapshot
        .combatants
        .iter()
        .find(|actor| actor.character_id.is_some())
        .unwrap()
        .id;
    (store, character, encounter, hero, npc)
}

#[tokio::test]
async fn legacy_bound_combat_recovers_profiles_after_reopen_and_restore() {
    use app_application::models::snapshot::RestoreSnapshotRecord;
    use app_application::ports::repositories::SnapshotRepository;
    let path = std::env::temp_dir().join(format!("dmai-weapon-upgrade-{}.sqlite", Uuid::new_v4()));
    let url = format!("sqlite://{}?mode=rwc", path.display());
    let (store, character, encounter, hero, _) =
        fixture_with_pool(sqlx::SqlitePool::connect(&url).await.unwrap()).await;
    let mut legacy = serde_json::to_value(store.get(encounter).await.unwrap().unwrap()).unwrap();
    for actor in legacy["snapshot"]["combatants"].as_array_mut().unwrap() {
        actor.as_object_mut().unwrap().remove("loadout");
    }
    legacy["snapshot"]["round"] = json!(3);
    legacy["snapshot"]["combatants"][0]["budget"]["movement_ft"] = json!(15);
    sqlx::query("UPDATE combat_encounters SET projection=?1 WHERE id=?2")
        .bind(legacy.to_string())
        .bind(encounter.to_string())
        .execute(store.pool())
        .await
        .unwrap();
    store.pool().close().await;
    drop(store);
    let db = sqlx::SqlitePool::connect(&url).await.unwrap();
    let store = Arc::new(SqliteStore::new(db));
    let current = store.load_current(character.campaign_id).await.unwrap();
    let loaded: CombatProjection =
        serde_json::from_value(current.projection.combat.unwrap()).unwrap();
    assert!(loaded
        .snapshot
        .combatants
        .iter()
        .all(|actor| actor.loadout.is_some()));
    assert_eq!(loaded.snapshot.round, 3);
    assert_eq!(loaded.snapshot.combatants[0].budget.movement_ft, 15);
    let saved = CreateSnapshot::new(store.clone())
        .execute(CreateSnapshotCommand {
            id: Uuid::new_v4(),
            campaign_id: character.campaign_id,
            session_id: character.session_id,
            expected_revision: current.campaign.revision,
            title: "Legacy".into(),
            kind: "manual".into(),
            summary: "".into(),
            tag: "combat".into(),
            causation_id: "legacy-save".into(),
            character: character.character,
        })
        .await
        .unwrap();
    let mut old_snapshot = saved.snapshot.projection.clone();
    old_snapshot.combat = Some(legacy);
    sqlx::query("UPDATE campaign_snapshots SET projection_json=?1 WHERE id=?2")
        .bind(serde_json::to_string(&old_snapshot).unwrap())
        .bind(saved.snapshot.id.to_string())
        .execute(store.pool())
        .await
        .unwrap();
    let mut future_sheet = saved.snapshot.projection.character.clone();
    future_sheet["combatResources"]["arrows"] = json!(5);
    sqlx::query("UPDATE campaign_characters SET character_json=?1 WHERE campaign_id=?2")
        .bind(future_sheet.to_string())
        .bind(character.campaign_id.0.to_string())
        .execute(store.pool())
        .await
        .unwrap();
    store
        .restore_snapshot(RestoreSnapshotRecord {
            save_id: saved.save_id,
            campaign_id: character.campaign_id,
            session_id: character.session_id,
            expected_revision: saved.snapshot.revision,
            causation_id: "legacy-restore".into(),
        })
        .await
        .unwrap();
    let recovered = store.get(encounter).await.unwrap().unwrap();
    assert!(recovered
        .snapshot
        .combatants
        .iter()
        .all(|actor| actor.loadout.is_some()));
    assert_eq!(recovered.snapshot.round, 3);
    assert_eq!(recovered.snapshot.combatants[0].budget.movement_ft, 15);
    assert_eq!(
        recovered.snapshot.combatants[0]
            .loadout
            .as_ref()
            .unwrap()
            .arrows,
        20
    );
    ResolveCombatAction::new(store.clone())
        .execute(ResolveCombatCommand {
            request_id: "legacy-end".into(),
            encounter_id: encounter,
            expected_revision: recovered.revision,
            rng_seed: 42,
            action: CombatActionCommand::EndTurn { combatant: hero },
        })
        .await
        .unwrap();
    store.pool().close().await;
    drop(store);
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn canonical_attack_spends_one_arrow_and_exact_replay_does_not_spend_again() {
    let (store, character, encounter, hero, npc) = fixture().await;
    let command = ResolveCombatCommand {
        request_id: "shoot".into(),
        encounter_id: encounter,
        expected_revision: 0,
        rng_seed: 42,
        action: CombatActionCommand::WeaponAttack {
            attacker: hero,
            target: npc,
            weapon_id: Some("shortbow".into()),
            mode: WeaponMode::Default,
            two_handed: false,
        },
    };
    let first = ResolveCombatAction::new(store.clone())
        .execute(command.clone())
        .await
        .unwrap();
    assert_eq!(
        first
            .projection
            .snapshot
            .combatants
            .iter()
            .find(|actor| actor.id == hero)
            .unwrap()
            .loadout
            .as_ref()
            .unwrap()
            .arrows,
        19
    );
    assert_eq!(
        store
            .load_current(character.campaign_id)
            .await
            .unwrap()
            .projection
            .character["combatResources"]["arrows"],
        19
    );
    let replay = ResolveCombatAction::new(store.clone())
        .execute(command)
        .await
        .unwrap();
    assert!(replay.duplicate);
    assert_eq!(
        store
            .load_current(character.campaign_id)
            .await
            .unwrap()
            .projection
            .character["combatResources"]["arrows"],
        19
    );
}

#[tokio::test]
async fn snapshot_outside_combat_cannot_reset_spent_ammunition() {
    let (store, character, encounter, hero, npc) = fixture().await;
    ResolveCombatAction::new(store.clone())
        .execute(ResolveCombatCommand {
            request_id: "shoot".into(),
            encounter_id: encounter,
            expected_revision: 0,
            rng_seed: 42,
            action: CombatActionCommand::WeaponAttack {
                attacker: hero,
                target: npc,
                weapon_id: None,
                mode: WeaponMode::Default,
                two_handed: false,
            },
        })
        .await
        .unwrap();
    store.end(encounter).await.unwrap();
    let current = store.load_current(character.campaign_id).await.unwrap();
    let saved = CreateSnapshot::new(store.clone())
        .execute(CreateSnapshotCommand {
            id: Uuid::new_v4(),
            campaign_id: character.campaign_id,
            session_id: character.session_id,
            expected_revision: current.campaign.revision,
            title: "Outside combat".into(),
            kind: "manual".into(),
            summary: "".into(),
            tag: "exploration".into(),
            causation_id: "outside-save".into(),
            character: character.character,
        })
        .await
        .unwrap();
    assert_eq!(
        saved.snapshot.projection.character["combatResources"]["arrows"],
        19
    );
}

#[tokio::test]
async fn ending_the_player_turn_executes_the_npc_and_returns_control() {
    let (store, _, encounter, hero, _) = fixture().await;
    let result = ResolveCombatAction::new(store)
        .execute(ResolveCombatCommand {
            request_id: "end".into(),
            encounter_id: encounter,
            expected_revision: 0,
            rng_seed: 42,
            action: CombatActionCommand::EndTurn { combatant: hero },
        })
        .await
        .unwrap();
    assert_eq!(result.projection.snapshot.current_combatant, Some(hero));
    assert!(result
        .projection
        .events
        .iter()
        .any(|event| !event.rolls.is_empty()));
    assert_eq!(result.projection.snapshot.round, 2);
}

#[tokio::test]
async fn an_encounter_finished_during_opening_is_closed_in_the_database() {
    let (store, character, encounter, _, _) = fixture().await;
    store.end(encounter).await.unwrap();
    let empty = Uuid::new_v4();
    store
        .create(character.session_id.0, CombatProjection::empty(empty, 0))
        .await
        .unwrap();
    assert!(store
        .load_current(character.campaign_id)
        .await
        .unwrap()
        .projection
        .combat
        .is_none());
    assert!(store.get(empty).await.unwrap().is_none());
}
