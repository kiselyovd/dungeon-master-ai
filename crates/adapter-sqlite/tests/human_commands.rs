use adapter_sqlite::{human_commands::SoloHumanCommandLedger, pool, SqliteStore};
use app_application::agent::hero_intentions::{
    HeroCombatIntent, HeroIntention, HeroIntentionRepository,
};
use app_application::models::combat::{
    CombatProjection, CombatSnapshot, COMBAT_PROJECTION_VERSION,
};
use app_application::ports::repositories::CombatRepository;
use app_application::{
    agent::{
        commands::AdventureActionCommand,
        human_commands::*,
        operations::{AgentOperation, AgentOperationRepository},
    },
    campaigns::{CreateCampaign, CreateCampaignCommand},
    models::{
        adventure::{Action, AdventureCommand, Location},
        agent::AgentTurnRequest,
        campaign_state::CommitCampaignCharacter,
        identity::CampaignId,
        session::CampaignSeed,
    },
    ports::repositories::{AdventureRepository, CampaignStateRepository},
};
use app_domain::combat::{combatant::Combatant, initiative::InitiativeEntry, types::CombatantId};
use serde_json::json;
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;
struct Fixture {
    store: SqliteStore,
    owner: Uuid,
    operation: AgentOperation,
    manifest: HumanCommandManifest,
}
async fn fixture(pool: SqlitePool) -> Fixture {
    pool::migrate(&pool).await.unwrap();
    let store = SqliteStore::new(pool);
    let campaign = CreateCampaign::new(Arc::new(store.clone()))
        .execute(CreateCampaignCommand {
            title: "Commands".into(),
            premise: "Inspect quay".into(),
            seed: CampaignSeed::Bundled {
                adventure_id: "bell-beneath-water".into(),
            },
        })
        .await
        .unwrap();
    let actor = Uuid::new_v4();
    store.commit_character(CommitCampaignCharacter {campaign_id:campaign.id,session_id:campaign.last_session_id,character_id:actor,expected_revision:campaign.revision,causation_id:"hero".into(),character:json!({"name":"Nora","heroClass":"fighter","level":1,"hp":12,"hpMax":12,"ac":14,"abilities":{"str":15,"dex":14,"con":13,"int":12,"wis":10,"cha":8},"inventory":[]})}).await.unwrap();
    let owner = Uuid::new_v4();
    let message = "Иду на причал и осматриваю его";
    let operation = AgentOperation {
        request_id: Uuid::new_v4(),
        request: AgentTurnRequest {
            campaign_id: campaign.id.0,
            session_id: campaign.last_session_id.0,
            player_message: message.into(),
            history: vec![],
            images: vec![],
            board: None,
        },
        model: None,
    };
    store.claim(operation.clone(), owner).await.unwrap();
    let manifest = HumanCommandManifest {
        version: HUMAN_COMMAND_VERSION,
        scope: CommandScope {
            request_id: operation.request_id,
            campaign_id: campaign.id.0,
            session_id: campaign.last_session_id.0,
            actor_id: actor,
            authority: CommandAuthority::Solo {
                restore_generation: 0,
            },
        },
        commands: vec![
            HumanCommand {
                id: 1,
                source: CommandSpan::from_quote(message, "Иду на причал", 0).unwrap(),
                goal: HumanCommandGoal::Exact {
                    action: HumanAction::Adventure(Action::Move(Location::Quay)),
                },
                condition: None,
            },
            HumanCommand {
                id: 2,
                source: CommandSpan::from_quote(message, "осматриваю его", 0).unwrap(),
                goal: HumanCommandGoal::Exact {
                    action: HumanAction::Adventure(Action::InspectQuay),
                },
                condition: None,
            },
        ],
        constraints: vec![],
    };
    Fixture {
        store,
        owner,
        operation,
        manifest,
    }
}
impl Fixture {
    fn journal(&self) -> SoloHumanCommandLedger {
        SoloHumanCommandLedger::new(self.store.clone(), self.operation.request_id, self.owner)
    }
    fn proposal(&self, id: u16, call: &str) -> HumanCommandStepProposal {
        let HumanCommandGoal::Exact { action } =
            self.manifest.commands[(id - 1) as usize].goal.clone()
        else {
            panic!("exact")
        };
        HumanCommandStepProposal {
            command_id: id,
            tool_call_id: call.into(),
            signature: action,
            terminal: true,
        }
    }
    async fn effect(&self, binding: &PreparedCommandBinding) {
        let HumanAction::Adventure(action) = binding.signature else {
            panic!("adventure")
        };
        let state = self
            .store
            .load_current(CampaignId(self.operation.request.campaign_id))
            .await
            .unwrap();
        self.store
            .act(AdventureCommand {
                campaign_id: state.campaign.id,
                session_id: state.session_id,
                character_id: self.manifest.scope.actor_id,
                expected_revision: state.campaign.revision,
                causation_id: format!(
                    "agent:{}:{}",
                    self.operation.request_id, binding.tool_call_id
                ),
                action,
            })
            .await
            .unwrap();
    }
}
#[tokio::test]
async fn manifest_is_immutable_and_scope_comes_from_saved_operation() {
    let f = fixture(SqlitePool::connect("sqlite::memory:").await.unwrap()).await;
    let journal = f.journal();
    assert!(journal.load_current().await.unwrap().is_none());
    journal
        .save_verified_manifest(f.manifest.clone())
        .await
        .unwrap();
    journal
        .save_verified_manifest(f.manifest.clone())
        .await
        .unwrap();
    let mut changed = f.manifest.clone();
    changed.commands[0].goal = HumanCommandGoal::Exact {
        action: HumanAction::Adventure(Action::Move(Location::Shrine)),
    };
    assert!(journal.save_verified_manifest(changed).await.is_err());
    let mut changed = f.manifest.clone();
    changed.scope.actor_id = Uuid::new_v4();
    assert!(journal.save_verified_manifest(changed).await.is_err());
    assert!(
        SoloHumanCommandLedger::new(f.store.clone(), f.operation.request_id, Uuid::new_v4())
            .load_current()
            .await
            .is_err()
    );
}
#[tokio::test]
async fn preparation_and_binding_are_atomic_and_keep_legacy_signature() {
    let f = fixture(SqlitePool::connect("sqlite::memory:").await.unwrap()).await;
    let journal = f.journal();
    journal
        .save_verified_manifest(f.manifest.clone())
        .await
        .unwrap();
    sqlx::query("CREATE TRIGGER fail_binding BEFORE INSERT ON solo_human_command_bindings BEGIN SELECT RAISE(ABORT,'fixture'); END").execute(f.store.pool()).await.unwrap();
    assert!(journal
        .prepare_after_permission(f.proposal(1, "travel"))
        .await
        .is_err());
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_tool_steps")
        .fetch_one(f.store.pool())
        .await
        .unwrap();
    assert_eq!(count, 0);
    sqlx::query("DROP TRIGGER fail_binding")
        .execute(f.store.pool())
        .await
        .unwrap();
    let prepared = journal
        .prepare_after_permission(f.proposal(1, "travel"))
        .await
        .unwrap();
    assert_eq!(
        journal
            .prepare_after_permission(f.proposal(1, "travel"))
            .await
            .unwrap(),
        prepared
    );
    let signature: String =
        sqlx::query_scalar("SELECT command_json FROM agent_tool_steps WHERE tool_call_id='travel'")
            .fetch_one(f.store.pool())
            .await
            .unwrap();
    assert_eq!(signature,json!({"tool_name":"adventure_action","args":AdventureActionCommand {action:Action::Move(Location::Quay)}}).to_string());
    let ledger = journal.load_current().await.unwrap().unwrap();
    assert!(ledger.receipts.is_empty());
    assert_eq!(
        ledger
            .manifest
            .progress(&ledger.bindings, &ledger.receipts)
            .unwrap()[0]
            .1,
        HumanCommandProgress::Prepared
    );
    assert!(journal
        .prepare_after_permission(f.proposal(2, "inspect"))
        .await
        .is_err());
}
#[tokio::test]
async fn effect_before_tool_result_recovers_and_compound_stays_finished_without_transcript() {
    let f = fixture(SqlitePool::connect("sqlite::memory:").await.unwrap()).await;
    let journal = f.journal();
    journal
        .save_verified_manifest(f.manifest.clone())
        .await
        .unwrap();
    let travel = journal
        .prepare_after_permission(f.proposal(1, "travel"))
        .await
        .unwrap();
    f.effect(&travel).await;
    let ledger = journal.load_current().await.unwrap().unwrap();
    assert_eq!(ledger.receipts.len(), 1);
    let result: Option<String> =
        sqlx::query_scalar("SELECT result_json FROM agent_tool_steps WHERE tool_call_id='travel'")
            .fetch_one(f.store.pool())
            .await
            .unwrap();
    assert!(result.is_some());
    let inspect = journal
        .prepare_after_permission(f.proposal(2, "inspect"))
        .await
        .unwrap();
    f.effect(&inspect).await;
    let before = f
        .store
        .load_current(CampaignId(f.operation.request.campaign_id))
        .await
        .unwrap();
    let ledger = journal.load_current().await.unwrap().unwrap();
    assert!(ledger
        .manifest
        .progress(&ledger.bindings, &ledger.receipts)
        .unwrap()
        .iter()
        .all(|(_, state)| state.closed()));
    journal.load_current().await.unwrap();
    let after = f
        .store
        .load_current(CampaignId(f.operation.request.campaign_id))
        .await
        .unwrap();
    assert_eq!(before.campaign.revision, after.campaign.revision);
    assert_eq!(before.projection.adventure, after.projection.adventure);
    let messages: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM agent_turn_messages WHERE request_id=?1")
            .bind(f.operation.request_id.to_string())
            .fetch_one(f.store.pool())
            .await
            .unwrap();
    assert_eq!(messages, 0);
    assert!(journal
        .prepare_after_permission(f.proposal(2, "new-inspect"))
        .await
        .is_err());
}
#[tokio::test]
async fn legacy_steps_cannot_gain_retroactive_authorization() {
    let f = fixture(SqlitePool::connect("sqlite::memory:").await.unwrap()).await;
    f.store.claim_tool_step(f.operation.request_id,"legacy",json!({"tool_name":"adventure_action","args":{"action":{"kind":"move","location":"quay"}}})).await.unwrap();
    assert!(f
        .journal()
        .save_verified_manifest(f.manifest.clone())
        .await
        .is_err());
    assert!(f.journal().load_current().await.unwrap().is_none());
}
#[tokio::test]
async fn timeline_change_and_changed_tool_signature_reject_old_metadata() {
    let f = fixture(SqlitePool::connect("sqlite::memory:").await.unwrap()).await;
    let journal = f.journal();
    journal
        .save_verified_manifest(f.manifest.clone())
        .await
        .unwrap();
    let prepared = journal
        .prepare_after_permission(f.proposal(1, "travel"))
        .await
        .unwrap();
    f.effect(&prepared).await;
    sqlx::query("UPDATE agent_tool_steps SET command_json=?1 WHERE tool_call_id='travel'").bind(json!({"tool_name":"adventure_action","args":{"action":{"kind":"move","location":"archive"}}}).to_string()).execute(f.store.pool()).await.unwrap();
    assert!(journal.load_current().await.is_err());
    sqlx::query("UPDATE sessions SET restore_generation=restore_generation+1 WHERE id=?1")
        .bind(f.operation.request.session_id.to_string())
        .execute(f.store.pool())
        .await
        .unwrap();
    assert!(journal.load_current().await.is_err());
    assert!(journal
        .prepare_after_permission(f.proposal(2, "inspect"))
        .await
        .is_err());
}
#[tokio::test]
async fn journal_survives_real_pool_close_reopen_and_new_process_owner() {
    let path = std::env::temp_dir().join(format!("dmai-human-commands-{}.db", Uuid::new_v4()));
    let url = format!(
        "sqlite://{}?mode=rwc",
        path.to_string_lossy().replace('\\', "/")
    );
    let mut f = fixture(SqlitePool::connect(&url).await.unwrap()).await;
    let journal = f.journal();
    journal
        .save_verified_manifest(f.manifest.clone())
        .await
        .unwrap();
    let prepared = journal
        .prepare_after_permission(f.proposal(1, "travel"))
        .await
        .unwrap();
    f.effect(&prepared).await;
    f.store
        .finish(f.operation.request_id, f.owner, false)
        .await
        .unwrap();
    f.store.pool().close().await;
    let reopened = SqlitePool::connect(&url).await.unwrap();
    f.store = SqliteStore::new(reopened);
    f.owner = Uuid::new_v4();
    f.store.resume(f.operation.clone(), f.owner).await.unwrap();
    let ledger = f.journal().load_current().await.unwrap().unwrap();
    assert_eq!(ledger.receipts.len(), 1);
    assert_eq!(
        ledger
            .manifest
            .progress(&ledger.bindings, &ledger.receipts)
            .unwrap()[1]
            .1,
        HumanCommandProgress::Ready
    );
    f.store.pool().close().await;
    std::fs::remove_file(path).unwrap();
}

