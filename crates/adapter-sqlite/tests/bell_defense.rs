use adapter_sqlite::{pool, SqliteStore};
use app_application::{
    campaigns::{CreateCampaign, CreateCampaignCommand},
    combat::{
        commands::{CombatActionCommand, ResolveCombatCommand},
        resolve::ResolveCombatAction,
    },
    models::{
        adventure::{Action, AdventureCommand, DefenseOutcome, Fact, Location},
        campaign_state::CommitCampaignCharacter,
        combat::CombatProjection,
        session::CampaignSeed,
    },
    ports::repositories::{AdventureRepository, CampaignStateRepository},
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn dm_retreat_delivers_terminal_combat_even_though_current_state_omits_it() {
    use app_application::agent::hero_intentions::{
        HeroCombatIntent, HeroIntention, HeroIntentionRepository,
    };
    let (store, command) = fixture(40, 30).await;
    store.act(command.clone()).await.unwrap();
    let intent = HeroIntention {
        campaign_id: command.campaign_id,
        session_id: command.session_id,
        causation_id: "agent:exit".into(),
        action: HeroCombatIntent::Move { x: 1, y: 3 },
    };
    store.combat_as_dm(intent.clone(), 1).await.unwrap();
    let original_move = intent.clone();
    let retreat = HeroIntention {
        causation_id: "agent:retreat".into(),
        action: HeroCombatIntent::Retreat,
        ..intent
    };
    let result = store.combat_as_dm(retreat.clone(), 2).await.unwrap();
    assert_eq!(
        result.projection.combat.as_ref().unwrap()["snapshot"]["active"],
        false
    );
    assert_eq!(
        result.projection.combat.as_ref().unwrap()["adventure"]["state"]["defense"],
        "retreated"
    );
    assert!(store
        .load_current(command.campaign_id)
        .await
        .unwrap()
        .projection
        .combat
        .is_none());
    assert_eq!(
        store.combat_as_dm(retreat.clone(), 999).await.unwrap(),
        result
    );
    assert_eq!(
        store
            .combat_as_dm(original_move.clone(), 999)
            .await
            .unwrap()
            .projection
            .combat
            .as_ref()
            .unwrap()["snapshot"]["active"],
        false,
        "An earlier move receipt must not resurrect the finished fight"
    );
    let mut command = command;
    for (index, action) in [Action::Move(Location::Archive), Action::StartDefense]
        .into_iter()
        .enumerate()
    {
        command.action = action;
        command.causation_id = format!("second-defense-{index}");
        command.expected_revision = store
            .load_current(command.campaign_id)
            .await
            .unwrap()
            .campaign
            .revision;
        store.act(command.clone()).await.unwrap();
    }
    let fresh = store.load_current(command.campaign_id).await.unwrap();
    assert_eq!(
        fresh.projection.combat.as_ref().unwrap()["snapshot"]["active"],
        true
    );
    assert_eq!(
        store.combat_as_dm(retreat, 999).await.unwrap(),
        fresh,
        "An old terminal receipt must not close a newer encounter"
    );
    let second_exit = HeroIntention {
        campaign_id: command.campaign_id,
        session_id: command.session_id,
        causation_id: "second-exit".into(),
        action: HeroCombatIntent::Move { x: 1, y: 3 },
    };
    store.combat_as_dm(second_exit.clone(), 1).await.unwrap();
    let second_terminal = store
        .combat_as_dm(
            HeroIntention {
                causation_id: "second-retreat".into(),
                action: HeroCombatIntent::Retreat,
                ..second_exit
            },
            2,
        )
        .await
        .unwrap();
    let old_replay = store.combat_as_dm(original_move, 999).await.unwrap();
    assert_eq!(
        old_replay, second_terminal,
        "Replaying fight A after fight B ends must deliver B's final projection"
    );
}

#[tokio::test]
async fn dm_combat_intentions_replay_the_original_receipt_and_return_fresh_state() {
    use app_application::agent::hero_intentions::{
        HeroCombatIntent, HeroIntention, HeroIntentionRepository,
    };
    let (store, command) = fixture(40, 30).await;
    store.act(command.clone()).await.unwrap();
    let intent = HeroIntention {
        campaign_id: command.campaign_id,
        session_id: command.session_id,
        causation_id: "agent:move".into(),
        action: HeroCombatIntent::Move { x: 3, y: 4 },
    };
    let moved = store.combat_as_dm(intent.clone(), 1).await.unwrap();
    let combat: CombatProjection =
        serde_json::from_value(moved.projection.combat.clone().unwrap()).unwrap();
    assert_eq!(
        combat
            .snapshot
            .combatants
            .iter()
            .find(|actor| actor.character_id.map(|id| id.0) == Some(command.character_id))
            .unwrap()
            .position
            .y,
        4
    );
    let next = store
        .combat_as_dm(
            HeroIntention {
                causation_id: "agent:turn".into(),
                action: HeroCombatIntent::EndTurn,
                ..intent.clone()
            },
            2,
        )
        .await
        .unwrap();
    let retried = store.combat_as_dm(intent.clone(), 999).await.unwrap();
    assert_eq!(retried, next);
    assert!(store
        .combat_as_dm(
            HeroIntention {
                action: HeroCombatIntent::Move { x: 4, y: 4 },
                ..intent
            },
            3
        )
        .await
        .is_err());
    assert_eq!(store.load_current(command.campaign_id).await.unwrap(), next);
}

#[tokio::test]
async fn dm_rest_derives_identity_and_revision_and_replays_without_spending_again() {
    use app_application::agent::hero_intentions::{HeroIntention, HeroIntentionRepository};
    use app_application::models::rest::RestIntent;
    let (store, mut command) = fixture(10, 20).await;
    for action in [Action::Move(Location::Quay), Action::Move(Location::Town)] {
        command.action = action;
        command.causation_id = format!("return-{action:?}");
        command.expected_revision = store
            .load_current(command.campaign_id)
            .await
            .unwrap()
            .campaign
            .revision;
        store.act(command.clone()).await.unwrap();
    }
    let intent = HeroIntention {
        campaign_id: command.campaign_id,
        session_id: command.session_id,
        causation_id: "agent:rest".into(),
        action: RestIntent::Short { hit_dice: 1 },
    };
    let rested = store.rest_as_dm(intent.clone(), 1).await.unwrap();
    assert!(rested.projection.character["hp"].as_i64().unwrap() > 10);
    assert_eq!(store.rest_as_dm(intent.clone(), 999).await.unwrap(), rested);
    assert!(store
        .rest_as_dm(
            HeroIntention {
                action: RestIntent::Long,
                ..intent
            },
            1
        )
        .await
        .is_err());
    assert_eq!(
        store.load_current(command.campaign_id).await.unwrap(),
        rested
    );
}

async fn fixture(hp: i32, ac: i32) -> (Arc<SqliteStore>, AdventureCommand) {
    let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&db).await.unwrap();
    let store = Arc::new(SqliteStore::new(db));
    let campaign = CreateCampaign::new(store.clone())
        .execute(CreateCampaignCommand {
            title: "Defense".into(),
            premise: "Archive".into(),
            seed: CampaignSeed::Bundled {
                adventure_id: "bell-beneath-water".into(),
            },
        })
        .await
        .unwrap();
    let character_id = Uuid::new_v4();
    store.commit_character(CommitCampaignCharacter{campaign_id:campaign.id,session_id:campaign.last_session_id,character_id,expected_revision:campaign.revision,causation_id:"hero".into(),character:json!({"name":"Mira","heroClass":"fighter","level":1,"hp":hp,"hpMax":40,"ac":ac,"abilities":{"str":16,"dex":14,"con":14,"int":10,"wis":12,"cha":8},"inventory":[]})}).await.unwrap();
    let mut command = AdventureCommand {
        campaign_id: campaign.id,
        session_id: campaign.last_session_id,
        character_id,
        expected_revision: campaign.revision,
        causation_id: "start".into(),
        action: Action::StartDefense,
    };
    for (index, action) in [
        Action::Move(Location::Quay),
        Action::Move(Location::Archive),
        Action::LocateTikhon,
    ]
    .into_iter()
    .enumerate()
    {
        command.action = action;
        command.causation_id = format!("step-{index}");
        command.expected_revision = store
            .load_current(campaign.id)
            .await
            .unwrap()
            .campaign
            .revision;
        store.act(command.clone()).await.unwrap();
    }
    command.action = Action::StartDefense;
    command.causation_id = "defend".into();
    command.expected_revision = store
        .load_current(campaign.id)
        .await
        .unwrap()
        .campaign
        .revision;
    (store, command)
}
async fn end(
    store: Arc<SqliteStore>,
    projection: &CombatProjection,
    index: usize,
) -> CombatProjection {
    let hero = projection
        .snapshot
        .combatants
        .iter()
        .find(|actor| actor.character_id.is_some())
        .unwrap()
        .id;
    ResolveCombatAction::new(store)
        .execute(ResolveCombatCommand {
            request_id: format!("end-{index}"),
            encounter_id: projection.encounter_id,
            expected_revision: projection.revision,
            rng_seed: 42,
            action: CombatActionCommand::EndTurn { combatant: hero },
        })
        .await
        .unwrap()
        .projection
}
#[tokio::test]
async fn starting_defense_is_atomic_retry_safe_and_holding_rounds_confirms_only_the_window() {
    let (store, command) = fixture(40, 16).await;
    let state = store.act(command.clone()).await.unwrap();
    let again = store.act(command.clone()).await.unwrap();
    assert_eq!(state, again);
    let mut projection: CombatProjection =
        serde_json::from_value(state.projection.combat.unwrap()).unwrap();
    assert_eq!(state.campaign.revision.0, command.expected_revision.0 + 1);
    assert_eq!(
        projection.campaign_scope.as_ref().unwrap().revision,
        state.campaign.revision.0
    );
    assert!(projection.authored.is_some());
    assert_eq!(
        state
            .projection
            .adventure
            .unwrap()
            .state
            .defense_encounter_id,
        Some(projection.encounter_id)
    );
    for index in 0..3 {
        projection = end(store.clone(), &projection, index).await;
    }
    assert!(!projection.snapshot.active);
    let current = store.load_current(command.campaign_id).await.unwrap();
    let adventure = current.projection.adventure.unwrap().state;
    assert_eq!(adventure.defense, Some(DefenseOutcome::Held));
    assert!(adventure.facts.contains(&Fact::DefenseHeld));
    assert!(!adventure.facts.contains(&Fact::WardRestored));
    assert!(!adventure.facts.contains(&Fact::TikhonRescued));
}
#[tokio::test]
async fn defeat_recovers_to_town_at_one_hp_without_reward_or_resource_refill() {
    let (store, command) = fixture(1, 1).await;
    let state = store.act(command.clone()).await.unwrap();
    let mut projection: CombatProjection =
        serde_json::from_value(state.projection.combat.unwrap()).unwrap();
    for index in 0..4 {
        if !projection.snapshot.active {
            break;
        }
        projection = end(store.clone(), &projection, index).await;
    }
    let current = store.load_current(command.campaign_id).await.unwrap();
    let adventure = current.projection.adventure.unwrap().state;
    assert_eq!(adventure.defense, Some(DefenseOutcome::Defeated));
    assert_eq!(adventure.location, Location::Town);
    assert_eq!(current.projection.character["hp"], 1);
    assert!(!adventure.facts.contains(&Fact::RewardGranted));
    assert!(current.projection.adventuring.unwrap().elapsed_seconds >= 4500);
}

