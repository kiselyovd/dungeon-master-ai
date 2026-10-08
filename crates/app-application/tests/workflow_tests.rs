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
    requests: Mutex<Vec<ChatRequest>>,
}

impl FakeProvider {
    fn scripted(rounds: Vec<Vec<ChatChunk>>) -> Self {
        Self {
            rounds: Mutex::new(rounds.into_iter().map(Ok).collect()),
            calls: AtomicUsize::new(0),
            requests: Mutex::new(vec![]),
        }
    }

    fn failing() -> Self {
        Self {
            rounds: Mutex::new(VecDeque::from([Err(LlmError::Network("offline".into()))])),
            calls: AtomicUsize::new(0),
            requests: Mutex::new(vec![]),
        }
    }
}

#[async_trait]
impl LlmProvider for FakeProvider {
    async fn stream_chat(&self, request: ChatRequest) -> Result<ChunkStream, LlmError> {
        self.requests.lock().unwrap().push(request);
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
            reasoning_is_private: false,
            adventure_active: false,
            system_prompt: request.base_system_prompt,
            combat_active: false,
            public_intention_context: serde_json::Value::Null,
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
    async fn replay(
        &self,
        _request: ToolDispatchRequest,
    ) -> Result<Option<ToolExecution>, ToolDispatchError> {
        Ok(None)
    }
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
        player_verifier: None,
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
    provider: Arc<dyn LlmProvider>,
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

#[tokio::test]
async fn recovered_final_reply_is_delivered_without_generating_or_appending_again() {
    let provider = Arc::new(FakeProvider::failing());
    let messages = Arc::new(FakeMessages::default());
    let dispatcher = Arc::new(FakeToolDispatcher {
        execution: ToolExecution {
            result: json!({}),
            is_error: false,
            handled_by: "engine".into(),
            media: vec![],
        },
        calls: AtomicUsize::new(0),
    });
    let events = service(
        provider.clone(),
        messages.clone(),
        dispatcher,
        Arc::new(NoopApplicationEventSink),
        Arc::new(NeverCancelled),
    )
    .execute_recovered(
        command("final-checkpoint"),
        Some(vec![
            ChatMessage::user_text("act"),
            ChatMessage::Assistant {
                content: "Confirmed reply".into(),
            },
        ]),
    )
    .collect::<Vec<_>>()
    .await;
    assert!(events
        .iter()
        .any(|event| matches!(event,Ok(AgentEvent::TextDelta{text}) if text=="Confirmed reply")));
    assert!(matches!(
        events.last(),
        Some(Ok(AgentEvent::AgentDone { .. }))
    ));
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
    assert!(messages.values.lock().unwrap().is_empty());
}

#[tokio::test]
async fn provider_eof_does_not_commit_an_unfinished_assistant_reply() {
    let messages = Arc::new(FakeMessages::default());
    let dispatcher = Arc::new(FakeToolDispatcher {
        execution: ToolExecution {
            result: json!({}),
            is_error: false,
            handled_by: "engine".into(),
            media: vec![],
        },
        calls: AtomicUsize::new(0),
    });
    let output = service(
        Arc::new(FakeProvider::scripted(vec![vec![ChatChunk::TextDelta {
            text: "Unfinished draft".into(),
        }]])),
        messages.clone(),
        dispatcher,
        Arc::new(NoopApplicationEventSink),
        Arc::new(NeverCancelled),
    )
    .execute(command("partial-eof"))
    .collect::<Vec<_>>()
    .await;
    assert_eq!(output.last(), Some(&Err(AgentTurnError::Provider)));
    assert!(!output
        .iter()
        .any(|event| matches!(event, Ok(AgentEvent::AgentDone { .. }))));
    assert!(matches!(
        messages.values.lock().unwrap().as_slice(),
        [ChatMessage::User { .. }]
    ));
}

struct HangingProvider {
    before_stream: bool,
    entered: tokio::sync::Notify,
}
#[async_trait]
impl LlmProvider for HangingProvider {
    async fn stream_chat(&self, _: ChatRequest) -> Result<ChunkStream, LlmError> {
        self.entered.notify_one();
        if self.before_stream {
            futures::future::pending().await
        } else {
            Ok(stream::pending().boxed())
        }
    }
    fn name(&self) -> &'static str {
        "hanging"
    }
    fn active_model(&self) -> &str {
        "hanging"
    }
    fn capabilities_for_model(&self, _: &str) -> Capabilities {
        Capabilities::default()
    }
}
#[tokio::test]
async fn cancellation_wakes_both_provider_connection_and_chunk_waits() {
    for before_stream in [true, false] {
        let provider = Arc::new(HangingProvider {
            before_stream,
            entered: tokio::sync::Notify::new(),
        });
        let cancellation =
            Arc::new(app_application::agent::operations::TurnCancellation::default());
        let dispatcher = Arc::new(FakeToolDispatcher {
            execution: ToolExecution {
                result: json!({}),
                is_error: false,
                handled_by: "engine".into(),
                media: vec![],
            },
            calls: AtomicUsize::new(0),
        });
        let events = service(
            provider.clone(),
            Arc::new(FakeMessages::default()),
            dispatcher,
            Arc::new(NoopApplicationEventSink),
            cancellation.clone(),
        )
        .execute(command("hanging"));
        let task = tokio::spawn(events.collect::<Vec<_>>());
        provider.entered.notified().await;
        cancellation.cancel();
        let output = tokio::time::timeout(std::time::Duration::from_millis(500), task)
            .await
            .expect("cancel must wake provider wait")
            .unwrap();
        assert_eq!(output.last(), Some(&Err(AgentTurnError::Cancelled)));
    }
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
                source: app_application::ports::media::ImageSource::Generated,
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
async fn agent_turn_coalesces_tiny_text_and_reasoning_chunks() {
    let mut chunks = Vec::new();
    for _ in 0..100 {
        chunks.push(ChatChunk::ThinkingDelta { text: "r".into() });
    }
    for _ in 0..100 {
        chunks.push(ChatChunk::TextDelta { text: "t".into() });
    }
    chunks.push(ChatChunk::Done {
        reason: FinishReason::Stop,
    });
    let service = service(
        Arc::new(FakeProvider::scripted(vec![chunks])),
        Arc::new(FakeMessages::default()),
        Arc::new(FakeToolDispatcher {
            execution: ToolExecution {
                result: json!({}),
                is_error: false,
                handled_by: "engine".into(),
                media: vec![],
            },
            calls: AtomicUsize::new(0),
        }),
        Arc::new(NoopApplicationEventSink),
        Arc::new(NeverCancelled),
    );

    let output = service
        .execute(command("coalesce"))
        .collect::<Vec<_>>()
        .await;
    let streamed = output.iter().filter(|event| {
        matches!(
            event,
            Ok(AgentEvent::TextDelta { .. } | AgentEvent::ReasoningText { .. })
        )
    });
    assert!(
        streamed.count() <= 8,
        "tiny provider chunks must not become hundreds of WebView SSE events"
    );
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
        vec![
            ChatChunk::TextDelta {
                text: "The requested tool is unavailable.".into(),
            },
            ChatChunk::Done {
                reason: FinishReason::Stop,
            },
        ],
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
        Arc::new(FakeProvider::scripted(vec![vec![
            ChatChunk::TextDelta {
                text: "Confirmed.".into(),
            },
            ChatChunk::Done {
                reason: FinishReason::Stop,
            },
        ]])),
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
    receipts: Mutex<std::collections::HashMap<(Uuid, String), (String, CombatProjection)>>,
    writes: AtomicUsize,
    fail_write: AtomicBool,
}

#[async_trait]
impl CombatRepository for FakeCombatRepository {
    async fn find_action(
        &self,
        encounter_id: Uuid,
        request_id: &str,
        signature: &str,
    ) -> Result<Option<CombatProjection>, RepositoryError> {
        match self
            .receipts
            .lock()
            .unwrap()
            .get(&(encounter_id, request_id.into()))
        {
            Some((original, projection)) if original == signature => Ok(Some(projection.clone())),
            Some(_) => Err(RepositoryError::Operation {
                operation: "combat_receipt",
                code: "combat_request_mismatch",
            }),
            None => Ok(None),
        }
    }
    async fn commit_action(
        &self,
        expected_revision: u64,
        projection: CombatProjection,
        request_id: &str,
        signature: &str,
    ) -> Result<app_application::models::combat::CombatActionCommit, RepositoryError> {
        let mut receipts = self.receipts.lock().unwrap();
        let key = (projection.encounter_id, request_id.into());
        if let Some((original, recorded)) = receipts.get(&key) {
            if original != signature {
                return Err(RepositoryError::Operation {
                    operation: "combat_receipt",
                    code: "combat_request_mismatch",
                });
            }
            return Ok(app_application::models::combat::CombatActionCommit {
                projection: recorded.clone(),
                duplicate: true,
            });
        }
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
        *current = projection.clone();
        self.writes.fetch_add(1, Ordering::SeqCst);
        receipts.insert(key, (signature.into(), projection.clone()));
        Ok(app_application::models::combat::CombatActionCommit {
            projection,
            duplicate: false,
        })
    }
    async fn create(
        &self,
        _session_id: Uuid,
        projection: CombatProjection,
    ) -> Result<CombatProjection, RepositoryError> {
        *self.projection.lock().unwrap() = projection.clone();
        self.writes.fetch_add(1, Ordering::SeqCst);
        Ok(projection)
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
        receipts: Mutex::new(std::collections::HashMap::new()),
        projection: Mutex::new(CombatProjection {
            authored: None,

            adventure: None,

            campaign_scope: None,
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

#[tokio::test]
async fn invalid_provider_terminal_rounds_never_commit_or_dispatch() {
    let call = || {
        vec![
            ChatChunk::ToolCallStart {
                id: "call-1".into(),
                name: "roll_dice".into(),
            },
            ChatChunk::ToolCallArgsDelta {
                id: "call-1".into(),
                args_fragment: r#"{"dice":"1d20"}"#.into(),
            },
            ChatChunk::ToolCallDone {
                id: "call-1".into(),
            },
        ]
    };
    let mut cases = vec![
        (
            "length",
            vec![
                ChatChunk::TextDelta {
                    text: "unfinished".into(),
                },
                ChatChunk::Done {
                    reason: FinishReason::Length,
                },
            ],
        ),
        (
            "error",
            vec![
                ChatChunk::TextDelta {
                    text: "unfinished".into(),
                },
                ChatChunk::Done {
                    reason: FinishReason::Error,
                },
            ],
        ),
        (
            "empty-stop",
            vec![ChatChunk::Done {
                reason: FinishReason::Stop,
            }],
        ),
        (
            "blank-stop",
            vec![
                ChatChunk::TextDelta { text: " \n".into() },
                ChatChunk::Done {
                    reason: FinishReason::Stop,
                },
            ],
        ),
        (
            "empty-tool-use",
            vec![ChatChunk::Done {
                reason: FinishReason::ToolUse,
            }],
        ),
        (
            "unfinished-tool",
            vec![
                ChatChunk::ToolCallStart {
                    id: "call-1".into(),
                    name: "roll_dice".into(),
                },
                ChatChunk::Done {
                    reason: FinishReason::ToolUse,
                },
            ],
        ),
    ];
    for (label, reason) in [
        ("truncated-tools", FinishReason::Length),
        ("stop-with-tools", FinishReason::Stop),
    ] {
        let mut chunks = call();
        chunks.push(ChatChunk::Done { reason });
        cases.push((label, chunks));
    }
    for (label, chunks) in cases {
        let messages = Arc::new(FakeMessages::default());
        let dispatcher = Arc::new(FakeToolDispatcher {
            execution: ToolExecution {
                result: json!({}),
                is_error: false,
                handled_by: "engine".into(),
                media: vec![],
            },
            calls: AtomicUsize::new(0),
        });
        let output = service(
            Arc::new(FakeProvider::scripted(vec![chunks])),
            messages.clone(),
            dispatcher.clone(),
            Arc::new(NoopApplicationEventSink),
            Arc::new(NeverCancelled),
        )
        .execute(command(label))
        .collect::<Vec<_>>()
        .await;
        assert!(
            matches!(output.last(), Some(Err(AgentTurnError::Provider))),
            "{label}: {output:?}"
        );
        assert!(
            !output
                .iter()
                .any(|event| matches!(event, Ok(AgentEvent::AgentDone { .. }))),
            "{label}"
        );
        assert_eq!(dispatcher.calls.load(Ordering::SeqCst), 0, "{label}");
        assert_eq!(
            messages.values.lock().unwrap().len(),
            1,
            "only user persists: {label}"
        );
    }
}

#[tokio::test]
async fn exhausted_rounds_resume_confirmed_tools_without_repeating_dispatch() {
    let provider = Arc::new(FakeProvider::scripted(vec![
        vec![
            ChatChunk::ToolCallStart {
                id: "confirmed-roll".into(),
                name: "roll_dice".into(),
            },
            ChatChunk::ToolCallArgsDelta {
                id: "confirmed-roll".into(),
                args_fragment: r#"{"dice":"1d20"}"#.into(),
            },
            ChatChunk::ToolCallDone {
                id: "confirmed-roll".into(),
            },
            ChatChunk::Done {
                reason: FinishReason::ToolUse,
            },
        ],
        vec![
            ChatChunk::TextDelta {
                text: "Confirmed result".into(),
            },
            ChatChunk::Done {
                reason: FinishReason::Stop,
            },
        ],
    ]));
    let messages = Arc::new(FakeMessages::default());
    let dispatcher = Arc::new(FakeToolDispatcher {
        execution: ToolExecution {
            result: json!({"total":14}),
            is_error: false,
            handled_by: "engine".into(),
            media: vec![],
        },
        calls: AtomicUsize::new(0),
    });
    let mut one_round = config();
    one_round.max_rounds = 1;
    let service = Arc::new(AgentTurnService::new(
        provider.clone(),
        Arc::new(FakeContext),
        messages.clone(),
        dispatcher.clone(),
        Arc::new(NoopRuntimeCoordinator),
        Arc::new(NoopApplicationEventSink),
        Arc::new(NeverCancelled),
        one_round,
    ));
    let original = command("round-limit");
    let first = service
        .clone()
        .execute(original.clone())
        .collect::<Vec<_>>()
        .await;
    assert!(
        matches!(first.last(), Some(Err(AgentTurnError::Provider))),
        "{first:?}"
    );
    assert_eq!(dispatcher.calls.load(Ordering::SeqCst), 1);
    let checkpoint = messages.values.lock().unwrap().clone();
    assert!(matches!(
        checkpoint.last(),
        Some(ChatMessage::ToolResult(_))
    ));
    let resumed = service
        .execute_recovered(original, Some(checkpoint))
        .collect::<Vec<_>>()
        .await;
    assert!(
        matches!(resumed.last(), Some(Ok(AgentEvent::AgentDone { .. }))),
        "{resumed:?}"
    );
    assert_eq!(dispatcher.calls.load(Ordering::SeqCst), 1);
    assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        messages
            .values
            .lock()
            .unwrap()
            .iter()
            .filter(|m| matches!(m, ChatMessage::User { .. }))
            .count(),
        1
    );
}

struct AuthoredAgencyContext;
#[async_trait]
impl AgentContextPort for AuthoredAgencyContext {
    async fn build(&self, request: AgentContextRequest) -> Result<AgentContext, AgentContextError> {
        Ok(AgentContext {
            reasoning_is_private: true,
            adventure_active: true,
            system_prompt: request.base_system_prompt,
            combat_active: false,
            public_intention_context: serde_json::Value::Null,
        })
    }
}
fn agency_tool(id: &str, name: &str, args: serde_json::Value) -> Vec<ChatChunk> {
    vec![
        ChatChunk::ToolCallStart {
            id: id.into(),
            name: name.into(),
        },
        ChatChunk::ToolCallArgsDelta {
            id: id.into(),
            args_fragment: args.to_string(),
        },
        ChatChunk::ToolCallDone { id: id.into() },
    ]
}
fn agency_round(mut chunks: Vec<ChatChunk>) -> Vec<ChatChunk> {
    chunks.push(ChatChunk::Done {
        reason: FinishReason::ToolUse,
    });
    chunks
}
fn agency_final() -> Vec<ChatChunk> {
    vec![
        ChatChunk::TextDelta {
            text: "Confirmed player choice".into(),
        },
        ChatChunk::Done {
            reason: FinishReason::Stop,
        },
    ]
}
#[derive(Debug)]
struct SingleFixtureVerifier;
#[async_trait]
impl app_application::agent::intentions::PlayerIntentionVerifier for SingleFixtureVerifier {
    async fn assess(
        &self,
        check: app_application::agent::intentions::PlayerIntentionCheck,
    ) -> Result<
        app_application::agent::intentions::PlayerIntentionDecision,
        app_application::agent::intentions::PlayerIntentionError,
    > {
        Ok(
            app_application::agent::intentions::PlayerIntentionDecision {
                allowed: check.resolved.is_empty(),
                further_requested: false,
            },
        )
    }
}
fn agency_service(
    provider: Arc<dyn LlmProvider>,
    messages: Arc<FakeMessages>,
    dispatcher: Arc<FakeToolDispatcher>,
    max_rounds: usize,
) -> Arc<AgentTurnService> {
    let mut settings = config();
    settings.max_rounds = max_rounds;
    settings.player_verifier = Some(Arc::new(SingleFixtureVerifier));
    settings.tools.exploration = [
        "adventure_action",
        "combat_action",
        "rest_action",
        "roll_dice",
        "query_rules",
        "end_combat",
        "quick_save",
    ]
    .iter()
    .map(|name| app_application::models::chat::Tool {
        name: (*name).into(),
        description: String::new(),
        parameters: json!({}),
    })
    .collect();
    Arc::new(AgentTurnService::new(
        provider,
        Arc::new(AuthoredAgencyContext),
        messages,
        dispatcher,
        Arc::new(NoopRuntimeCoordinator),
        Arc::new(NoopApplicationEventSink),
        Arc::new(NeverCancelled),
        settings,
    ))
}
fn agency_dispatcher() -> Arc<FakeToolDispatcher> {
    Arc::new(FakeToolDispatcher {
        execution: ToolExecution {
            result: json!({"confirmed":true}),
            is_error: false,
            handled_by: "engine".into(),
            media: vec![],
        },
        calls: AtomicUsize::new(0),
    })
}
#[tokio::test]
async fn authored_player_choice_cannot_expand_into_another_hero_action() {
    for (name, args) in [
        (
            "adventure_action",
            json!({"action":{"kind":"inspect_quay"}}),
        ),
        ("combat_action", json!({"kind":"move","x":8,"y":8})),
        ("rest_action", json!({"kind":"wait","minutes":1})),
    ] {
        for same_round in [false, true] {
            let first = agency_tool(
                "chosen",
                "adventure_action",
                json!({"action":{"kind":"move","location":"quay"}}),
            );
            let second = agency_tool("unchosen", name, args.clone());
            let rounds = if same_round {
                let mut both = first;
                both.extend(second);
                vec![agency_round(both), agency_final()]
            } else {
                vec![agency_round(first), agency_round(second), agency_final()]
            };
            let dispatcher = agency_dispatcher();
            let events = agency_service(
                Arc::new(FakeProvider::scripted(rounds)),
                Arc::new(FakeMessages::default()),
                dispatcher.clone(),
                3,
            )
            .execute(command("agency"))
            .collect::<Vec<_>>()
            .await;
            assert!(
                matches!(events.last(), Some(Ok(AgentEvent::AgentDone { .. }))),
                "{events:?}"
            );
            assert_eq!(
                dispatcher.calls.load(Ordering::SeqCst),
                1,
                "{name}, same_round={same_round}"
            );
            assert!(events.iter().any(|event|matches!(event,Ok(AgentEvent::ToolCallResult{id,is_error:true,result,..}) if id=="unchosen" && result["error"]=="player_intention_already_resolved")));
        }
    }
}
#[tokio::test]
async fn authored_choice_budget_survives_interrupted_turn_but_not_prior_turn_history() {
    let dispatcher = agency_dispatcher();
    let messages = Arc::new(FakeMessages::default());
    let original = command("agency-resume");
    let first = agency_service(
        Arc::new(FakeProvider::scripted(vec![agency_round(agency_tool(
            "chosen",
            "adventure_action",
            json!({"action":{"kind":"move","location":"quay"}}),
        ))])),
        messages.clone(),
        dispatcher.clone(),
        1,
    )
    .execute(original.clone())
    .collect::<Vec<_>>()
    .await;
    assert!(matches!(first.last(), Some(Err(AgentTurnError::Provider))));
    let saved = messages.values.lock().unwrap().clone();
    let resumed = agency_service(
        Arc::new(FakeProvider::scripted(vec![
            agency_round(agency_tool(
                "unchosen",
                "rest_action",
                json!({"kind":"wait","minutes":1}),
            )),
            agency_final(),
        ])),
        messages.clone(),
        dispatcher.clone(),
        3,
    )
    .execute_recovered(original, Some(saved.clone()))
    .collect::<Vec<_>>()
    .await;
    assert!(matches!(
        resumed.last(),
        Some(Ok(AgentEvent::AgentDone { .. }))
    ));
    assert_eq!(dispatcher.calls.load(Ordering::SeqCst), 1);
    let mut next = command("agency-next");
    next.request.history = saved;
    let events = agency_service(
        Arc::new(FakeProvider::scripted(vec![
            agency_round(agency_tool(
                "new-choice",
                "adventure_action",
                json!({"action":{"kind":"inspect_quay"}}),
            )),
            agency_final(),
        ])),
        Arc::new(FakeMessages::default()),
        dispatcher.clone(),
        3,
    )
    .execute(next)
    .collect::<Vec<_>>()
    .await;
    assert!(matches!(
        events.last(),
        Some(Ok(AgentEvent::AgentDone { .. }))
    ));
    assert_eq!(dispatcher.calls.load(Ordering::SeqCst), 2);
}
#[tokio::test]
async fn rejected_authored_action_can_be_corrected_and_rules_remain_available() {
    let dispatcher = agency_dispatcher();
    let mut calls = agency_tool(
        "invalid",
        "adventure_action",
        json!({"action":{"kind":"move","location":"moon"}}),
    );
    calls.extend(agency_tool(
        "chosen",
        "adventure_action",
        json!({"action":{"kind":"move","location":"quay"}}),
    ));
    calls.extend(agency_tool(
        "rules",
        "query_rules",
        json!({"question":"Armor class"}),
    ));
    let events = agency_service(
        Arc::new(FakeProvider::scripted(vec![
            agency_round(calls),
            agency_final(),
        ])),
        Arc::new(FakeMessages::default()),
        dispatcher.clone(),
        3,
    )
    .execute(command("agency-corrected"))
    .collect::<Vec<_>>()
    .await;
    assert!(matches!(
        events.last(),
        Some(Ok(AgentEvent::AgentDone { .. }))
    ));
    assert_eq!(dispatcher.calls.load(Ordering::SeqCst), 2);
    assert!(events.iter().any(
        |event| matches!(event,Ok(AgentEvent::ToolCallResult{id,is_error:false,..}) if id=="rules")
    ));
}

#[tokio::test]
async fn definitive_engine_refusal_does_not_authorize_end_turn_or_new_dice() {
    let dispatcher = Arc::new(FakeToolDispatcher {
        execution: ToolExecution {
            result: json!({"error":"hero_combat_rejected"}),
            is_error: true,
            handled_by: "engine".into(),
            media: vec![],
        },
        calls: AtomicUsize::new(0),
    });
    let first = agency_round(agency_tool(
        "requested-attack",
        "combat_action",
        json!({"kind":"weapon","target":Uuid::new_v4(),"weapon_id":"longbow"}),
    ));
    let mut other = agency_tool(
        "unasked-end-turn",
        "combat_action",
        json!({"kind":"end_turn"}),
    );
    other.extend(agency_tool(
        "unasked-dice",
        "roll_dice",
        json!({"dice":"1d20"}),
    ));
    let events = agency_service(
        Arc::new(FakeProvider::scripted(vec![
            first,
            agency_round(other),
            agency_final(),
        ])),
        Arc::new(FakeMessages::default()),
        dispatcher.clone(),
        3,
    )
    .execute(command("refused-attack"))
    .collect::<Vec<_>>()
    .await;
    assert!(
        matches!(events.last(), Some(Ok(AgentEvent::AgentDone { .. }))),
        "{events:?}"
    );
    assert_eq!(dispatcher.calls.load(Ordering::SeqCst), 1);
    for expected in ["unasked-end-turn", "unasked-dice"] {
        assert!(events.iter().any(|event|matches!(event,Ok(AgentEvent::ToolCallResult{id,is_error:true,result,..}) if id==expected && result["error"]=="player_intention_already_resolved")));
    }
}

#[derive(Debug)]
struct CompoundFixtureVerifier;
#[async_trait]
impl app_application::agent::intentions::PlayerIntentionVerifier for CompoundFixtureVerifier {
    async fn assess(
        &self,
        check: app_application::agent::intentions::PlayerIntentionCheck,
    ) -> Result<
        app_application::agent::intentions::PlayerIntentionDecision,
        app_application::agent::intentions::PlayerIntentionError,
    > {
        assert_eq!(check.player_message, "Move to the quay and inspect it");
        Ok(
            app_application::agent::intentions::PlayerIntentionDecision {
                allowed: check.candidate.is_some(),
                further_requested: check.resolved.is_empty(),
            },
        )
    }
}

#[derive(Debug)]
struct IncorrectForecastFixtureVerifier;
#[async_trait]
impl app_application::agent::intentions::PlayerIntentionVerifier
    for IncorrectForecastFixtureVerifier
{
    async fn assess(
        &self,
        check: app_application::agent::intentions::PlayerIntentionCheck,
    ) -> Result<
        app_application::agent::intentions::PlayerIntentionDecision,
        app_application::agent::intentions::PlayerIntentionError,
    > {
        Ok(
            app_application::agent::intentions::PlayerIntentionDecision {
                allowed: check.candidate.is_some(),
                further_requested: check.candidate.is_none() && check.resolved.len() < 2,
            },
        )
    }
}
#[tokio::test]
async fn final_completion_ledger_can_correct_a_false_single_action_forecast() {
    let dispatcher = agency_dispatcher();
    let messages = Arc::new(FakeMessages::default());
    let mut settings = config();
    settings.max_rounds = 4;
    settings.player_verifier = Some(Arc::new(IncorrectForecastFixtureVerifier));
    settings.tools.exploration = vec![app_application::models::chat::Tool {
        name: "adventure_action".into(),
        description: String::new(),
        parameters: json!({}),
    }];
    let provider = Arc::new(FakeProvider::scripted(vec![
        agency_round(agency_tool(
            "move",
            "adventure_action",
            json!({"action":{"kind":"move","location":"quay"}}),
        )),
        vec![
            ChatChunk::TextDelta {
                text: "Premature completion after only movement".into(),
            },
            ChatChunk::Done {
                reason: FinishReason::Stop,
            },
        ],
        agency_round(agency_tool(
            "inspect",
            "adventure_action",
            json!({"action":{"kind":"inspect_quay"}}),
        )),
        agency_final(),
    ]));
    let service = Arc::new(AgentTurnService::new(
        provider,
        Arc::new(AuthoredAgencyContext),
        messages.clone(),
        dispatcher.clone(),
        Arc::new(NoopRuntimeCoordinator),
        Arc::new(NoopApplicationEventSink),
        Arc::new(NeverCancelled),
        settings,
    ));
    let mut input = command("false-forecast");
    input.request.player_message = "Move to quay and inspect it".into();
    let events = service.execute(input).collect::<Vec<_>>().await;
    assert!(
        matches!(events.last(), Some(Ok(AgentEvent::AgentDone { .. }))),
        "{events:?}"
    );
    assert_eq!(dispatcher.calls.load(Ordering::SeqCst), 2);
    assert!(!events.iter().any(
        |event| matches!(event,Ok(AgentEvent::TextDelta{text}) if text.contains("Premature"))
    ));
    assert!(!messages.values.lock().unwrap().iter().any(|message|matches!(message,ChatMessage::Assistant {content} if content.contains("Premature"))));
}
#[tokio::test]
async fn explicitly_requested_compound_intentions_are_not_reduced_to_one_action() {
    let dispatcher = agency_dispatcher();
    let messages = Arc::new(FakeMessages::default());
    let mut settings = config();
    settings.player_verifier = Some(Arc::new(CompoundFixtureVerifier));
    settings.tools.exploration = vec![app_application::models::chat::Tool {
        name: "adventure_action".into(),
        description: String::new(),
        parameters: json!({}),
    }];
    let mut both = agency_tool(
        "travel",
        "adventure_action",
        json!({"action":{"kind":"move","location":"quay"}}),
    );
    both.extend(agency_tool(
        "inspect",
        "adventure_action",
        json!({"action":{"kind":"inspect_quay"}}),
    ));
    let service = Arc::new(AgentTurnService::new(
        Arc::new(FakeProvider::scripted(vec![
            agency_round(both),
            agency_final(),
        ])),
        Arc::new(AuthoredAgencyContext),
        messages,
        dispatcher.clone(),
        Arc::new(NoopRuntimeCoordinator),
        Arc::new(NoopApplicationEventSink),
        Arc::new(NeverCancelled),
        settings,
    ));
    let mut input = command("compound");
    input.request.player_message = "Move to the quay and inspect it".into();
    let events = service.execute(input).collect::<Vec<_>>().await;
    assert!(
        matches!(events.last(), Some(Ok(AgentEvent::AgentDone { .. }))),
        "{events:?}"
    );
    assert_eq!(dispatcher.calls.load(Ordering::SeqCst), 2);
}
#[derive(Debug)]
struct PendingActionFixtureVerifier;
#[async_trait]
impl app_application::agent::intentions::PlayerIntentionVerifier for PendingActionFixtureVerifier {
    async fn assess(
        &self,
        check: app_application::agent::intentions::PlayerIntentionCheck,
    ) -> Result<
        app_application::agent::intentions::PlayerIntentionDecision,
        app_application::agent::intentions::PlayerIntentionError,
    > {
        Ok(
            app_application::agent::intentions::PlayerIntentionDecision {
                allowed: check.candidate.is_some(),
                further_requested: check.candidate.is_none() && check.resolved.is_empty(),
            },
        )
    }
}
#[tokio::test]
async fn prose_cannot_complete_an_unexecuted_player_action_or_leak_a_phantom_arrival() {
    let dispatcher = agency_dispatcher();
    let messages = Arc::new(FakeMessages::default());
    let mut settings = config();
    settings.player_verifier = Some(Arc::new(PendingActionFixtureVerifier));
    settings.tools.exploration = vec![app_application::models::chat::Tool {
        name: "adventure_action".into(),
        description: String::new(),
        parameters: json!({}),
    }];
    let phantom = vec![
        ChatChunk::TextDelta {
            text: "You have arrived, but no tool ran".into(),
        },
        ChatChunk::Done {
            reason: FinishReason::Stop,
        },
    ];
    let provider = Arc::new(FakeProvider::scripted(vec![
        phantom,
        agency_round(agency_tool(
            "actual",
            "adventure_action",
            json!({"action":{"kind":"move","location":"quay"}}),
        )),
        agency_final(),
    ]));
    let service = Arc::new(AgentTurnService::new(
        provider,
        Arc::new(AuthoredAgencyContext),
        messages.clone(),
        dispatcher.clone(),
        Arc::new(NoopRuntimeCoordinator),
        Arc::new(NoopApplicationEventSink),
        Arc::new(NeverCancelled),
        settings,
    ));
    let events = service
        .execute(command("missing-action"))
        .collect::<Vec<_>>()
        .await;
    assert!(
        matches!(events.last(), Some(Ok(AgentEvent::AgentDone { .. }))),
        "{events:?}"
    );
    assert_eq!(dispatcher.calls.load(Ordering::SeqCst), 1);
    assert!(!events.iter().any(
        |event| matches!(event,Ok(AgentEvent::TextDelta{text}) if text.contains("no tool ran"))
    ));
    assert!(!messages.values.lock().unwrap().iter().any(|message|matches!(message,ChatMessage::Assistant{content} if content.contains("no tool ran"))));
}

#[derive(Debug)]
struct DeniedProposalFixtureVerifier;
#[async_trait]
impl app_application::agent::intentions::PlayerIntentionVerifier for DeniedProposalFixtureVerifier {
    async fn assess(
        &self,
        check: app_application::agent::intentions::PlayerIntentionCheck,
    ) -> Result<
        app_application::agent::intentions::PlayerIntentionDecision,
        app_application::agent::intentions::PlayerIntentionError,
    > {
        Ok(
            app_application::agent::intentions::PlayerIntentionDecision {
                allowed: check
                    .candidate
                    .as_ref()
                    .is_some_and(|(_, args)| args["action"]["kind"] == "move"),
                further_requested: check.candidate.is_none() && check.resolved.is_empty(),
            },
        )
    }
}
#[tokio::test]
async fn permission_denial_cannot_resolve_the_human_request_or_publish_phantom_progress() {
    let dispatcher = agency_dispatcher();
    let messages = Arc::new(FakeMessages::default());
    let mut settings = config();
    settings.max_rounds = 4;
    settings.player_verifier = Some(Arc::new(DeniedProposalFixtureVerifier));
    settings.tools.exploration = vec![app_application::models::chat::Tool {
        name: "adventure_action".into(),
        description: String::new(),
        parameters: json!({}),
    }];
    let provider = Arc::new(FakeProvider::scripted(vec![
        agency_round(agency_tool(
            "denied",
            "adventure_action",
            json!({"action":{"kind":"inspect_quay"}}),
        )),
        vec![
            ChatChunk::TextDelta {
                text: "Phantom arrival without movement".into(),
            },
            ChatChunk::Done {
                reason: FinishReason::Stop,
            },
        ],
        agency_round(agency_tool(
            "requested",
            "adventure_action",
            json!({"action":{"kind":"move","location":"quay"}}),
        )),
        agency_final(),
    ]));
    let service = Arc::new(AgentTurnService::new(
        provider,
        Arc::new(AuthoredAgencyContext),
        messages.clone(),
        dispatcher.clone(),
        Arc::new(NoopRuntimeCoordinator),
        Arc::new(NoopApplicationEventSink),
        Arc::new(NeverCancelled),
        settings,
    ));
    let mut input = command("denial-is-not-an-outcome");
    input.request.player_message = "I go to quay".into();
    let events = service.execute(input).collect::<Vec<_>>().await;
    assert!(
        matches!(events.last(), Some(Ok(AgentEvent::AgentDone { .. }))),
        "{events:?}"
    );
    assert_eq!(dispatcher.calls.load(Ordering::SeqCst), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event,Ok(AgentEvent::TextDelta{text}) if text.contains("Phantom"))));
    assert!(!messages.values.lock().unwrap().iter().any(
        |message| matches!(message,ChatMessage::Assistant {content} if content.contains("Phantom"))
    ));
}

#[tokio::test]
async fn confirmed_obstruction_supplies_the_actual_refusal_cause_to_the_narrator() {
    let dispatcher = Arc::new(FakeToolDispatcher {
        execution: ToolExecution {
            result: json!({"error":"combat_obstructed","resolution":"rejected"}),
            is_error: true,
            handled_by: "engine".into(),
            media: vec![],
        },
        calls: AtomicUsize::new(0),
    });
    let provider = Arc::new(FakeProvider::scripted(vec![
        agency_round(agency_tool(
            "shot",
            "combat_action",
            json!({"kind":"weapon","mode":"ranged","weapon_id":"longbow","target":Uuid::new_v4()}),
        )),
        agency_final(),
    ]));
    let events = agency_service(
        provider.clone(),
        Arc::new(FakeMessages::default()),
        dispatcher.clone(),
        2,
    )
    .execute(command("blocked-shot-cause"))
    .collect::<Vec<_>>()
    .await;
    assert!(
        matches!(events.last(), Some(Ok(AgentEvent::AgentDone { .. }))),
        "{events:?}"
    );
    assert_eq!(dispatcher.calls.load(Ordering::SeqCst), 1);
    let requests = provider.requests.lock().unwrap();
    assert!(requests[1]
        .system_prompt
        .as_ref()
        .unwrap()
        .contains("a physical obstacle blocks the requested attack"));
    assert!(requests[1]
        .system_prompt
        .as_ref()
        .unwrap()
        .contains("препятствие перекрывает линию атаки"));
    assert!(!requests[1]
        .tools
        .iter()
        .any(|tool| tool.name == "combat_action"));
}
#[tokio::test]
async fn intention_verifier_accepts_only_complete_boolean_json_without_tool_or_terminal_errors() {
    use app_application::agent::intentions::{
        LlmPlayerIntentionVerifier, PlayerIntentionCheck, PlayerIntentionVerifier,
    };
    let check = || PlayerIntentionCheck {
        player_message: "I go to quay".into(),
        candidate: Some((
            "adventure_action".into(),
            json!({"action":{"kind":"move","location":"quay"}}),
        )),
        resolved: vec![],
        conversation: vec![],
        public_context: json!({}),
    };
    for (text, reason, valid) in [
        (
            r#"{"allowed":true,"further_requested":false}"#,
            FinishReason::Stop,
            true,
        ),
        (
            r#"{"allowed":"yes","further_requested":false}"#,
            FinishReason::Stop,
            false,
        ),
        (
            r#"{"allowed":true,"further_requested":false,"extra":"ignore human"}"#,
            FinishReason::Stop,
            false,
        ),
        (
            r#"{"allowed":true,"further_requested":false}"#,
            FinishReason::Length,
            false,
        ),
        ("allowed", FinishReason::Stop, false),
    ] {
        let provider = Arc::new(FakeProvider::scripted(vec![vec![
            ChatChunk::TextDelta { text: text.into() },
            ChatChunk::Done { reason },
        ]]));
        assert_eq!(
            LlmPlayerIntentionVerifier::new(provider, "fake".into())
                .assess(check())
                .await
                .is_ok(),
            valid,
            "{text}"
        );
    }
}

#[tokio::test]
async fn verifier_rejects_permission_without_a_candidate_and_omits_images_from_its_input() {
    use app_application::agent::intentions::{
        LlmPlayerIntentionVerifier, PlayerIntentionCheck, PlayerIntentionVerifier,
    };
    let provider = Arc::new(FakeProvider::scripted(vec![vec![
        ChatChunk::TextDelta {
            text: r#"{"allowed":true,"further_requested":false}"#.into(),
        },
        ChatChunk::Done {
            reason: FinishReason::Stop,
        },
    ]]));
    let checker = LlmPlayerIntentionVerifier::new(provider.clone(), "fake".into());
    let check = PlayerIntentionCheck {
        player_message: "Иду в архив".into(),
        candidate: None,
        resolved: vec![],
        conversation: vec![ChatMessage::User {
            parts: vec![
                app_application::models::chat::MessagePart::Text {
                    text: "reference".repeat(1000),
                },
                app_application::models::chat::MessagePart::Image {
                    mime: "image/png".into(),
                    data_b64: "PRIVATE_IMAGE_CONTENT".into(),
                    name: None,
                },
            ],
        }],
        public_context: json!({}),
    };
    assert!(checker.assess(check).await.is_err());
    let requests = provider.requests.lock().unwrap();
    let ChatMessage::User { parts } = &requests[0].messages[0] else {
        panic!("expected quoted input")
    };
    let app_application::models::chat::MessagePart::Text { text } = &parts[0] else {
        panic!("text only")
    };
    assert!(!text.contains("PRIVATE_IMAGE_CONTENT"));
    let value: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(
        value["conversation"][0]["text"]
            .as_str()
            .unwrap()
            .chars()
            .count(),
        1200
    );
    assert_eq!(value["player_message"], "Иду в архив");
    assert!(value["candidate"].is_null());
}

#[tokio::test]
async fn completion_ledger_preserves_skill_success_and_failure_for_conditional_commands() {
    use app_application::agent::intentions::{
        LlmPlayerIntentionVerifier, PlayerIntentionCheck, PlayerIntentionVerifier,
        ResolvedPlayerIntention,
    };
    for success in [true, false] {
        let pending = if success {
            vec![]
        } else {
            vec!["перемещаюсь к выходу"]
        };
        let provider = Arc::new(FakeProvider::scripted(vec![vec![
            ChatChunk::TextDelta {
                text: json!({"remaining_commands":pending}).to_string(),
            },
            ChatChunk::Done {
                reason: FinishReason::Stop,
            },
        ]]));
        let decision=LlmPlayerIntentionVerifier::new(provider.clone(),"fixture".into()).assess(PlayerIntentionCheck {
            player_message:"Проверяю Скрытность; если проверка провалится, перемещаюсь к выходу".into(),candidate:None,conversation:vec![],
            resolved:vec![ResolvedPlayerIntention {tool_call_id:"skill".into(),tool_name:"roll_dice".into(),args:json!({"dice":"1d20","check":{"skill":"stealth"}}),rejected:false,result:json!({"success":success,"total":if success {18} else {7},"dc":12,"skill":"stealth","rolls":[if success {14} else {3}],"modifier":4})}],
            public_context:json!({"adventure":{"available_actions":[{"kind":"finish"}]},"authored_map":{"blocked":[1]},"targets":[]}),
        }).await.unwrap();
        assert!(!decision.allowed);
        assert_eq!(decision.further_requested, !success);
        let requests = provider.requests.lock().unwrap();
        let ChatMessage::User { parts } = &requests[0].messages[0] else {
            panic!("input missing")
        };
        let app_application::models::chat::MessagePart::Text { text } = &parts[0] else {
            panic!("text input required")
        };
        let input: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(
            input["completed_attempts"][0]["check_result"]["success"],
            success
        );
        assert_eq!(
            input["completed_attempts"][0]["check_result"]["total"],
            if success { 18 } else { 7 }
        );
        assert_eq!(input["completed_attempts"][0]["check_result"]["dc"], 12);
        assert!(input["public_context"].is_null());
        assert!(input["reference_labels"]["adventure"].is_null());
    }
}

#[tokio::test]
async fn legacy_end_combat_cannot_bypass_a_resolved_human_intention() {
    let dispatcher = agency_dispatcher();
    let first = agency_round(agency_tool(
        "asked",
        "adventure_action",
        json!({"action":{"kind":"move","location":"quay"}}),
    ));
    let extra = agency_round(agency_tool("unasked-close", "end_combat", json!({})));
    let events = agency_service(
        Arc::new(FakeProvider::scripted(vec![first, extra, agency_final()])),
        Arc::new(FakeMessages::default()),
        dispatcher.clone(),
        3,
    )
    .execute(command("don't-close"))
    .collect::<Vec<_>>()
    .await;
    assert!(
        matches!(events.last(), Some(Ok(AgentEvent::AgentDone { .. }))),
        "{events:?}"
    );
    assert_eq!(dispatcher.calls.load(Ordering::SeqCst), 1);
    assert!(events.iter().any(|event|matches!(event,Ok(AgentEvent::ToolCallResult{id,is_error:true,..}) if id=="unasked-close")));
}

#[derive(Debug)]
struct RawConsequenceFixtureVerifier {
    allowed: bool,
    candidates: AtomicUsize,
}

struct ConfirmedRawReceipt;
#[async_trait]
impl ToolDispatcher for ConfirmedRawReceipt {
    async fn replay(
        &self,
        request: ToolDispatchRequest,
    ) -> Result<Option<ToolExecution>, ToolDispatchError> {
        assert_eq!(request.tool_call_id, "committed-raw");
        Ok(Some(ToolExecution {
            result: json!({"hp":7,"confirmed":true}),
            is_error: false,
            handled_by: "engine".into(),
            media: vec![],
        }))
    }
    async fn execute(&self, _: ToolDispatchRequest) -> Result<ToolExecution, ToolDispatchError> {
        panic!("confirmed raw damage must not execute again")
    }
}
#[tokio::test]
async fn confirmed_raw_receipt_precedes_a_new_denial_and_is_delivered_truthfully() {
    let verifier = Arc::new(RawConsequenceFixtureVerifier {
        allowed: false,
        candidates: AtomicUsize::new(0),
    });
    let mut settings = config();
    settings.player_verifier = Some(verifier.clone());
    let provider = Arc::new(FakeProvider::scripted(vec![agency_final()]));
    let service = Arc::new(AgentTurnService::new(
        provider,
        Arc::new(FakeContext),
        Arc::new(FakeMessages::default()),
        Arc::new(ConfirmedRawReceipt),
        Arc::new(NoopRuntimeCoordinator),
        Arc::new(NoopApplicationEventSink),
        Arc::new(NeverCancelled),
        settings,
    ));
    let checkpoint = vec![
        ChatMessage::user_text("Watch the NPC"),
        ChatMessage::AssistantWithToolCalls {
            content: None,
            tool_calls: vec![app_application::models::chat::ToolCall {
                id: "committed-raw".into(),
                name: "apply_damage".into(),
                args: json!({"token_id":"npc","amount":3,"type":"fire"}),
            }],
        },
    ];
    let events = service
        .execute_recovered(command("recover-raw"), Some(checkpoint))
        .collect::<Vec<_>>()
        .await;
    assert!(
        matches!(events.last(), Some(Ok(AgentEvent::AgentDone { .. }))),
        "{events:?}"
    );
    assert_eq!(verifier.candidates.load(Ordering::SeqCst), 0);
    assert!(events.iter().any(|event|matches!(event,Ok(AgentEvent::ToolCallResult{is_error:false,result,..}) if result["hp"]==7)));
}
#[async_trait]
impl app_application::agent::intentions::PlayerIntentionVerifier for RawConsequenceFixtureVerifier {
    async fn assess(
        &self,
        check: app_application::agent::intentions::PlayerIntentionCheck,
    ) -> Result<
        app_application::agent::intentions::PlayerIntentionDecision,
        app_application::agent::intentions::PlayerIntentionError,
    > {
        if let Some((name, _)) = check.candidate {
            assert!(matches!(name.as_str(), "apply_damage" | "apply_healing"));
            self.candidates.fetch_add(1, Ordering::SeqCst);
        }
        Ok(
            app_application::agent::intentions::PlayerIntentionDecision {
                allowed: self.allowed,
                further_requested: false,
            },
        )
    }
}
#[tokio::test]
async fn raw_npc_effects_require_source_authorization_without_consuming_a_hero_action() {
    for (name, args) in [
        (
            "apply_damage",
            json!({"token_id":"npc","amount":3,"type":"fire"}),
        ),
        ("apply_healing", json!({"token_id":"npc","amount":3})),
    ] {
        for allowed in [false, true] {
            let dispatcher = agency_dispatcher();
            let verifier = Arc::new(RawConsequenceFixtureVerifier {
                allowed,
                candidates: AtomicUsize::new(0),
            });
            let mut settings = config();
            settings.player_verifier = Some(verifier.clone());
            let service = Arc::new(AgentTurnService::new(
                Arc::new(FakeProvider::scripted(vec![
                    agency_round(agency_tool("raw", name, args.clone())),
                    agency_final(),
                ])),
                Arc::new(FakeContext),
                Arc::new(FakeMessages::default()),
                dispatcher.clone(),
                Arc::new(NoopRuntimeCoordinator),
                Arc::new(NoopApplicationEventSink),
                Arc::new(NeverCancelled),
                settings,
            ));
            let events = service
                .execute(command("raw-source"))
                .collect::<Vec<_>>()
                .await;
            assert!(
                matches!(events.last(), Some(Ok(AgentEvent::AgentDone { .. }))),
                "{events:?}"
            );
            assert_eq!(verifier.candidates.load(Ordering::SeqCst), 1);
            assert_eq!(
                dispatcher.calls.load(Ordering::SeqCst),
                usize::from(allowed)
            );
            assert!(events.iter().any(|event|matches!(event,Ok(AgentEvent::ToolCallResult{is_error,..}) if *is_error==!allowed)));
        }
    }
}