async fn start_combat(f: &Fixture) -> CombatProjection {
    let npc = CombatantId::new();
    let projection = CombatProjection {
        authored: None,
        adventure: None,
        campaign_scope: None,
        schema_version: COMBAT_PROJECTION_VERSION,
        encounter_id: Uuid::new_v4(),
        revision: 0,
        snapshot: CombatSnapshot {
            active: true,
            round: 1,
            current_combatant: Some(npc),
            initiative: vec![InitiativeEntry {
                id: npc,
                roll: 1,
                dex_tiebreak: 0,
            }],
            combatants: vec![Combatant::new(npc, "Echo".into(), 8, 8, 12)],
        },
        events: vec![],
    };
    let mut result = f
        .store
        .create(f.operation.request.session_id, projection)
        .await
        .unwrap();
    let hero = result
        .snapshot
        .combatants
        .iter()
        .find(|actor| actor.character_id.is_some())
        .unwrap()
        .id;
    result.snapshot.current_combatant = Some(hero);
    result
        .snapshot
        .initiative
        .sort_by_key(|entry| entry.id != hero);
    result.revision += 1;
    f.store
        .compare_and_set(result.revision - 1, result.clone())
        .await
        .unwrap();
    f.store.get(result.encounter_id).await.unwrap().unwrap()
}
async fn combat_fixture() -> (Fixture, CombatProjection, PreparedCommandBinding) {
    let mut f = fixture(SqlitePool::connect("sqlite::memory:").await.unwrap()).await;
    f.store
        .finish(f.operation.request_id, f.owner, false)
        .await
        .unwrap();
    f.operation.request_id = Uuid::new_v4();
    f.operation.request.player_message = "Завершаю ход".into();
    f.store.claim(f.operation.clone(), f.owner).await.unwrap();
    f.manifest.scope.request_id = f.operation.request_id;
    f.manifest.commands.truncate(1);
    f.manifest.commands[0].source =
        CommandSpan::from_quote(&f.operation.request.player_message, "Завершаю ход", 0).unwrap();
    f.manifest.commands[0].goal = HumanCommandGoal::Exact {
        action: HumanAction::Combat(HeroCombatIntent::EndTurn),
    };
    let combat = start_combat(&f).await;
    f.journal()
        .save_verified_manifest(f.manifest.clone())
        .await
        .unwrap();
    let binding = f
        .journal()
        .prepare_after_permission(f.proposal(1, "end-turn"))
        .await
        .unwrap();
    (f, combat, binding)
}
async fn bound_end_turn(
    f: &Fixture,
) -> Result<
    app_application::models::campaign_state::CampaignSessionState,
    app_application::ports::repositories::RepositoryError,
