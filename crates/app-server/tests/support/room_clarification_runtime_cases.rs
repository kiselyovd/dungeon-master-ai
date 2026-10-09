use super::*;
use app_application::agent::{
    command_planning::*,
    operations::{AgentOperationDriver, DurableAgentTurns, TurnCancellation},
};
use tokio::sync::mpsc;
struct Clarifying {
    manifest: HumanCommandManifest,
    plans: AtomicUsize,
    resolutions: AtomicUsize,
}
#[async_trait::async_trait]
impl HumanCommandPlanner for Clarifying {
    async fn plan(
        &self,
        _: CommandPlanningInput,
    ) -> Result<HumanCommandManifest, CommandPlanningError> {
        self.plans.fetch_add(1, Ordering::SeqCst);
        Ok(self.manifest.clone())
    }
    async fn resolve(
        &self,
        input: CommandResolutionInput,
    ) -> Result<HumanCommandResolution, CommandPlanningError> {
        self.resolutions.fetch_add(1, Ordering::SeqCst);
        assert_eq!(input.command_id, 1);
        if input
            .clarifications
            .last()
            .and_then(|entry| entry.reply.as_deref())
            == Some("Одну минуту")
        {
            Ok(HumanCommandResolution::Resolved(HumanAction::Rest(
                RestIntent::Wait { minutes: 1 },
            )))
        } else {
            Ok(HumanCommandResolution::NeedsInput(
                "Сколько минут ждать?".into(),
            ))
        }
    }
}
#[tokio::test]
async fn question_is_delivered_without_done_then_authenticated_reply_resolves_same_unfinished_command(
) {
    let f = fixture().await;
    let (ledger, ports, mut manifest) = journal(&f).await;
    manifest.commands[0].goal = HumanCommandGoal::NeedsResolution {
        description: "Длительность ожидания".into(),
    };
    let planner = Arc::new(Clarifying {
        manifest: manifest.clone(),
        plans: AtomicUsize::new(0),
        resolutions: AtomicUsize::new(0),
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
    let events = engine(ports.clone(), ledger.clone(), planner.clone(), vec![])
        .execute(AgentTurnCommand {
            request_id: input.request_id.to_string(),
            request: input.request.clone(),
        })
        .collect::<Vec<_>>()
        .await;
    assert!(events.iter().all(Result::is_ok), "{events:?}");
    let Some(Ok(AgentEvent::AgentNeedsInput {
        clarification_id, ..
    })) = events.last()
    else {
        panic!("{events:?}");
    };
    assert!(events.iter().any(
        |event| matches!(event,Ok(AgentEvent::TextDelta {text}) if text=="Сколько минут ждать?")
    ));
    assert!(!events
        .iter()
        .any(|event| matches!(event, Ok(AgentEvent::AgentDone { .. }))));
    assert_eq!(
        f.store
            .load_current(f.campaign)
            .await
            .unwrap()
            .campaign
            .revision
            .0,
        before
    );
    let saved = ledger.load_current().await.unwrap().unwrap();
    assert_eq!(saved.resolution_command().unwrap(), Some(1));
    assert!(saved.bindings.is_empty());
    ledger
        .submit_clarification_reply(*clarification_id, "Одну минуту".into())
        .await
        .unwrap();
    let subject = f
        .rooms
        .authenticate(f.host.room.room_id, f.host.member_id, &f.host.capability)
        .await
        .unwrap();
    let checkpoint = f
        .store
        .room_agent_checkpoint(subject.lease(), input.request_id)
        .await
        .unwrap()
        .into_iter()
        .map(|entry| entry.message)
        .collect();
    let events = engine(
        ports,
        ledger.clone(),
        planner.clone(),
        vec![action("first"), action("second"), reply()],
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
    assert_eq!(planner.plans.load(Ordering::SeqCst), 1);
    assert_eq!(planner.resolutions.load(Ordering::SeqCst), 2);
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
struct QuestionDriver;

#[tokio::test]
async fn lost_question_ack_redelivers_same_id_without_another_chat_message_or_model_call() {
    let f = fixture().await;
    let (ledger, ports, mut manifest) = journal(&f).await;
    manifest.commands[0].goal = HumanCommandGoal::NeedsResolution {
        description: "Длительность ожидания".into(),
    };
    let planner = Arc::new(Clarifying {
        manifest,
        plans: AtomicUsize::new(0),
        resolutions: AtomicUsize::new(0),
    });
    let input = operation(&ports, "Жду минуту и ещё жду минуту").await;
    let first = engine(ports.clone(), ledger.clone(), planner.clone(), vec![])
        .execute(AgentTurnCommand {
            request_id: input.request_id.to_string(),
            request: input.request.clone(),
        })
        .collect::<Vec<_>>()
        .await;
    let Some(Ok(AgentEvent::AgentNeedsInput {
        clarification_id, ..
    })) = first.last()
    else {
        panic!("{first:?}");
    };
    let subject = f
        .rooms
        .authenticate(f.host.room.room_id, f.host.member_id, &f.host.capability)
        .await
        .unwrap();
    let checkpoint = f
        .store
        .room_agent_checkpoint(subject.lease(), input.request_id)
        .await
        .unwrap()
        .into_iter()
        .map(|entry| entry.message)
        .collect();
    let again = engine(ports, ledger, planner.clone(), vec![])
        .execute_recovered(
            AgentTurnCommand {
                request_id: input.request_id.to_string(),
                request: input.request,
            },
            Some(checkpoint),
        )
        .collect::<Vec<_>>()
        .await;
    assert!(
        matches!(again.last(),Some(Ok(AgentEvent::AgentNeedsInput {clarification_id:id,..})) if id==clarification_id)
    );
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM room_agent_messages WHERE turn_id=?1 AND message_id=?2",
    )
    .bind(input.request_id.to_string())
    .bind(clarification_id.to_string())
    .fetch_one(f.store.pool())
    .await
    .unwrap();
    assert_eq!(count, 1);
    assert_eq!(planner.resolutions.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn saved_final_after_effects_is_replayed_without_provider_or_duplicate_final() {
    let f = fixture().await;
    let (ledger, ports, manifest) = journal(&f).await;
    ledger
        .save_verified_manifest(manifest.clone())
        .await
        .unwrap();
    for (id, call) in [(1, "first"), (2, "second")] {
        let binding = ledger
            .prepare_after_permission(proposal(id, call))
            .await
            .unwrap();
        ports
            .execute(ToolDispatchRequest {
                request_id: manifest.scope.request_id.to_string(),
                tool_call_id: call.into(),
                campaign_id: f.campaign.0,
                session_id: f.session.0,
                command: binding.signature.to_tool(),
            })
            .await
            .unwrap();
    }
    let saved = ledger
        .record_final_narration("Ответ уже подтверждён.".into())
        .await
        .unwrap();
    let input = operation(&ports, "Жду минуту и ещё жду минуту").await;
    let subject = f
        .rooms
        .authenticate(f.host.room.room_id, f.host.member_id, &f.host.capability)
        .await
        .unwrap();
    let checkpoint = f
        .store
        .room_agent_checkpoint(subject.lease(), input.request_id)
        .await
        .unwrap()
        .into_iter()
        .map(|entry| entry.message)
        .collect();
    let revision = f
        .store
        .load_current(f.campaign)
        .await
        .unwrap()
        .campaign
        .revision;
    let planner = Arc::new(Plan {
        manifest,
        calls: AtomicUsize::new(0),
    });
    let events = engine(ports, ledger.clone(), planner.clone(), vec![])
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
        Some(Ok(AgentEvent::AgentDone { total_rounds: 0 }))
    ));
    assert!(events
        .iter()
        .any(|event| matches!(event,Ok(AgentEvent::TextDelta {text}) if text==&saved.text)));
    assert_eq!(planner.calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        f.store
            .load_current(f.campaign)
            .await
            .unwrap()
            .campaign
            .revision,
        revision
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE id=?1")
        .bind(saved.id.to_string())
        .fetch_one(f.store.pool())
        .await
        .unwrap();
    assert_eq!(count, 1);
}
#[async_trait::async_trait]
impl AgentOperationDriver for QuestionDriver {
    async fn run(
        &self,
        _: AgentOperation,
        events: mpsc::Sender<AgentEvent>,
        _: Arc<TurnCancellation>,
    ) -> Result<(), &'static str> {
        events
            .send(AgentEvent::TextDelta {
                text: "Кого атаковать?".into(),
            })
            .await
            .map_err(|_| "closed")?;
        events
            .send(AgentEvent::AgentNeedsInput {
                clarification_id: Uuid::new_v4(),
                command_id: 1,
                total_rounds: 1,
            })
            .await
            .map_err(|_| "closed")?;
        Ok(())
    }
}
#[tokio::test]
async fn clarification_terminal_is_sent_only_after_incomplete_operation_status_commits() {
    let f = fixture().await;
    let turn = Uuid::new_v4();
    let ports = f.ports(&f.host, turn).await;
    let input = operation(&ports, "Атакую врага").await;
    let mut receiver = DurableAgentTurns::new(ports, Arc::new(QuestionDriver), Uuid::new_v4())
        .execute(input)
        .await
        .unwrap();
    let mut saw_question = false;
    while let Some(event) = receiver.recv().await {
        assert!(!matches!(event, AgentEvent::AgentDone { .. }));
        if matches!(event, AgentEvent::AgentNeedsInput { .. }) {
            let status: String =
                sqlx::query_scalar("SELECT status FROM room_dm_turns WHERE turn_id=?1")
                    .bind(turn.to_string())
                    .fetch_one(f.store.pool())
                    .await
                    .unwrap();
            assert_eq!(status, "interrupted");
            saw_question = true;
        }
    }
    assert!(saw_question);
}
