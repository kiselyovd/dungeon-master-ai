use super::*;
#[path = "room_clarification_runtime_cases.rs"]
mod clarification;
use app_application::{
    agent::{
        command_planning::*,
        intentions::*,
        tool_dispatch::NoopRuntimeCoordinator,
        turn::{AgentTurnCommand, AgentTurnService, NeverCancelled},
    },
    ports::events::NoopApplicationEventSink,
};
use std::collections::VecDeque;
#[derive(Debug)]
struct Permission;
#[derive(Debug)]
struct Denied {
    calls: AtomicUsize,
}
#[async_trait::async_trait]
impl PlayerIntentionVerifier for Denied {
    async fn assess(
        &self,
        _: PlayerIntentionCheck,
    ) -> Result<PlayerIntentionDecision, PlayerIntentionError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(PlayerIntentionDecision {
            allowed: false,
            further_requested: true,
        })
    }
}
#[async_trait::async_trait]
impl PlayerIntentionVerifier for Permission {
    async fn assess(
        &self,
        check: PlayerIntentionCheck,
    ) -> Result<PlayerIntentionDecision, PlayerIntentionError> {
        assert!(
            check.candidate.is_some(),
            "completion must use the authoritative journal, not another semantic guess"
        );
        Ok(PlayerIntentionDecision {
            allowed: true,
            further_requested: false,
        })
    }
}
struct Plan {
    manifest: HumanCommandManifest,
    calls: AtomicUsize,
}
#[async_trait::async_trait]
impl HumanCommandPlanner for Plan {
    async fn plan(
        &self,
        input: CommandPlanningInput,
    ) -> Result<HumanCommandManifest, CommandPlanningError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(input.scope, self.manifest.scope);
        assert_eq!(input.player_message, "Жду минуту и ещё жду минуту");
        Ok(self.manifest.clone())
    }
}
struct Script {
    rounds: std::sync::Mutex<VecDeque<Vec<ChatChunk>>>,
}
#[async_trait::async_trait]
impl LlmProvider for Script {
    async fn stream_chat(&self, request: ChatRequest) -> Result<ChunkStream, LlmError> {
        assert!(request
            .system_prompt
            .as_ref()
            .unwrap()
            .contains("durable command ledger"));
        let next = self
            .rounds
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected extra model round");
        Ok(futures::stream::iter(next.into_iter().map(Ok)).boxed())
    }
    fn name(&self) -> &'static str {
        "script"
    }
    fn active_model(&self) -> &str {
        "fixture"
    }
    fn capabilities_for_model(&self, _: &str) -> Capabilities {
        Default::default()
    }
}
fn action(call: &str) -> Vec<ChatChunk> {
    vec![
        ChatChunk::ToolCallStart {
            id: call.into(),
            name: "rest_action".into(),
        },
        ChatChunk::ToolCallArgsDelta {
            id: call.into(),
            args_fragment: json!({"kind":"wait","minutes":1}).to_string(),
        },
        ChatChunk::ToolCallDone { id: call.into() },
        ChatChunk::Done {
            reason: FinishReason::ToolUse,
        },
    ]
}
fn reply() -> Vec<ChatChunk> {
    vec![
        ChatChunk::TextDelta {
            text: "Вы подождали две минуты. Что делаете дальше?".into(),
        },
        ChatChunk::Done {
            reason: FinishReason::Stop,
        },
    ]
}