> {
    f.store
        .combat_as_dm(
            HeroIntention {
                campaign_id: CampaignId(f.operation.request.campaign_id),
                session_id: app_application::models::identity::SessionId(
                    f.operation.request.session_id,
                ),
                causation_id: format!("agent:{}:end-turn", f.operation.request_id),
                action: HeroCombatIntent::EndTurn,
            },
            7,
        )
        .await
}
#[tokio::test]
async fn pending_end_turn_cannot_move_to_a_new_encounter_or_later_revision() {
    for new_encounter in [false, true] {
        let (f, combat, _) = combat_fixture().await;
        if new_encounter {
            f.store.end(combat.encounter_id).await.unwrap();
            start_combat(&f).await;
        } else {
            let mut next = combat.clone();
            next.revision += 1;
            next.snapshot.round += 1;
            f.store
                .compare_and_set(combat.revision, next)
                .await
                .unwrap();
        }
        let before = f
            .store
            .load_current(CampaignId(f.operation.request.campaign_id))
            .await
            .unwrap();
        assert!(f.journal().load_current().await.is_err());
        assert!(bound_end_turn(&f).await.is_err());
        let after = f
            .store
            .load_current(CampaignId(f.operation.request.campaign_id))
            .await
            .unwrap();
        assert_eq!(before.projection.combat, after.projection.combat);
        assert_eq!(before.campaign.revision, after.campaign.revision);
    }
}
#[tokio::test]
async fn committed_combat_receipt_is_recovered_before_stale_pending_checks() {
    let (f, combat, _) = combat_fixture().await;
    bound_end_turn(&f).await.unwrap();
    f.store.end(combat.encounter_id).await.unwrap();
    start_combat(&f).await;
    let before = f
        .store
        .load_current(CampaignId(f.operation.request.campaign_id))
        .await
        .unwrap();
    let ledger = f.journal().load_current().await.unwrap().unwrap();
    assert_eq!(ledger.receipts.len(), 1);
    assert_eq!(
        ledger
            .manifest
            .progress(&ledger.bindings, &ledger.receipts)
            .unwrap()[0]
            .1,
        HumanCommandProgress::Finished
    );
    let after = f
        .store
        .load_current(CampaignId(f.operation.request.campaign_id))
        .await
        .unwrap();
    assert_eq!(before.projection.combat, after.projection.combat);
    assert_eq!(before.campaign.revision, after.campaign.revision);
}
#[tokio::test]
async fn commit_transaction_rejects_context_or_signature_changed_after_preparation() {
    let (f, combat, _) = combat_fixture().await;
    let before = f
        .store
        .load_current(CampaignId(f.operation.request.campaign_id))
        .await
        .unwrap();
    let hero = combat
        .snapshot
        .combatants
        .iter()
        .find(|actor| actor.character_id.is_some())
        .unwrap()
        .id;
    let key = format!("agent:{}:end-turn", f.operation.request_id);
    let wrong=json!({"encounter_id":combat.encounter_id,"expected_revision":combat.revision,"action":HeroCombatIntent::Move {x:2,y:3}.for_actor(hero)}).to_string();
    let mut next = combat.clone();
    next.revision += 1;
    assert!(f
        .store
        .commit_action(combat.revision, next, &key, &wrong)
        .await
        .is_err());
    let signature=json!({"encounter_id":combat.encounter_id,"expected_revision":combat.revision,"action":HeroCombatIntent::EndTurn.for_actor(hero)}).to_string();
    for field in ["expected_revision", "encounter_id"] {
        let mut altered: serde_json::Value = serde_json::from_str(&signature).unwrap();
        altered[field] = if field == "expected_revision" {
            json!(combat.revision + 99)
        } else {
            json!(Uuid::new_v4())
        };
        let mut next = combat.clone();
        next.revision += 1;
        assert!(f
            .store
            .commit_action(combat.revision, next, &key, &altered.to_string())
            .await
            .is_err());
    }
    let mut unbound = combat.clone();
    unbound.revision += 1;
    unbound.campaign_scope = None;
    assert!(f
        .store
        .commit_action(combat.revision, unbound, &key, &signature)
        .await
        .is_err());
    let after = f
        .store
        .load_current(CampaignId(f.operation.request.campaign_id))
        .await
        .unwrap();
    assert_eq!(before.projection.combat, after.projection.combat);
    assert_eq!(before.campaign.revision, after.campaign.revision);
}

