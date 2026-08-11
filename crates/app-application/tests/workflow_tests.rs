use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use app_application::agent::context::{
    AgentContext, AgentContextError, AgentContextPort, AgentContextRequest,
};
use app_application::agent::tool_dispatch::{
    GeneratedAgentMedia, NoopRuntimeCoordinator, ToolDispatchError, ToolDispatchRequest,
    ToolDispatcher, ToolExecution,
};
use app_application::agent::tools::{AgentToolCatalog, ToolAvailability};
use app_application::agent::turn::{
    AgentTurnCommand, AgentTurnConfig, AgentTurnError, AgentTurnService, CancellationSignal,
    NeverCancelled,
};
use app_application::combat::commands::{CombatActionCommand, ResolveCombatCommand};
use app_application::combat::resolve::{ResolveCombatAction, ResolveCombatError};
use app_application::models::agent::{AgentEvent, AgentTurnRequest};
use app_application::models::campaign::StoredMessage;
use app_application::models::chat::{
    Capabilities, ChatChunk, ChatMessage, ChatRequest, FinishReason, ReasoningSpec,
};
use app_application::models::combat::{
    CombatProjection, CombatSnapshot, COMBAT_PROJECTION_VERSION,
};
use app_application::ports::events::{
    ApplicationEvent, ApplicationEventSink, EventSinkError, NoopApplicationEventSink,
};
use app_application::ports::llm::{ChunkStream, LlmError, LlmProvider};
use app_application::ports::repositories::{CombatRepository, MessageRepository, RepositoryError};
use app_domain::combat::combatant::Combatant;
use app_domain::combat::initiative::InitiativeEntry;
use app_domain::combat::types::{CombatantId, DamageType};
use app_domain::dice::{DiceExpr, Die};
use async_trait::async_trait;
use futures::{stream, StreamExt};
use serde_json::json;
use uuid::Uuid;

struct FakeProvider {
    rounds: Mutex<VecDeque<Result<Vec<ChatChunk>, LlmError>>>,
    calls: AtomicUsize,
}

impl FakeProvider {
    fn scripted(rounds: Vec<Vec<ChatChunk>>) -> Self {
        Self {
            rounds: Mutex::new(rounds.into_iter().map(Ok).collect()),
            calls: AtomicUsize::new(0),
        }
    }

    fn failing() -> Self {
        Self {
            rounds: Mutex::new(VecDeque::from([Err(LlmError::Network("offline".into()))])),
            calls: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl LlmProvider for FakeProvider {
    async fn stream_chat(&self, _request: ChatRequest) -> Result<ChunkStream, LlmError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let chunks = self.rounds.lock().unwrap().pop_front().unwrap_or_else(|| {
            Ok(vec![ChatChunk::Done {
                reason: FinishReason::Stop,
            }])
        })?;
        Ok(stream::iter(chunks.into_iter().map(Ok)).boxed())
    }

    fn name(&self) -> &'static str {
        "fake"
    }

    fn capabilities_for_model(&self, _model_id: &str) -> Capabilities {
        Capabilities {
            tool_calls: true,
            streaming: true,
            ..Capabilities::default()
        }
    }

    fn active_model(&self) -> &str {
        "fake-model"
    }
}

struct FakeContext;

#[async_trait]
impl AgentContextPort for FakeContext {
    async fn build(&self, request: AgentContextRequest) -> Result<AgentContext, AgentContextError> {
        Ok(AgentContext {
            system_prompt: request.base_system_prompt,
            combat_active: false,
        })
    }
}

#[derive(Default)]
struct FakeMessages {
    values: Mutex<Vec<ChatMessage>>,
    fail: AtomicBool,
}

#[async_trait]
impl MessageRepository for FakeMessages {
    async fn append(
        &self,
        session_id: Uuid,
        message: ChatMessage,
    ) -> Result<StoredMessage, RepositoryError> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(RepositoryError::Operation {
                operation: "append",
                code: "injected",
            });
        }
        let mut values = self.values.lock().unwrap();
        values.push(message.clone());
        Ok(StoredMessage {
            id: Uuid::new_v4(),
            session_id,
            sequence: values.len() as i64,
            message,
        })
    }

    async fn list(&self, _session_id: Uuid) -> Result<Vec<StoredMessage>, RepositoryError> {
        Ok(Vec::new())
    }
}

struct FakeToolDispatcher {
    execution: ToolExecution,
    calls: AtomicUsize,
}

#[async_trait]
impl ToolDispatcher for FakeToolDispatcher {
    async fn execute(
        &self,
        _request: ToolDispatchRequest,
    ) -> Result<ToolExecution, ToolDispatchError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.execution.clone())
    }
}