#[tokio::test]
async fn generic_end_cannot_skip_the_exit_but_a_real_exit_retreat_updates_the_story() {
    use app_application::{models::combat::Combatant, ports::repositories::CombatRepository};
    let (store, command) = fixture(40, 16).await;
    let state = store.act(command.clone()).await.unwrap();
    let projection: CombatProjection =
        serde_json::from_value(state.projection.combat.unwrap()).unwrap();
    let hero = projection
        .snapshot
        .combatants
        .iter()
        .find(|actor: &&Combatant| actor.character_id.is_some())
        .unwrap()
        .id;
    assert!(store.end(projection.encounter_id).await.is_err());
    let moved = ResolveCombatAction::new(store.clone())
        .execute(ResolveCombatCommand {
            request_id: "move-exit".into(),
            encounter_id: projection.encounter_id,
            expected_revision: 0,
            rng_seed: 42,
            action: CombatActionCommand::Move {
                combatant: hero,
                to: app_domain::combat::types::Position { x: 1, y: 3 },
            },
        })
        .await
        .unwrap()
        .projection;
    let escaped = ResolveCombatAction::new(store.clone())
        .execute(ResolveCombatCommand {
            request_id: "retreat".into(),
            encounter_id: projection.encounter_id,
            expected_revision: moved.revision,
            rng_seed: 42,
            action: CombatActionCommand::Retreat { combatant: hero },
        })
        .await
        .unwrap()
        .projection;
    assert_eq!(
        escaped.authored.unwrap().outcome,
        Some(DefenseOutcome::Retreated)
    );
    let current = store.load_current(command.campaign_id).await.unwrap();
    assert_eq!(
        current.projection.adventure.unwrap().state.location,
        Location::Quay
    );
    assert_eq!(current.projection.character["hp"], 40);
}