#[tokio::test]
async fn refused_prepared_call_recovers_without_reexecuting_or_rechecking_permission() {
    let f = fixture().await;
    let (ledger, ports, manifest) = journal(&f).await;
    ledger
        .save_verified_manifest(manifest.clone())
        .await
        .unwrap();
    ledger
        .prepare_after_permission(proposal(1, "first"))
        .await
        .unwrap();
    ledger
        .record_definitive_refusal("first", "rest_unsafe")
        .await
        .unwrap();
    let input = operation(&ports, "Жду минуту и ещё жду минуту").await;
    let checkpoint = vec![
        ChatMessage::user_text(input.request.player_message.clone()),
        ChatMessage::AssistantWithToolCalls {
            content: None,
            tool_calls: vec![app_application::models::chat::ToolCall {
                id: "first".into(),
                name: "rest_action".into(),
                args: json!({"kind":"wait","minutes":1}),
            }],
        },
    ];
    let denied = Arc::new(Denied {
        calls: AtomicUsize::new(0),
    });
    let planner = Arc::new(Plan {
        manifest,
        calls: AtomicUsize::new(0),
    });
    let revision = f
        .store
        .load_current(f.campaign)
        .await
        .unwrap()
        .campaign
        .revision;
    let events = engine_with_permission(
        ports,
        ledger.clone(),
        planner,
        vec![vec![ChatChunk::Done {
            reason: FinishReason::Error,
        }]],
        denied.clone(),
    )
    .execute_recovered(
        AgentTurnCommand {
            request_id: input.request_id.to_string(),
            request: input.request,
        },
        Some(checkpoint),
    )
    .collect::<Vec<_>>()
    .await;
    assert!(events.iter().any(|event|matches!(event,Ok(AgentEvent::ToolCallResult {is_error:true,result,..}) if result["error"]=="rest_unsafe")));
    assert_eq!(denied.calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        f.store
            .load_current(f.campaign)
            .await
            .unwrap()
            .campaign
            .revision,
        revision
    );
    assert_eq!(
        ledger.load_current().await.unwrap().unwrap().receipts[0].outcome,
        CommandAttemptOutcome::Refused {
            code: "rest_unsafe".into()
        }
    );
}
fn engine(
    ports: Arc<RoomAgentPorts>,
    ledger: RoomHumanCommandLedger,
    planner: Arc<dyn HumanCommandPlanner>,
    rounds: Vec<Vec<ChatChunk>>,
) -> Arc<AgentTurnService> {
    engine_with_permission(ports, ledger, planner, rounds, Arc::new(Permission))
}

#[tokio::test]
async fn tool_use_prose_is_not_saved_as_public_narration_for_verified_turns() {
    let f = fixture().await;
    let (ledger, ports, manifest) = journal(&f).await;
    let input = operation(&ports, "Жду минуту и ещё жду минуту").await;
    let mut first = action("first");
    first.insert(
        0,
        ChatChunk::TextDelta {
            text: "UNCHECKED_DRAFT".into(),
        },
    );
    let planner = Arc::new(Plan {
        manifest,
        calls: AtomicUsize::new(0),
    });
    let events = engine(
        ports,
        ledger,
        planner,
        vec![first, action("second"), reply()],
    )
    .execute(AgentTurnCommand {
        request_id: input.request_id.to_string(),
        request: input.request,
    })
    .collect::<Vec<_>>()
    .await;
    assert!(events.iter().all(Result::is_ok), "{events:?}");
    assert!(!events.iter().any(
        |event| matches!(event,Ok(AgentEvent::TextDelta {text}) if text.contains("UNCHECKED_DRAFT"))
    ));
    let subject = f
        .rooms
        .authenticate(f.host.room.room_id, f.host.member_id, &f.host.capability)
        .await
        .unwrap();
    let saved = f
        .store
        .room_agent_checkpoint(subject.lease(), input.request_id)
        .await
        .unwrap();
    assert!(saved.iter().all(|entry| !matches!(
        &entry.message,
        ChatMessage::AssistantWithToolCalls {
            content: Some(_),
            ..
        }
    )));
}
fn engine_with_permission(
    ports: Arc<RoomAgentPorts>,
    ledger: RoomHumanCommandLedger,
    planner: Arc<dyn HumanCommandPlanner>,
    rounds: Vec<Vec<ChatChunk>>,
    verifier: Arc<dyn PlayerIntentionVerifier>,
) -> Arc<AgentTurnService> {
    let mut settings = config();
    settings.player_verifier = Some(verifier);
    Arc::new(
        AgentTurnService::new(
            Arc::new(Script {
                rounds: std::sync::Mutex::new(rounds.into()),
            }),
            ports.clone(),
            ports.clone(),
            ports,
            Arc::new(NoopRuntimeCoordinator),
            Arc::new(NoopApplicationEventSink),
            Arc::new(NeverCancelled),
            settings,
        )
        .with_human_commands(Arc::new(ledger), planner),
    )
}