#[tokio::test]
async fn definitive_refusal_is_durable_and_cannot_reexecute_the_old_bound_command() {
    let f = fixture(SqlitePool::connect("sqlite::memory:").await.unwrap()).await;
    f.journal()
        .save_verified_manifest(f.manifest.clone())
        .await
        .unwrap();
    f.journal()
        .prepare_after_permission(f.proposal(1, "travel"))
        .await
        .unwrap();
    let refused = app_application::agent::tool_dispatch::ToolExecution {
        result: json!({"error":"adventure_action_unavailable","resolution":"rejected"}),
        is_error: true,
        handled_by: "engine".into(),
        media: vec![],
    };
    f.store
        .finish_tool_step(f.operation.request_id, "travel", &refused)
        .await
        .unwrap();
    let ledger = f.journal().load_current().await.unwrap().unwrap();
    assert_eq!(
        ledger
            .manifest
            .progress(&ledger.bindings, &ledger.receipts)
            .unwrap()[0]
            .1,
        HumanCommandProgress::Finished
    );
    let before = f
        .store
        .load_current(CampaignId(f.operation.request.campaign_id))
        .await
        .unwrap();
    assert!(f
        .store
        .act(AdventureCommand {
            campaign_id: before.campaign.id,
            session_id: before.session_id,
            character_id: f.manifest.scope.actor_id,
            expected_revision: before.campaign.revision,
            causation_id: format!("agent:{}:travel", f.operation.request_id),
            action: Action::Move(Location::Quay)
        })
        .await
        .is_err());
    let after = f
        .store
        .load_current(CampaignId(f.operation.request.campaign_id))
        .await
        .unwrap();
    assert_eq!(before.campaign.revision, after.campaign.revision);
}
#[tokio::test]
async fn uncertain_failures_do_not_poison_a_bound_prepared_step() {
    let f = fixture(SqlitePool::connect("sqlite::memory:").await.unwrap()).await;
    f.journal()
        .save_verified_manifest(f.manifest.clone())
        .await
        .unwrap();
    let binding = f
        .journal()
        .prepare_after_permission(f.proposal(1, "travel"))
        .await
        .unwrap();
    let uncertain = app_application::agent::tool_dispatch::ToolExecution {
        result: json!({"error":"tool_checkpoint_failed"}),
        is_error: true,
        handled_by: "engine".into(),
        media: vec![],
    };
    assert!(f
        .store
        .finish_tool_step(f.operation.request_id, "travel", &uncertain)
        .await
        .is_err());
    let ledger = f.journal().load_current().await.unwrap().unwrap();
    assert!(ledger.receipts.is_empty());
    assert_eq!(
        ledger
            .manifest
            .progress(&ledger.bindings, &ledger.receipts)
            .unwrap()[0]
            .1,
        HumanCommandProgress::Prepared
    );
    f.effect(&binding).await;
    assert_eq!(
        f.journal()
            .load_current()
            .await
            .unwrap()
            .unwrap()
            .receipts
            .len(),
        1
    );
}
#[tokio::test]
async fn dice_result_settlement_checks_the_prepared_world_context() {
    for changed in [false, true] {
        let f = fixture(SqlitePool::connect("sqlite::memory:").await.unwrap()).await;
        let mut manifest = f.manifest.clone();
        manifest.commands.truncate(1);
        manifest.commands[0].goal = HumanCommandGoal::Exact {
            action: HumanAction::Dice(app_application::agent::commands::RollDiceCommand {
                dice: "1d6".into(),
                modifier: None,
                advantage: None,
                disadvantage: None,
                reason: None,
                check: None,
            }),
        };
        f.journal()
            .save_verified_manifest(manifest.clone())
            .await
            .unwrap();
        let HumanCommandGoal::Exact { action } = manifest.commands[0].goal.clone() else {
            unreachable!()
        };
        f.journal()
            .prepare_after_permission(HumanCommandStepProposal {
                command_id: 1,
                tool_call_id: "dice".into(),
                signature: action,
                terminal: true,
            })
            .await
            .unwrap();
        if changed {
            let state = f
                .store
                .load_current(CampaignId(f.operation.request.campaign_id))
                .await
                .unwrap();
            f.store
                .act(AdventureCommand {
                    campaign_id: state.campaign.id,
                    session_id: state.session_id,
                    character_id: f.manifest.scope.actor_id,
                    expected_revision: state.campaign.revision,
                    causation_id: "other-action".into(),
                    action: Action::Move(Location::Quay),
                })
                .await
                .unwrap();
        }
        let result = app_application::agent::tool_dispatch::ToolExecution {
            result: json!({"rolls":[5],"total":5}),
            is_error: false,
            handled_by: "engine".into(),
            media: vec![],
        };
        assert_eq!(
            f.store
                .finish_tool_step(f.operation.request_id, "dice", &result)
                .await
                .is_ok(),
            !changed
        );
    }
}