#[tokio::test]
async fn failed_start_receipt_rolls_back_the_encounter_link_and_revision() {
    let (store, command) = fixture(40, 16).await;
    sqlx::query("CREATE TRIGGER reject_defense_start BEFORE INSERT ON adventure_operations WHEN NEW.causation_id='defend' BEGIN SELECT RAISE(FAIL,'fixture'); END").execute(store.pool()).await.unwrap();
    assert!(store.act(command.clone()).await.is_err());
    let current = store.load_current(command.campaign_id).await.unwrap();
    assert_eq!(current.campaign.revision, command.expected_revision);
    assert!(current
        .projection
        .adventure
        .unwrap()
        .state
        .defense
        .is_none());
    assert!(current.projection.combat.is_none());
    let encounters: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM combat_encounters")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(encounters, 0);
}

#[tokio::test]
async fn failed_story_resolution_rolls_back_combat_health_time_and_receipt() {
    let (store, command) = fixture(40, 16).await;
    let state = store.act(command.clone()).await.unwrap();
    let mut projection: CombatProjection =
        serde_json::from_value(state.projection.combat.unwrap()).unwrap();
    projection = end(store.clone(), &projection, 0).await;
    projection = end(store.clone(), &projection, 1).await;
    let before = store.load_current(command.campaign_id).await.unwrap();
    sqlx::query("CREATE TRIGGER reject_defense_result BEFORE UPDATE ON campaign_adventures WHEN json_extract(NEW.state_json,'$.defense')='held' BEGIN SELECT RAISE(FAIL,'fixture'); END").execute(store.pool()).await.unwrap();
    let hero = projection.snapshot.combatants[0].id;
    assert!(ResolveCombatAction::new(store.clone())
        .execute(ResolveCombatCommand {
            request_id: "end-2".into(),
            encounter_id: projection.encounter_id,
            expected_revision: projection.revision,
            rng_seed: 42,
            action: CombatActionCommand::EndTurn { combatant: hero }
        })
        .await
        .is_err());
    assert_eq!(
        store.load_current(command.campaign_id).await.unwrap(),
        before
    );
}