#[derive(Default)]
struct RecordingEvents(Mutex<Vec<&'static str>>);

#[async_trait]
impl ApplicationEventSink for RecordingEvents {
    async fn publish(&self, event: ApplicationEvent) -> Result<(), EventSinkError> {
        let kind = match event {
            ApplicationEvent::Agent { event, .. } => event_kind(&event),
            ApplicationEvent::CombatProjectionChanged { .. } => "combat_projection_changed",
        };
        self.0.lock().unwrap().push(kind);
        Ok(())
    }
}

struct FixedCancellation(bool);

impl CancellationSignal for FixedCancellation {
    fn is_cancelled(&self) -> bool {
        self.0
    }
}

fn config() -> AgentTurnConfig {
    AgentTurnConfig {
        model: "fake-model".into(),
        system_prompt: "system".into(),
        temperature: 0.0,
        max_rounds: 3,
        embedding_model: "fake-embedding".into(),
        reasoning_enabled: false,
        reasoning_budget: ReasoningSpec::Medium,
        tool_availability: ToolAvailability::all(),
        tools: AgentToolCatalog::default(),
    }
}

fn command(request_id: &str) -> AgentTurnCommand {
    AgentTurnCommand {
        request_id: request_id.into(),
        request: AgentTurnRequest {
            campaign_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            player_message: "act".into(),
            history: Vec::new(),
            images: Vec::new(),
            board: None,
        },
    }
}

fn service(
    provider: Arc<FakeProvider>,
    messages: Arc<FakeMessages>,
    dispatcher: Arc<FakeToolDispatcher>,
    events: Arc<dyn ApplicationEventSink>,
    cancellation: Arc<dyn CancellationSignal>,
) -> Arc<AgentTurnService> {
    Arc::new(AgentTurnService::new(
        provider,
        Arc::new(FakeContext),
        messages,
        dispatcher,
        Arc::new(NoopRuntimeCoordinator),
        events,
        cancellation,
        config(),
    ))
}

fn event_kind(event: &AgentEvent) -> &'static str {
    match event {
        AgentEvent::TextDelta { .. } => "text_delta",
        AgentEvent::ToolCallStart { .. } => "tool_call_start",
        AgentEvent::ToolCallResult { .. } => "tool_call_result",
        AgentEvent::ReasoningText { .. } => "reasoning_text",
        AgentEvent::ImageGenerated { .. } => "image_generated",
        AgentEvent::VideoGenerated { .. } => "video_generated",
        AgentEvent::AgentDone { .. } => "agent_done",
    }
}

#[tokio::test]
async fn agent_turn_orders_media_before_tool_result_and_persists_each_role() {
    let provider = Arc::new(FakeProvider::scripted(vec![
        vec![
            ChatChunk::ToolCallStart {
                id: "call-1".into(),
                name: "generate_map".into(),
            },
            ChatChunk::ToolCallArgsDelta {
                id: "call-1".into(),
                args_fragment: r#"{"prompt":"crypt"}"#.into(),
            },
            ChatChunk::ToolCallDone {
                id: "call-1".into(),
            },
            ChatChunk::Done {
                reason: FinishReason::ToolUse,
            },
        ],
        vec![
            ChatChunk::TextDelta {
                text: "Done".into(),
            },
            ChatChunk::Done {
                reason: FinishReason::Stop,
            },
        ],
    ]));
    let messages = Arc::new(FakeMessages::default());
    let dispatcher = Arc::new(FakeToolDispatcher {
        execution: ToolExecution {
            result: json!({"ok": true}),
            is_error: false,
            handled_by: "image-provider".into(),
            media: vec![GeneratedAgentMedia::Image {
                mime_type: "image/png".into(),
                data_b64: "AA==".into(),
                kind: "map".into(),
            }],
        },
        calls: AtomicUsize::new(0),
    });
    let events = Arc::new(RecordingEvents::default());
    let service = service(
        provider,
        messages.clone(),
        dispatcher,
        events.clone(),
        Arc::new(NeverCancelled),
    );

    let output = service
        .execute(command("media-order"))
        .collect::<Vec<_>>()
        .await;
    assert!(output.iter().all(Result::is_ok));
    assert_eq!(
        *events.0.lock().unwrap(),
        [
            "tool_call_start",
            "image_generated",
            "tool_call_result",
            "text_delta",
            "agent_done"
        ]
    );
    assert_eq!(messages.values.lock().unwrap().len(), 4);
}