#[tokio::test]
async fn prepared_permission_survives_resume_and_does_not_create_a_policy_denial_receipt() {
    let f = fixture().await;
    let (ledger, ports, manifest) = journal(&f).await;
    ledger
        .save_verified_manifest(manifest.clone())
        .await
        .unwrap();
    ledger
        .prepare_after_permission(proposal(1, "first"))
        .await
        .unwrap();
    let input = operation(&ports, "Жду минуту и ещё жду минуту").await;
    let checkpoint = vec![
        ChatMessage::user_text(input.request.player_message.clone()),
        ChatMessage::AssistantWithToolCalls {
            content: None,
            tool_calls: vec![app_application::models::chat::ToolCall {
                id: "first".into(),
                name: "rest_action".into(),
                args: json!({"kind":"wait","minutes":1}),
            }],
        },
    ];
    for message in &checkpoint {
        ports.append(f.session.0, message.clone()).await.unwrap();
    }
    let denied = Arc::new(Denied {
        calls: AtomicUsize::new(0),
    });
    let planner = Arc::new(Plan {
        manifest: manifest.clone(),
        calls: AtomicUsize::new(0),
    });
    let before = f
        .store
        .load_current(f.campaign)
        .await
        .unwrap()
        .campaign
        .revision
        .0;
    let events = engine_with_permission(
        ports.clone(),
        ledger.clone(),
        planner.clone(),
        vec![vec![ChatChunk::Done {
            reason: FinishReason::Error,
        }]],
        denied.clone(),
    )
    .execute_recovered(
        AgentTurnCommand {
            request_id: input.request_id.to_string(),
            request: input.request.clone(),
        },
        Some(checkpoint),
    )
    .collect::<Vec<_>>()
    .await;
    assert!(
        events.last().unwrap().is_err(),
        "later provider interruption is expected"
    );
    assert_eq!(
        denied.calls.load(Ordering::SeqCst),
        0,
        "a stored exact permission is not reinterpreted"
    );
    let saved = ledger.load_current().await.unwrap().unwrap();
    assert_eq!(saved.receipts.len(), 1);
    assert!(matches!(
        saved.receipts[0].outcome,
        CommandAttemptOutcome::Performed { .. }
    ));
    assert_eq!(
        f.store
            .load_current(f.campaign)
            .await
            .unwrap()
            .campaign
            .revision
            .0,
        before + 1
    );
    let subject = f
        .rooms
        .authenticate(f.host.room.room_id, f.host.member_id, &f.host.capability)
        .await
        .unwrap();
    let checkpoint = f
        .store
        .room_agent_checkpoint(subject.lease(), manifest.scope.request_id)
        .await
        .unwrap()
        .into_iter()
        .map(|entry| entry.message)
        .collect();
    let events = engine(
        ports,
        ledger.clone(),
        planner,
        vec![action("second"), reply()],
    )
    .execute_recovered(
        AgentTurnCommand {
            request_id: input.request_id.to_string(),
            request: input.request,
        },
        Some(checkpoint),
    )
    .collect::<Vec<_>>()
    .await;
    assert!(events.iter().all(Result::is_ok), "{events:?}");
    assert_eq!(
        f.store
            .load_current(f.campaign)
            .await
            .unwrap()
            .campaign
            .revision
            .0,
        before + 2
    );
    assert_eq!(
        ledger.load_current().await.unwrap().unwrap().receipts.len(),
        2
    );
}
#[tokio::test]
async fn chat_runtime_saves_plan_and_finishes_both_commands_despite_false_completion_forecast() {
    let f = fixture().await;
    let (ledger, ports, manifest) = journal(&f).await;
    let planner = Arc::new(Plan {
        manifest: manifest.clone(),
        calls: AtomicUsize::new(0),
    });
    let input = operation(&ports, "Жду минуту и ещё жду минуту").await;
    let before = f
        .store
        .load_current(f.campaign)
        .await
        .unwrap()
        .campaign
        .revision
        .0;
    let events = engine(
        ports,
        ledger.clone(),
        planner.clone(),
        vec![action("first"), action("second"), reply()],
    )
    .execute(AgentTurnCommand {
        request_id: input.request_id.to_string(),
        request: input.request,
    })
    .collect::<Vec<_>>()
    .await;
    assert!(events.iter().all(Result::is_ok), "{events:?}");
    assert!(matches!(
        events.last(),
        Some(Ok(AgentEvent::AgentDone { .. }))
    ));
    assert_eq!(planner.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        f.store
            .load_current(f.campaign)
            .await
            .unwrap()
            .campaign
            .revision
            .0,
        before + 2
    );
    let saved = ledger.load_current().await.unwrap().unwrap();
    assert_eq!(saved.receipts.len(), 2);
    assert!(saved
        .manifest
        .progress(&saved.bindings, &saved.receipts)
        .unwrap()
        .iter()
        .all(|(_, status)| status.closed()));
}
#[tokio::test]
async fn chat_runtime_recovers_committed_effect_without_tool_result_and_never_replans() {
    let f = fixture().await;
    let (ledger, ports, manifest) = journal(&f).await;
    ledger
        .save_verified_manifest(manifest.clone())
        .await
        .unwrap();
    let first = ledger
        .prepare_after_permission(proposal(1, "first"))
        .await
        .unwrap();
    assert!(
        !ports
            .execute(ToolDispatchRequest {
                request_id: manifest.scope.request_id.to_string(),
                tool_call_id: "first".into(),
                campaign_id: f.campaign.0,
                session_id: f.session.0,
                command: first.signature.to_tool()
            })
            .await
            .unwrap()
            .is_error
    );
    let planner = Arc::new(Plan {
        manifest: manifest.clone(),
        calls: AtomicUsize::new(0),
    });
    let input = operation(&ports, "Жду минуту и ещё жду минуту").await;
    let checkpoint = vec![
        ChatMessage::user_text(input.request.player_message.clone()),
        ChatMessage::AssistantWithToolCalls {
            content: None,
            tool_calls: vec![app_application::models::chat::ToolCall {
                id: "first".into(),
                name: "rest_action".into(),
                args: json!({"kind":"wait","minutes":1}),
            }],
        },
    ];
    let before = f
        .store
        .load_current(f.campaign)
        .await
        .unwrap()
        .campaign
        .revision
        .0;
    let events = engine(
        ports,
        ledger.clone(),
        planner.clone(),
        vec![action("second"), reply()],
    )
    .execute_recovered(
        AgentTurnCommand {
            request_id: input.request_id.to_string(),
            request: input.request,
        },
        Some(checkpoint),
    )
    .collect::<Vec<_>>()
    .await;
    assert!(events.iter().all(Result::is_ok), "{events:?}");
    assert!(matches!(
        events.last(),
        Some(Ok(AgentEvent::AgentDone { .. }))
    ));
    assert_eq!(planner.calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        f.store
            .load_current(f.campaign)
            .await
            .unwrap()
            .campaign
            .revision
            .0,
        before + 1
    );
    assert_eq!(
        ledger.load_current().await.unwrap().unwrap().receipts.len(),
        2
    );
}