#[tokio::test]
async fn restoring_an_active_defense_preserves_map_link_round_and_new_revision() {
    use app_application::{
        campaigns::{CreateSnapshot, CreateSnapshotCommand},
        models::snapshot::RestoreSnapshotRecord,
        ports::repositories::SnapshotRepository,
    };
    let (store, command) = fixture(40, 16).await;
    let state = store.act(command.clone()).await.unwrap();
    let original: CombatProjection =
        serde_json::from_value(state.projection.combat.clone().unwrap()).unwrap();
    let saved = CreateSnapshot::new(store.clone())
        .execute(CreateSnapshotCommand {
            id: Uuid::new_v4(),
            campaign_id: command.campaign_id,
            session_id: command.session_id,
            expected_revision: state.campaign.revision,
            title: "Defense".into(),
            kind: "manual".into(),
            summary: "".into(),
            tag: "combat".into(),
            causation_id: "save-defense".into(),
            character: state.projection.character,
        })
        .await
        .unwrap();
    let advanced = end(store.clone(), &original, 0).await;
    let current = store.load_current(command.campaign_id).await.unwrap();
    store
        .restore_snapshot(RestoreSnapshotRecord {
            save_id: saved.save_id,
            campaign_id: command.campaign_id,
            session_id: command.session_id,
            expected_revision: current.campaign.revision,
            causation_id: "restore-defense".into(),
        })
        .await
        .unwrap();
    let restored = store.load_current(command.campaign_id).await.unwrap();
    let projection: CombatProjection =
        serde_json::from_value(restored.projection.combat.unwrap()).unwrap();
    assert!(projection.revision > advanced.revision);
    assert_eq!(projection.snapshot.round, 1);
    assert_eq!(
        projection.authored.as_ref().unwrap().map,
        original.authored.unwrap().map
    );
    assert_eq!(
        restored
            .projection
            .adventure
            .unwrap()
            .state
            .defense_encounter_id,
        Some(original.encounter_id)
    );
    assert_eq!(
        projection.campaign_scope.unwrap().revision,
        restored.campaign.revision.0
    );
}