#[tokio::test]
async fn agent_turn_rejects_unknown_tool_without_calling_dispatcher() {
    let provider = Arc::new(FakeProvider::scripted(vec![
        vec![
            ChatChunk::ToolCallStart {
                id: "bad".into(),
                name: "unknown_tool".into(),
            },
            ChatChunk::ToolCallArgsDelta {
                id: "bad".into(),
                args_fragment: "{}".into(),
            },
            ChatChunk::ToolCallDone { id: "bad".into() },
            ChatChunk::Done {
                reason: FinishReason::ToolUse,
            },
        ],
        vec![ChatChunk::Done {
            reason: FinishReason::Stop,
        }],
    ]));
    let dispatcher = Arc::new(FakeToolDispatcher {
        execution: ToolExecution {
            result: json!({}),
            is_error: false,
            handled_by: "engine".into(),
            media: vec![],
        },
        calls: AtomicUsize::new(0),
    });
    let service = service(
        provider,
        Arc::new(FakeMessages::default()),
        dispatcher.clone(),
        Arc::new(NoopApplicationEventSink),
        Arc::new(NeverCancelled),
    );

    let output = service
        .execute(command("reject-tool"))
        .collect::<Vec<_>>()
        .await;
    assert!(output.iter().all(Result::is_ok));
    assert_eq!(dispatcher.calls.load(Ordering::SeqCst), 0);
    assert!(output
        .iter()
        .any(|item| matches!(item, Ok(AgentEvent::ToolCallResult { is_error: true, .. }))));
}

#[tokio::test]
async fn agent_turn_surfaces_provider_persistence_cancellation_and_duplicate_failures() {
    let dispatcher = Arc::new(FakeToolDispatcher {
        execution: ToolExecution {
            result: json!({}),
            is_error: false,
            handled_by: "engine".into(),
            media: vec![],
        },
        calls: AtomicUsize::new(0),
    });

    let provider_failure = service(
        Arc::new(FakeProvider::failing()),
        Arc::new(FakeMessages::default()),
        dispatcher.clone(),
        Arc::new(NoopApplicationEventSink),
        Arc::new(NeverCancelled),
    )
    .execute(command("provider-failure"))
    .collect::<Vec<_>>()
    .await;
    assert_eq!(
        provider_failure.last(),
        Some(&Err(AgentTurnError::Provider))
    );

    let failing_messages = Arc::new(FakeMessages::default());
    failing_messages.fail.store(true, Ordering::SeqCst);
    let provider = Arc::new(FakeProvider::scripted(vec![vec![ChatChunk::Done {
        reason: FinishReason::Stop,
    }]]));
    let persistence_failure = service(
        provider.clone(),
        failing_messages,
        dispatcher.clone(),
        Arc::new(NoopApplicationEventSink),
        Arc::new(NeverCancelled),
    )
    .execute(command("persistence-failure"))
    .collect::<Vec<_>>()
    .await;
    assert_eq!(persistence_failure, [Err(AgentTurnError::Persistence)]);
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);

    let cancelled = service(
        Arc::new(FakeProvider::scripted(vec![])),
        Arc::new(FakeMessages::default()),
        dispatcher.clone(),
        Arc::new(NoopApplicationEventSink),
        Arc::new(FixedCancellation(true)),
    )
    .execute(command("cancelled"))
    .collect::<Vec<_>>()
    .await;
    assert_eq!(cancelled, [Err(AgentTurnError::Cancelled)]);

    let duplicate_service = service(
        Arc::new(FakeProvider::scripted(vec![vec![ChatChunk::Done {
            reason: FinishReason::Stop,
        }]])),
        Arc::new(FakeMessages::default()),
        dispatcher,
        Arc::new(NoopApplicationEventSink),
        Arc::new(NeverCancelled),
    );
    let first = duplicate_service.clone().execute(command("same-request"));
    let duplicate = duplicate_service
        .clone()
        .execute(command("same-request"))
        .collect::<Vec<_>>()
        .await;
    assert_eq!(duplicate, [Err(AgentTurnError::DuplicateRequest)]);
    assert!(first.collect::<Vec<_>>().await.iter().all(Result::is_ok));
}

struct FakeCombatRepository {
    projection: Mutex<CombatProjection>,
    writes: AtomicUsize,
    fail_write: AtomicBool,
}

#[async_trait]
impl CombatRepository for FakeCombatRepository {
    async fn create(
        &self,
        _session_id: Uuid,
        projection: CombatProjection,
    ) -> Result<(), RepositoryError> {
        *self.projection.lock().unwrap() = projection;
        self.writes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn get(&self, _encounter_id: Uuid) -> Result<Option<CombatProjection>, RepositoryError> {
        Ok(Some(self.projection.lock().unwrap().clone()))
    }

    async fn compare_and_set(
        &self,
        expected_revision: u64,
        projection: CombatProjection,
    ) -> Result<(), RepositoryError> {
        if self.fail_write.load(Ordering::SeqCst) {
            return Err(RepositoryError::Operation {
                operation: "combat_cas",
                code: "injected",
            });
        }
        let mut current = self.projection.lock().unwrap();
        if current.revision != expected_revision {
            return Err(RepositoryError::RevisionConflict {
                expected: expected_revision,
                actual: current.revision,
            });
        }
        *current = projection;
        self.writes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn end(&self, _encounter_id: Uuid) -> Result<Option<CombatProjection>, RepositoryError> {
        let mut projection = self.projection.lock().unwrap();
        projection.snapshot.active = false;
        projection.revision = projection.revision.saturating_add(1);
        Ok(Some(projection.clone()))
    }
}

fn combat_fixture() -> (Arc<FakeCombatRepository>, CombatantId, CombatantId, Uuid) {
    let encounter_id = Uuid::new_v4();
    let attacker = CombatantId(Uuid::new_v4());
    let target = CombatantId(Uuid::new_v4());
    let repository = Arc::new(FakeCombatRepository {
        projection: Mutex::new(CombatProjection {
            schema_version: COMBAT_PROJECTION_VERSION,
            encounter_id,
            revision: 4,
            snapshot: CombatSnapshot {
                active: true,
                round: 1,
                current_combatant: Some(attacker),
                initiative: vec![
                    InitiativeEntry {
                        id: attacker,
                        roll: 20,
                        dex_tiebreak: 2,
                    },
                    InitiativeEntry {
                        id: target,
                        roll: 10,
                        dex_tiebreak: 1,
                    },
                ],
                combatants: vec![
                    Combatant::new(attacker, "Hero".into(), 20, 20, 15),
                    Combatant::new(target, "Goblin".into(), 12, 12, 10),
                ],
            },
            events: vec![],
        }),
        writes: AtomicUsize::new(0),
        fail_write: AtomicBool::new(false),
    });
    (repository, attacker, target, encounter_id)
}

fn resolve_command(
    request_id: &str,
    encounter_id: Uuid,
    attacker: CombatantId,
    target: CombatantId,
    revision: u64,
) -> ResolveCombatCommand {
    ResolveCombatCommand {
        request_id: request_id.into(),
        encounter_id,
        expected_revision: revision,
        rng_seed: 42,
        action: CombatActionCommand::Attack {
            attacker,
            target,
            attack_modifier: 20,
            damage_expr: DiceExpr {
                count: 1,
                die: Die::D6,
                modifier: 2,
            },
            damage_type: DamageType::Slashing,
        },
    }
}

#[tokio::test]
async fn combat_resolution_is_authoritative_versioned_and_idempotent() {
    let (repository, attacker, target, encounter_id) = combat_fixture();
    let use_case = ResolveCombatAction::new(repository.clone());
    let command = resolve_command("attack-1", encounter_id, attacker, target, 4);

    let first = use_case.execute(command.clone()).await.unwrap();
    assert_eq!(first.projection.revision, 5);
    assert!(!first.duplicate);
    assert_eq!(repository.writes.load(Ordering::SeqCst), 1);

    let duplicate = use_case.execute(command).await.unwrap();
    assert!(duplicate.duplicate);
    assert_eq!(duplicate.projection.revision, 5);
    assert_eq!(repository.writes.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn combat_resolution_rejects_stale_revision_and_has_no_partial_write() {
    let (repository, attacker, target, encounter_id) = combat_fixture();
    let use_case = ResolveCombatAction::new(repository.clone());
    let stale = use_case
        .execute(resolve_command("stale", encounter_id, attacker, target, 3))
        .await;
    assert!(matches!(
        stale,
        Err(ResolveCombatError::StaleRevision { actual: 4, .. })
    ));
    assert_eq!(repository.writes.load(Ordering::SeqCst), 0);

    repository.fail_write.store(true, Ordering::SeqCst);
    let failed = use_case
        .execute(resolve_command(
            "write-fails",
            encounter_id,
            attacker,
            target,
            4,
        ))
        .await;
    assert!(matches!(failed, Err(ResolveCombatError::Persistence)));
    assert_eq!(repository.projection.lock().unwrap().revision, 4);
    assert_eq!(repository.writes.load(Ordering::SeqCst), 0);
}
