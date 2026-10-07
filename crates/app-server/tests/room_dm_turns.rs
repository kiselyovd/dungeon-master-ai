use adapter_secrets::InMemorySecretsStore;
use adapter_sqlite::{pool, SqliteStore};
use app_application::{
    agent::{
        operations::{AgentOperation, AgentOperationRepository, OperationClaim},
        tools::{AgentToolCatalog, ToolAvailability},
        turn::AgentTurnConfig,
    },
    campaigns::{CreateCampaign, CreateCampaignCommand},
    models::{
        agent::AgentEvent,
        campaign_state::CommitCampaignCharacter,
        chat::{Capabilities, ChatChunk, ChatMessage, ChatRequest, FinishReason},
        identity::{CampaignId, SessionId},
        room::CreateLocalRoom,
        room_character::{ApproveRoomCharacter, SubmitRoomCharacter},
        session::CampaignSeed,
    },
    ports::{
        llm::{ChunkStream, LlmError, LlmProvider},
        repositories::{
            CampaignStateRepository, MessageRepository, PartyRepository, RoomRepository,
        },
    },
    rooms::{
        agent::RoomAgentPorts, dm_turns::RoomDmRunner, JoinRoom, RoomGrant, RoomSecret,
        RoomService, SystemRoomClock,
    },
};
use futures::StreamExt;
use serde_json::json;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::sync::{Mutex, Notify};
use uuid::Uuid;
struct Fixture {
    store: Arc<SqliteStore>,
    rooms: Arc<RoomService>,
    host: RoomGrant,
    guest: RoomGrant,
    campaign: CampaignId,
    session: SessionId,
}
impl Fixture {
    async fn ports(&self, member: &RoomGrant, turn: Uuid) -> Arc<RoomAgentPorts> {
        Arc::new(RoomAgentPorts::new(
            self.rooms.clone(),
            self.rooms
                .authenticate(member.room.room_id, member.member_id, &member.capability)
                .await
                .unwrap(),
            turn,
        ))
    }
}
async fn fixture() -> Fixture {
    let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&db).await.unwrap();
    let store = Arc::new(SqliteStore::new(db));
    let campaign = CreateCampaign::new(store.clone())
        .execute(CreateCampaignCommand {
            title: "DM lane".into(),
            premise: "Road".into(),
            seed: CampaignSeed::Collaborative {
                answers: Default::default(),
            },
        })
        .await
        .unwrap();
    let sheet = json!({"name":"Mira","heroClass":"fighter","level":1,"hp":12,"hpMax":12,"ac":14,"abilities":{"str":15,"dex":14,"con":13,"int":12,"wis":10,"cha":8},"inventory":[]});
    let current = store
        .commit_character(CommitCampaignCharacter {
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            character_id: Uuid::new_v4(),
            expected_revision: campaign.revision,
            causation_id: "hero".into(),
            character: sheet.clone(),
        })
        .await
        .unwrap();
    let rooms = Arc::new(RoomService::new(
        store.clone(),
        store.clone(),
        Arc::new(InMemorySecretsStore::default()),
        Arc::new(SystemRoomClock),
    ));
    let created = rooms
        .create_local_room(CreateLocalRoom {
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            expected_revision: current.campaign.revision,
            causation_id: "room".into(),
        })
        .await
        .unwrap();
    let guest = rooms
        .join(JoinRoom {
            room_id: created.grant.room.room_id,
            join_id: Uuid::new_v4(),
            display_name: "Guest".into(),
            invitation: RoomSecret::from_input(created.invitation.expose().into()).unwrap(),
            resume_proof: RoomSecret::generate(),
        })
        .await
        .unwrap();
    let guest_subject = rooms
        .authenticate(guest.room.room_id, guest.member_id, &guest.capability)
        .await
        .unwrap();
    let host_subject = rooms
        .authenticate(
            created.grant.room.room_id,
            created.grant.member_id,
            &created.grant.capability,
        )
        .await
        .unwrap();
    let character = Uuid::new_v4();
    let proposal = Uuid::new_v4();
    let mut sheet = sheet;
    sheet["characterId"] = json!(character);
    sheet["name"] = json!("Tara");
    rooms
        .submit_character(
            &guest_subject,
            SubmitRoomCharacter {
                proposal_id: proposal,
                character_id: character,
                character: sheet,
            },
        )
        .await
        .unwrap();
    rooms
        .approve_character(
            &host_subject,
            ApproveRoomCharacter {
                proposal_id: proposal,
                expected_revision: store.party(campaign.id).await.unwrap().unwrap().revision,
                causation_id: "accept".into(),
            },
        )
        .await
        .unwrap();
    Fixture {
        store,
        rooms,
        host: created.grant,
        guest,
        campaign: campaign.id,
        session: campaign.last_session_id,
    }
}
fn config() -> AgentTurnConfig {
    let available = ToolAvailability::default();
    AgentTurnConfig {
        model: "fixture".into(),
        system_prompt: "DM".into(),
        temperature: 0.0,
        max_rounds: 4,
        embedding_model: "unused".into(),
        reasoning_enabled: false,
        reasoning_budget: app_application::models::chat::ReasoningSpec::Low,
        tool_availability: available,
        tools: AgentToolCatalog {
            exploration: app_server::agent::tools::tools_for_phase(available, false),
            combat: app_server::agent::tools::tools_for_phase(available, true),
        },
    }
}
#[derive(Default)]
struct Provider {
    calls: AtomicUsize,
    hold: bool,
    started: Notify,
}
#[async_trait::async_trait]
impl LlmProvider for Provider {
    async fn stream_chat(&self, request: ChatRequest) -> Result<ChunkStream, LlmError> {
        assert!(request
            .system_prompt
            .as_ref()
            .unwrap()
            .contains("You are the Dungeon Master"));
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.started.notify_one();
        if self.hold {
            std::future::pending::<()>().await;
        }
        Ok(futures::stream::iter(vec![
            Ok(ChatChunk::TextDelta {
                text: "Перед вами дорога. Что вы делаете?".into(),
            }),
            Ok(ChatChunk::Done {
                reason: FinishReason::Stop,
            }),
        ])
        .boxed())
    }
    fn name(&self) -> &'static str {
        "fixture"
    }
    fn active_model(&self) -> &str {
        "fixture"
    }
    fn capabilities_for_model(&self, _: &str) -> Capabilities {
        Default::default()
    }
}
async fn collect(mut stream: futures::stream::BoxStream<'static, AgentEvent>) -> Vec<AgentEvent> {
    let mut events = vec![];
    while let Some(event) = tokio::time::timeout(std::time::Duration::from_secs(3), stream.next())
        .await
        .unwrap()
    {
        events.push(event);
    }
    events
}
fn runner(
    ports: Arc<RoomAgentPorts>,
    provider: Arc<Provider>,
    lane: Arc<Mutex<()>>,
    owner: Uuid,
) -> RoomDmRunner {
    RoomDmRunner::new(ports, provider, config(), lane, owner)
}
async fn operation(ports: &RoomAgentPorts, text: &str) -> AgentOperation {
    let command = ports.command(text.into()).await.unwrap();
    AgentOperation {
        request_id: Uuid::parse_str(&command.request_id).unwrap(),
        request: command.request,
        model: Some("fixture".into()),
    }
}
#[tokio::test]
async fn completed_turn_replays_only_for_its_owner_without_model_or_message_duplication() {
    let f = fixture().await;
    let turn = Uuid::new_v4();
    let ports = f.ports(&f.guest, turn).await;
    let provider = Arc::new(Provider::default());
    let lane = Arc::new(Mutex::new(()));
    let owner = Uuid::new_v4();
    let first = collect(
        runner(ports.clone(), provider.clone(), lane.clone(), owner)
            .execute("Осматриваюсь".into(), false)
            .await
            .unwrap(),
    )
    .await;
    assert!(matches!(first.last(), Some(AgentEvent::AgentDone { .. })));
    let messages = f.store.list(f.session.0).await.unwrap();
    let reconnect = f.ports(&f.guest, turn).await;
    let replay = collect(
        runner(
            reconnect.clone(),
            provider.clone(),
            lane.clone(),
            Uuid::new_v4(),
        )
        .execute("Осматриваюсь".into(), false)
        .await
        .unwrap(),
    )
    .await;
    assert_eq!(first, replay);
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    assert_eq!(messages, f.store.list(f.session.0).await.unwrap());
    assert!(runner(reconnect, provider.clone(), lane.clone(), owner)
        .execute("Другой ход".into(), false)
        .await
        .is_err());
    let host = f.ports(&f.host, turn).await;
    assert!(f
        .store
        .room_dm_request(
            f.rooms
                .authenticate(f.host.room.room_id, f.host.member_id, &f.host.capability)
                .await
                .unwrap()
                .lease(),
            turn
        )
        .await
        .unwrap()
        .is_none());
    collect(
        runner(host, provider.clone(), lane, owner)
            .execute("Слушаю".into(), false)
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
}
#[tokio::test]
async fn two_members_cannot_run_a_dm_concurrently_and_disconnect_requires_explicit_resume() {
    let f = fixture().await;
    let owner = Uuid::new_v4();
    let turn = Uuid::new_v4();
    let ports = f.ports(&f.guest, turn).await;
    let waiting = Arc::new(Provider {
        hold: true,
        ..Default::default()
    });
    let lane = Arc::new(Mutex::new(()));
    let stream = runner(ports.clone(), waiting.clone(), lane.clone(), owner)
        .execute("Жду ответа".into(), false)
        .await
        .unwrap();
    tokio::time::timeout(
        std::time::Duration::from_secs(3),
        waiting.started.notified(),
    )
    .await
    .unwrap();
    let host = f.ports(&f.host, Uuid::new_v4()).await;
    let ready = Arc::new(Provider::default());
    // Independent in-memory lane still cannot bypass the durable campaign claim.
    assert!(
        runner(host.clone(), ready.clone(), Arc::new(Mutex::new(())), owner)
            .execute("Мой ход".into(), false)
            .await
            .is_err()
    );
    let subject = f
        .rooms
        .authenticate(f.guest.room.room_id, f.guest.member_id, &f.guest.capability)
        .await
        .unwrap();
    assert!(
        f.rooms
            .initial_updates(&subject, None)
            .await
            .unwrap()
            .state
            .agent_busy
    );
    drop(stream);
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let status: String =
                sqlx::query_scalar("SELECT status FROM room_dm_turns WHERE turn_id=?1")
                    .bind(turn.to_string())
                    .fetch_one(f.store.pool())
                    .await
                    .unwrap();
            if status == "interrupted" {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(runner(ports.clone(), ready.clone(), lane.clone(), owner)
        .execute("Жду ответа".into(), false)
        .await
        .is_err());
    assert!(runner(host, ready.clone(), Arc::new(Mutex::new(())), owner)
        .execute("Мой ход".into(), false)
        .await
        .is_err());
    let resumed = collect(
        runner(ports.clone(), ready.clone(), lane, owner)
            .execute("Жду ответа".into(), true)
            .await
            .unwrap(),
    )
    .await;
    assert!(matches!(resumed.last(), Some(AgentEvent::AgentDone { .. })));
    assert_eq!(ports.checkpoint().await.unwrap().len(), 2);
    assert!(
        !f.rooms
            .initial_updates(&subject, None)
            .await
            .unwrap()
            .state
            .agent_busy
    );
}
#[tokio::test]
async fn restart_needs_exclusive_authority_and_cannot_steal_another_members_turn() {
    let f = fixture().await;
    let turn = Uuid::new_v4();
    let ports = f.ports(&f.guest, turn).await;
    let input = operation(&ports, "Осматриваюсь").await;
    let old_owner = Uuid::new_v4();
    let new_owner = Uuid::new_v4();
    assert!(matches!(
        ports.claim(input.clone(), old_owner).await.unwrap(),
        OperationClaim::Started
    ));
    assert!(matches!(
        ports.resume(input.clone(), new_owner).await.unwrap(),
        OperationClaim::Running
    ));
    let host = f.ports(&f.host, turn).await;
    assert!(host
        .resume(operation(&host, "Забираю чужой ход").await, new_owner)
        .await
        .is_err());
    // Test simulates the bootstrap call after the sole-process authority lock.
    assert_eq!(
        f.store
            .recover_after_exclusive_authority(new_owner)
            .await
            .unwrap(),
        1
    );
    assert!(runner(
        ports.clone(),
        Arc::new(Provider::default()),
        Arc::new(Mutex::new(())),
        new_owner
    )
    .execute("Изменённый запрос".into(), true)
    .await
    .is_err());
    let provider = Arc::new(Provider::default());
    let result = collect(
        runner(ports, provider.clone(), Arc::new(Mutex::new(())), new_owner)
            .execute("Осматриваюсь".into(), true)
            .await
            .unwrap(),
    )
    .await;
    assert!(matches!(result.last(), Some(AgentEvent::AgentDone { .. })));
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn delivery_sequence_owner_and_completion_are_transactionally_guarded() {
    let f = fixture().await;
    let ports = f.ports(&f.guest, Uuid::new_v4()).await;
    let input = operation(&ports, "Слушаю").await;
    let owner = Uuid::new_v4();
    ports.claim(input.clone(), owner).await.unwrap();
    assert!(ports
        .append_event(
            input.request_id,
            Uuid::new_v4(),
            0,
            AgentEvent::AgentDone { total_rounds: 1 }
        )
        .await
        .is_err());
    assert!(ports
        .append_event(
            input.request_id,
            owner,
            1,
            AgentEvent::AgentDone { total_rounds: 1 }
        )
        .await
        .is_err());
    assert!(ports
        .append_event(
            input.request_id,
            owner,
            0,
            AgentEvent::ReasoningText {
                text: "secret".into()
            }
        )
        .await
        .is_err());
    assert!(ports.finish(input.request_id, owner, true).await.is_err());
    ports
        .append_event(
            input.request_id,
            owner,
            0,
            AgentEvent::TextDelta {
                text: "Ответ".into(),
            },
        )
        .await
        .unwrap();
    ports
        .append(f.session.0, ChatMessage::user_text("Слушаю"))
        .await
        .unwrap();
    ports
        .append(
            f.session.0,
            ChatMessage::Assistant {
                content: "Ответ".into(),
            },
        )
        .await
        .unwrap();
    ports
        .append_event(
            input.request_id,
            owner,
            1,
            AgentEvent::AgentDone { total_rounds: 1 },
        )
        .await
        .unwrap();
    ports.finish(input.request_id, owner, true).await.unwrap();
    assert!(ports
        .append_event(
            input.request_id,
            owner,
            2,
            AgentEvent::TextDelta {
                text: "late".into()
            }
        )
        .await
        .is_err());
}

#[tokio::test]
async fn snapshot_restore_invalidates_even_a_claim_without_a_transcript() {
    use app_application::{
        campaigns::{CreateSnapshot, CreateSnapshotCommand},
        models::snapshot::RestoreSnapshotRecord,
        ports::repositories::SnapshotRepository,
    };
    let f = fixture().await;
    let turn = Uuid::new_v4();
    let ports = f.ports(&f.guest, turn).await;
    let input = operation(&ports, "Слушаю").await;
    ports.claim(input, Uuid::new_v4()).await.unwrap();
    let current = f.store.load_current(f.campaign).await.unwrap();
    let saved = CreateSnapshot::new(f.store.clone())
        .execute(CreateSnapshotCommand {
            id: Uuid::new_v4(),
            campaign_id: f.campaign,
            session_id: f.session,
            expected_revision: current.campaign.revision,
            title: "Before DM".into(),
            kind: "manual".into(),
            summary: "".into(),
            tag: "".into(),
            causation_id: "save".into(),
            character: current.projection.character,
        })
        .await
        .unwrap();
    let current = f.store.load_current(f.campaign).await.unwrap();
    f.store
        .restore_snapshot(RestoreSnapshotRecord {
            save_id: saved.save_id,
            campaign_id: f.campaign,
            session_id: f.session,
            expected_revision: current.campaign.revision,
            causation_id: "restore".into(),
        })
        .await
        .unwrap();
    assert!(ports.checkpoint().await.is_err());
    assert!(ports.command("Слушаю".into()).await.is_err());
    let provider = Arc::new(Provider::default());
    collect(
        runner(
            f.ports(&f.host, Uuid::new_v4()).await,
            provider,
            Arc::new(Mutex::new(())),
            Uuid::new_v4(),
        )
        .execute("Начинаем заново".into(), false)
        .await
        .unwrap(),
    )
    .await;
}
#[tokio::test]
async fn revocation_cancels_waiting_model_and_releases_the_shared_dm_lane() {
    let f = fixture().await;
    let turn = Uuid::new_v4();
    let ports = f.ports(&f.guest, turn).await;
    let waiting = Arc::new(Provider {
        hold: true,
        ..Default::default()
    });
    let lane = Arc::new(Mutex::new(()));
    let stream = runner(ports.clone(), waiting.clone(), lane.clone(), Uuid::new_v4())
        .execute("Жду".into(), false)
        .await
        .unwrap();
    tokio::time::timeout(
        std::time::Duration::from_secs(3),
        waiting.started.notified(),
    )
    .await
    .unwrap();
    let host = f
        .rooms
        .authenticate(f.host.room.room_id, f.host.member_id, &f.host.capability)
        .await
        .unwrap();
    f.rooms.revoke(&host, f.guest.member_id).await.unwrap();
    assert!(collect(stream).await.is_empty());
    assert!(runner(
        ports,
        Arc::new(Provider::default()),
        lane.clone(),
        Uuid::new_v4()
    )
    .execute("Жду".into(), true)
    .await
    .is_err());
    let status: String = sqlx::query_scalar("SELECT status FROM room_dm_turns WHERE turn_id=?1")
        .bind(turn.to_string())
        .fetch_one(f.store.pool())
        .await
        .unwrap();
    assert_eq!(status, "superseded");
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            if lane.try_lock().is_ok() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    collect(
        runner(
            f.ports(&f.host, Uuid::new_v4()).await,
            Arc::new(Provider::default()),
            lane,
            Uuid::new_v4(),
        )
        .execute("Продолжаем".into(), false)
        .await
        .unwrap(),
    )
    .await;
}
#[tokio::test]
async fn resume_repairs_confirmed_tool_delivery_and_uses_the_original_request_history() {
    use app_application::{
        agent::{
            commands::AgentToolCommand,
            tool_dispatch::{ToolDispatchRequest, ToolDispatcher},
        },
        models::{
            chat::{ToolCall, ToolResult},
            rest::RestIntent,
        },
    };
    let f = fixture().await;
    let turn = Uuid::new_v4();
    let ports = f.ports(&f.guest, turn).await;
    let input = operation(&ports, "Долгий отдых").await;
    let owner = Uuid::new_v4();
    ports.claim(input.clone(), owner).await.unwrap();
    ports
        .append(f.session.0, ChatMessage::user_text("Долгий отдых"))
        .await
        .unwrap();
    ports
        .append(
            f.session.0,
            ChatMessage::AssistantWithToolCalls {
                content: None,
                tool_calls: vec![ToolCall {
                    id: "rest-intent".into(),
                    name: "rest_action".into(),
                    args: json!({"kind":"long"}),
                }],
            },
        )
        .await
        .unwrap();
    let result = ports
        .execute(ToolDispatchRequest {
            request_id: turn.to_string(),
            tool_call_id: "rest-intent".into(),
            campaign_id: f.campaign.0,
            session_id: f.session.0,
            command: AgentToolCommand::RestAction(RestIntent::Long),
        })
        .await
        .unwrap();
    assert!(result.is_error);
    ports
        .append(
            f.session.0,
            ChatMessage::ToolResult(ToolResult {
                tool_call_id: "rest-intent".into(),
                content: serde_json::to_string(&result.result).unwrap(),
                is_error: result.is_error,
            }),
        )
        .await
        .unwrap();
    ports.finish(turn, owner, false).await.unwrap();
    f.store
        .append(
            f.session.0,
            ChatMessage::user_text("Позднее сообщение другого игрока"),
        )
        .await
        .unwrap();
    assert_eq!(
        ports.command("Долгий отдых".into()).await.unwrap().request,
        input.request
    );
    let provider = Arc::new(Provider::default());
    let events = collect(
        runner(ports, provider.clone(), Arc::new(Mutex::new(())), owner)
            .execute("Долгий отдых".into(), true)
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, AgentEvent::ToolCallResult { .. }))
            .count(),
        1
    );
    assert!(events.iter().any(|event|matches!(event,AgentEvent::ToolCallResult {id,is_error:true,..} if id==&format!("{turn}:rest-intent"))));
    assert!(matches!(events.last(), Some(AgentEvent::AgentDone { .. })));
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn real_room_http_uses_host_provider_and_rejects_forged_authority() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .with_test_writer()
        .try_init();
    let f = fixture().await;
    let provider = Arc::new(Provider::default());
    let state =
        app_server::AppState::new(provider.clone(), "fixture".into(), f.store.pool().clone());
    let mut configured = state.agent_config();
    configured.model = "fixture".into();
    state.set_agent_config(configured);
    let dm = app_server::http_services::room_dm_http_service(state, f.rooms.clone());
    let router = adapter_http::rooms::router_with_dm(f.rooms.clone(), dm);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let client = reqwest::Client::new();
    let url = format!("http://{address}/rooms/{}/dm/turn", f.guest.room.room_id);
    let turn = Uuid::new_v4();
    let input = json!({"request_id":turn,"player_message":"Осматриваюсь"});
    assert_eq!(
        client
            .post(&url)
            .json(&input)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    for key in [
        "campaign_id",
        "session_id",
        "actor_id",
        "model",
        "history",
        "board",
    ] {
        let mut forged = input.clone();
        forged[key] = json!("forged");
        assert_eq!(
            client
                .post(&url)
                .bearer_auth(f.guest.capability.expose())
                .header("x-room-member-id", f.guest.member_id.to_string())
                .json(&forged)
                .send()
                .await
                .unwrap()
                .status(),
            422
        );
    }
    let request = || {
        client
            .post(&url)
            .bearer_auth(f.guest.capability.expose())
            .header("x-room-member-id", f.guest.member_id.to_string())
            .json(&input)
    };
    let first = request().send().await.unwrap();
    assert_eq!(first.status(), 200);
    assert_eq!(first.headers()["cache-control"], "no-store");
    let first = first.text().await.unwrap();
    assert!(first.contains("event: agent_done"), "SSE body: {first}");
    assert!(first.contains("Перед вами дорога"));
    assert!(!first.contains("reasoning_text"));
    let replay = request().send().await.unwrap();
    assert_eq!(replay.status(), 200);
    assert_eq!(first, replay.text().await.unwrap());
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        client
            .post(format!("http://{address}/api/settings"))
            .json(&json!({}))
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    server.abort();
}

struct IncompleteProvider {
    tool: bool,
    whitespace: bool,
}
#[async_trait::async_trait]
impl LlmProvider for IncompleteProvider {
    async fn stream_chat(&self, _: ChatRequest) -> Result<ChunkStream, LlmError> {
        let chunks = if self.tool {
            vec![
                ChatChunk::ToolCallStart {
                    id: "limit-rest".into(),
                    name: "rest_action".into(),
                },
                ChatChunk::ToolCallArgsDelta {
                    id: "limit-rest".into(),
                    args_fragment: json!({"kind":"long"}).to_string(),
                },
                ChatChunk::ToolCallDone {
                    id: "limit-rest".into(),
                },
                ChatChunk::Done {
                    reason: FinishReason::ToolUse,
                },
            ]
        } else if self.whitespace {
            vec![
                ChatChunk::TextDelta {
                    text: " \n ".into(),
                },
                ChatChunk::Done {
                    reason: FinishReason::Stop,
                },
            ]
        } else {
            vec![ChatChunk::Done {
                reason: FinishReason::Stop,
            }]
        };
        Ok(futures::stream::iter(chunks.into_iter().map(Ok)).boxed())
    }
    fn name(&self) -> &'static str {
        "fixture"
    }
    fn active_model(&self) -> &str {
        "fixture"
    }
    fn capabilities_for_model(&self, _: &str) -> Capabilities {
        Capabilities {
            tool_calls: true,
            ..Default::default()
        }
    }
}
#[tokio::test]
async fn round_limit_and_empty_final_reply_interrupt_instead_of_sticking_running() {
    for (tool, whitespace) in [(true, false), (false, false), (false, true)] {
        let f = fixture().await;
        let turn = Uuid::new_v4();
        let ports = f.ports(&f.guest, turn).await;
        let owner = Uuid::new_v4();
        let lane = Arc::new(Mutex::new(()));
        let mut limited = config();
        limited.max_rounds = 1;
        let events = collect(
            RoomDmRunner::new(
                ports.clone(),
                Arc::new(IncompleteProvider { tool, whitespace }),
                limited,
                lane.clone(),
                owner,
            )
            .execute("Действую".into(), false)
            .await
            .unwrap(),
        )
        .await;
        assert!(!events
            .iter()
            .any(|event| matches!(event, AgentEvent::AgentDone { .. })));
        let status: String =
            sqlx::query_scalar("SELECT status FROM room_dm_turns WHERE turn_id=?1")
                .bind(turn.to_string())
                .fetch_one(f.store.pool())
                .await
                .unwrap();
        assert_eq!(status, "interrupted");
        let resumed = collect(
            runner(ports, Arc::new(Provider::default()), lane, owner)
                .execute("Действую".into(), true)
                .await
                .unwrap(),
        )
        .await;
        assert!(matches!(resumed.last(), Some(AgentEvent::AgentDone { .. })));
    }
}
#[tokio::test]
async fn completed_replay_with_backpressure_stops_when_member_is_revoked() {
    let f = fixture().await;
    let turn = Uuid::new_v4();
    let ports = f.ports(&f.guest, turn).await;
    let input = operation(&ports, "Слушаю").await;
    let owner = Uuid::new_v4();
    ports.claim(input, owner).await.unwrap();
    ports
        .append(f.session.0, ChatMessage::user_text("Слушаю"))
        .await
        .unwrap();
    ports
        .append(
            f.session.0,
            ChatMessage::Assistant {
                content: "Ответ".into(),
            },
        )
        .await
        .unwrap();
    for sequence in 0..100 {
        ports
            .append_event(
                turn,
                owner,
                sequence,
                AgentEvent::TextDelta {
                    text: format!("fragment{sequence}"),
                },
            )
            .await
            .unwrap();
    }
    ports
        .append_event(turn, owner, 100, AgentEvent::AgentDone { total_rounds: 1 })
        .await
        .unwrap();
    ports.finish(turn, owner, true).await.unwrap();
    let replay = runner(
        ports,
        Arc::new(Provider::default()),
        Arc::new(Mutex::new(())),
        owner,
    )
    .execute("Слушаю".into(), false)
    .await
    .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    let host = f
        .rooms
        .authenticate(f.host.room.room_id, f.host.member_id, &f.host.capability)
        .await
        .unwrap();
    f.rooms.revoke(&host, f.guest.member_id).await.unwrap();
    assert!(collect(replay).await.is_empty());
}

#[tokio::test]
async fn resume_ignores_a_historical_blank_terminal_assistant() {
    let f = fixture().await;
    let turn = Uuid::new_v4();
    let ports = f.ports(&f.guest, turn).await;
    let owner = Uuid::new_v4();
    let input = operation(&ports, "Осматриваюсь").await;
    ports.claim(input, owner).await.unwrap();
    ports
        .append(f.session.0, ChatMessage::user_text("Осматриваюсь"))
        .await
        .unwrap();
    ports
        .append(
            f.session.0,
            ChatMessage::Assistant {
                content: " \n ".into(),
            },
        )
        .await
        .unwrap();
    ports.finish(turn, owner, false).await.unwrap();
    let provider = Arc::new(Provider::default());
    let events = collect(
        runner(ports, provider.clone(), Arc::new(Mutex::new(())), owner)
            .execute("Осматриваюсь".into(), true)
            .await
            .unwrap(),
    )
    .await;
    assert!(matches!(events.last(), Some(AgentEvent::AgentDone { .. })));
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn temporary_sqlite_lock_retries_validation_without_unverified_delivery() {
    let f = fixture().await;
    let turn = Uuid::new_v4();
    let ports = f.ports(&f.guest, turn).await;
    let owner = Uuid::new_v4();
    let provider = Arc::new(Provider::default());
    collect(
        runner(
            ports.clone(),
            provider.clone(),
            Arc::new(Mutex::new(())),
            owner,
        )
        .execute("Слушаю".into(), false)
        .await
        .unwrap(),
    )
    .await;
    let replay = runner(ports, provider.clone(), Arc::new(Mutex::new(())), owner)
        .execute("Слушаю".into(), false)
        .await
        .unwrap();
    let mut blocker = f.store.pool().begin_with("BEGIN IMMEDIATE").await.unwrap();
    sqlx::query("UPDATE room_dm_turns SET process_owner=process_owner WHERE turn_id=?1")
        .bind(turn.to_string())
        .execute(&mut *blocker)
        .await
        .unwrap();
    let waiting = tokio::spawn(collect(replay));
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(
        !waiting.is_finished(),
        "No event may be delivered while verification is locked"
    );
    blocker.commit().await.unwrap();
    let events = waiting.await.unwrap();
    assert!(matches!(events.last(), Some(AgentEvent::AgentDone { .. })));
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn persistent_sqlite_lock_exhausts_bounded_retry_without_delivering_events() {
    let f = fixture().await;
    let turn = Uuid::new_v4();
    let ports = f.ports(&f.guest, turn).await;
    let owner = Uuid::new_v4();
    let provider = Arc::new(Provider::default());
    collect(
        runner(
            ports.clone(),
            provider.clone(),
            Arc::new(Mutex::new(())),
            owner,
        )
        .execute("Слушаю".into(), false)
        .await
        .unwrap(),
    )
    .await;
    let replay = runner(
        ports.clone(),
        provider.clone(),
        Arc::new(Mutex::new(())),
        owner,
    )
    .execute("Слушаю".into(), false)
    .await
    .unwrap();
    let mut blocker = f.store.pool().begin_with("BEGIN IMMEDIATE").await.unwrap();
    sqlx::query("UPDATE room_dm_turns SET process_owner=process_owner WHERE turn_id=?1")
        .bind(turn.to_string())
        .execute(&mut *blocker)
        .await
        .unwrap();
    let events = tokio::time::timeout(std::time::Duration::from_secs(2), collect(replay))
        .await
        .unwrap();
    assert!(events.is_empty());
    blocker.rollback().await.unwrap();
    let recovered = collect(
        runner(ports, provider.clone(), Arc::new(Mutex::new(())), owner)
            .execute("Слушаю".into(), false)
            .await
            .unwrap(),
    )
    .await;
    assert!(matches!(
        recovered.last(),
        Some(AgentEvent::AgentDone { .. })
    ));
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
}

struct StopFailsOnce {
    failures: AtomicUsize,
}
impl app_application::ports::room_listener::RoomListenerControl for StopFailsOnce {
    fn start(
        &self,
        _: app_application::ports::room_listener::RoomListenerSpec,
    ) -> app_application::ports::room_listener::RoomListenerFuture<'_, std::net::SocketAddr> {
        Box::pin(async { Ok("127.0.0.1:4141".parse().unwrap()) })
    }
    fn stop(&self, _: Uuid) -> app_application::ports::room_listener::RoomListenerFuture<'_, ()> {
        Box::pin(async move {
            if self
                .failures
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                    count.checked_sub(1)
                })
                .is_ok()
            {
                return Err(app_application::ports::room_listener::RoomListenerError {
                    code: "fixture_stop_failed",
                });
            }
            Ok(())
        })
    }
}
struct DeleteFailsOnce {
    inner: Arc<dyn app_application::ports::secrets::SecretsStore>,
    failures: AtomicUsize,
}
#[async_trait::async_trait]
impl app_application::ports::secrets::SecretsStore for DeleteFailsOnce {
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
        self.inner.set(key, value).await
    }
    async fn delete(&self, key: &str) -> Result<(), app_application::ports::secrets::SecretsError> {
        if key == "native:host_room_request"
            && self
                .failures
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                    count.checked_sub(1)
                })
                .is_ok()
        {
            return Err(app_application::ports::secrets::SecretsError::Operation {
                operation: "delete",
                code: "fixture_delete_failed",
            });
        }
        self.inner.delete(key).await
    }
}
#[tokio::test]
async fn native_stop_resumes_cleanup_after_transport_or_secret_failure() {
    for fail_transport in [true, false] {
        let f = fixture().await;
        let state = app_server::AppState::new(
            Arc::new(Provider::default()),
            "fixture".into(),
            f.store.pool().clone(),
        );
        if !fail_transport {
            state.set_secrets_repo(Arc::new(DeleteFailsOnce {
                inner: state.secrets_repo(),
                failures: AtomicUsize::new(1),
            }));
        }
        let control = app_server::http_services::native_room_host_http_service(
            state.clone(),
            f.rooms.clone(),
            Arc::new(StopFailsOnce {
                failures: AtomicUsize::new(usize::from(fail_transport)),
            }),
        );
        let request = app_application::models::room_host::NativeHostRoomRequest {
            request_id: Uuid::new_v4(),
            campaign_id: f.campaign,
            session_id: f.session,
            expected_revision: f
                .store
                .load_current(f.campaign)
                .await
                .unwrap()
                .campaign
                .revision,
            bind_addr: "127.0.0.1:0".parse().unwrap(),
        };
        let grant = control.start(request.clone()).await.unwrap();
        assert!(control.stop(grant.room.room_id).await.is_err());
        assert!(
            !f.store
                .room(grant.room.room_id)
                .await
                .unwrap()
                .unwrap()
                .active
        );
        control.stop(grant.room.room_id).await.unwrap();
        control.stop(grant.room.room_id).await.unwrap();
        assert!(state
            .secrets_repo()
            .get("native:host_room_request")
            .await
            .unwrap()
            .is_none());
        assert!(control.start(request).await.is_err());
    }
}

struct CredentialReadFailsOnce {
    inner: Arc<dyn app_application::ports::secrets::SecretsStore>,
    failures: AtomicUsize,
}
#[async_trait::async_trait]
impl app_application::ports::secrets::SecretsStore for CredentialReadFailsOnce {
    async fn get(
        &self,
        key: &str,
    ) -> Result<Option<String>, app_application::ports::secrets::SecretsError> {
        if key.contains(":credential:")
            && self
                .failures
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                    count.checked_sub(1)
                })
                .is_ok()
        {
            return Err(app_application::ports::secrets::SecretsError::Operation {
                operation: "get",
                code: "fixture_credential_failed",
            });
        }
        self.inner.get(key).await
    }
    async fn set(
        &self,
        key: &str,
        value: &str,
    ) -> Result<(), app_application::ports::secrets::SecretsError> {
        self.inner.set(key, value).await
    }
    async fn delete(&self, key: &str) -> Result<(), app_application::ports::secrets::SecretsError> {
        self.inner.delete(key).await
    }
}
#[tokio::test]
async fn native_grant_read_failure_compensates_committed_creation_before_solo_resumes() {
    let f = fixture().await;
    let state = app_server::AppState::new(
        Arc::new(Provider::default()),
        "fixture".into(),
        f.store.pool().clone(),
    );
    let faulty = Arc::new(CredentialReadFailsOnce {
        inner: state.secrets_repo(),
        failures: AtomicUsize::new(1),
    });
    state.set_secrets_repo(faulty.clone());
    let rooms = Arc::new(RoomService::new(
        f.store.clone(),
        f.store.clone(),
        faulty,
        Arc::new(SystemRoomClock),
    ));
    let service = app_server::http_services::native_room_host_http_service(
        state.clone(),
        rooms,
        Arc::new(StopFailsOnce {
            failures: AtomicUsize::new(0),
        }),
    );
    let mut request = app_application::models::room_host::NativeHostRoomRequest {
        request_id: Uuid::new_v4(),
        campaign_id: f.campaign,
        session_id: f.session,
        expected_revision: f
            .store
            .load_current(f.campaign)
            .await
            .unwrap()
            .campaign
            .revision,
        bind_addr: "127.0.0.1:0".parse().unwrap(),
    };
    assert!(service.start(request.clone()).await.is_err());
    let active: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM coop_rooms WHERE active=1")
        .fetch_one(f.store.pool())
        .await
        .unwrap();
    assert_eq!(active, 0);
    let private = f
        .store
        .append(
            f.session.0,
            ChatMessage::user_text("PRIVATE_AFTER_GRANT_DELIVERY_FAILURE"),
        )
        .await
        .unwrap();
    let shared: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM shared_messages WHERE message_id=?1)")
            .bind(private.id.to_string())
            .fetch_one(f.store.pool())
            .await
            .unwrap();
    assert!(!shared);
    assert!(state
        .secrets_repo()
        .get("native:host_room_request")
        .await
        .unwrap()
        .is_none());
    request.request_id = Uuid::new_v4();
    request.expected_revision = f
        .store
        .load_current(f.campaign)
        .await
        .unwrap()
        .campaign
        .revision;
    let grant = service.start(request).await.unwrap();
    service.stop(grant.room.room_id).await.unwrap();
}

struct JoinSaveFailsOnce {
    inner: Arc<dyn app_application::ports::secrets::SecretsStore>,
    calls: AtomicUsize,
}
#[async_trait::async_trait]
impl app_application::ports::secrets::SecretsStore for JoinSaveFailsOnce {
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
        if key.starts_with("native:guest-join:") && self.calls.fetch_add(1, Ordering::SeqCst) == 1 {
            return Err(app_application::ports::secrets::SecretsError::Operation {
                operation: "set",
                code: "fixture_join_save_failed",
            });
        }
        self.inner.set(key, value).await
    }
    async fn delete(&self, key: &str) -> Result<(), app_application::ports::secrets::SecretsError> {
        self.inner.delete(key).await
    }
}
#[tokio::test]
async fn native_guest_recovers_a_committed_remote_join_when_final_local_save_failed() {
    let f = fixture().await;
    let host = f
        .rooms
        .authenticate(f.host.room.room_id, f.host.member_id, &f.host.capability)
        .await
        .unwrap();
    let invitation = f.rooms.rotate_invitation(&host).await.unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let router = adapter_http::rooms::router(f.rooms.clone());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let local_db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    adapter_sqlite::pool::migrate(&local_db).await.unwrap();
    let local =
        app_server::AppState::new(Arc::new(Provider::default()), "fixture".into(), local_db);
    local.set_secrets_repo(Arc::new(JoinSaveFailsOnce {
        inner: local.secrets_repo(),
        calls: AtomicUsize::new(0),
    }));
    let join = app_application::models::room_client::NativeGuestJoinRequest {
        address,
        room_id: f.host.room.room_id,
        join_id: Uuid::new_v4(),
        display_name: "Recovering guest".into(),
        invitation: invitation.expose().into(),
    };
    assert!(app_server::native_room_client::service(local.clone())
        .unwrap()
        .join(join.clone())
        .await
        .is_err());
    let before = f.rooms.current_room(&host).await.unwrap();
    assert_eq!(before.members.len(), 3);
    let recovered = app_server::native_room_client::service(local.clone())
        .unwrap()
        .resume(
            app_application::models::room_client::NativeGuestResumeRequest {
                room_id: join.room_id,
                join_id: join.join_id,
            },
        )
        .await
        .unwrap();
    let after = f.rooms.current_room(&host).await.unwrap();
    assert_eq!(after.members.len(), 3);
    assert!(after
        .members
        .iter()
        .any(|member| member.member_id == recovered.member_id));
    server.abort();
}

#[tokio::test]
async fn native_guest_resumes_after_stronghold_reopening_and_rejects_revocation() {
    let f = fixture().await;
    let host = f
        .rooms
        .authenticate(f.host.room.room_id, f.host.member_id, &f.host.capability)
        .await
        .unwrap();
    let invitation = f.rooms.rotate_invitation(&host).await.unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let router = adapter_http::rooms::router(f.rooms.clone());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let vault_path = std::env::temp_dir().join(format!("dmai-guest-{}.hold", Uuid::new_v4()));
    let local_db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    adapter_sqlite::pool::migrate(&local_db).await.unwrap();
    let local = app_server::AppState::new(
        Arc::new(Provider::default()),
        "fixture".into(),
        local_db.clone(),
    );
    local.set_secrets_repo(Arc::new(
        adapter_secrets::StrongholdSecretsStore::open(
            vault_path.clone(),
            b"guest-test-passphrase".to_vec(),
        )
        .unwrap(),
    ));
    let join = app_application::models::room_client::NativeGuestJoinRequest {
        address,
        room_id: f.host.room.room_id,
        join_id: Uuid::new_v4(),
        display_name: "Persistent guest".into(),
        invitation: invitation.expose().into(),
    };
    let first = app_server::native_room_client::service(local.clone())
        .unwrap()
        .join(join.clone())
        .await
        .unwrap();
    drop(local);
    let reopened =
        app_server::AppState::new(Arc::new(Provider::default()), "fixture".into(), local_db);
    reopened.set_secrets_repo(Arc::new(
        adapter_secrets::StrongholdSecretsStore::open(
            vault_path.clone(),
            b"guest-test-passphrase".to_vec(),
        )
        .unwrap(),
    ));
    let service = app_server::native_room_client::service(reopened.clone()).unwrap();
    let request = app_application::models::room_client::NativeGuestResumeRequest {
        room_id: join.room_id,
        join_id: join.join_id,
    };
    let recovered = service
        .resume(
            app_application::models::room_client::NativeGuestResumeRequest {
                room_id: request.room_id,
                join_id: request.join_id,
            },
        )
        .await
        .unwrap();
    assert_eq!(first.member_id, recovered.member_id);
    assert_eq!(first.capability, recovered.capability);
    assert_eq!(f.rooms.current_room(&host).await.unwrap().members.len(), 3);
    f.rooms.revoke(&host, recovered.member_id).await.unwrap();
    assert!(service.resume(request).await.is_err());
    drop(service);
    drop(reopened);
    server.abort();
    std::fs::remove_file(vault_path).unwrap();
}

#[tokio::test]
async fn native_transport_scopes_private_state_and_recovers_exact_pending_action() {
    use app_application::models::room_transport::*;
    let f = fixture().await;
    let reserved = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = reserved.local_addr().unwrap();
    drop(reserved);
    let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&db).await.unwrap();
    let local = app_server::AppState::new(Arc::new(Provider::default()), "fixture".into(), db);
    let service = app_server::native_room_transport::service(local.clone()).unwrap();
    let revision = f
        .store
        .load_current(f.campaign)
        .await
        .unwrap()
        .campaign
        .revision
        .0;
    let request = NativeRoomTransportRequest {
        address,
        room_id: f.host.room.room_id,
        member_id: f.host.member_id,
        capability: f.host.capability.expose().into(),
        operation: NativeRoomOperation::Rest {
            request_id: "transport-wait".into(),
            expected_revision: revision,
            intent: app_application::models::rest::RestIntent::Wait { minutes: 1 },
        },
    };
    assert!(service.execute(request.clone()).await.is_err());
    let mut read = request.clone();
    read.operation = NativeRoomOperation::PendingAction {};
    let pending = service.execute(read.clone()).await.unwrap().body.unwrap();
    assert_eq!(pending["request_id"], "transport-wait");
    assert!(!pending.to_string().contains(f.host.capability.expose()));
    let mut changed = request.clone();
    changed.operation = NativeRoomOperation::Rest {
        request_id: "another-wait".into(),
        expected_revision: revision,
        intent: app_application::models::rest::RestIntent::Wait { minutes: 1 },
    };
    assert!(matches!(
        service.execute(changed).await,
        Err(adapter_http::services::HttpServiceError::Conflict {
            code: "native_room_action_pending"
        })
    ));
    // Recreate the client service as a native backend would after interruption.
    let service = app_server::native_room_transport::service(local).unwrap();
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    let router = adapter_http::rooms::router(f.rooms.clone());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let result = service.execute(request.clone()).await.unwrap();
    assert_eq!(result.status, 200, "{:?}", result.body);
    assert!(service.execute(read).await.unwrap().body.is_none());
    let replay = service.execute(request).await.unwrap();
    assert_eq!(replay.status, 200);
    assert_eq!(replay.body.unwrap()["duplicate"], true);
    let guest = NativeRoomTransportRequest {
        address,
        room_id: f.guest.room.room_id,
        member_id: f.guest.member_id,
        capability: f.guest.capability.expose().into(),
        operation: NativeRoomOperation::Party {},
    };
    let own = service.execute(guest.clone()).await.unwrap();
    assert_eq!(own.status, 200);
    let own = own.body.unwrap();
    assert_eq!(own["own_character"]["name"], "Tara");
    assert!(own.get("characters").is_none());
    // Host-only operations retain the remote rejection status for the renderer.
    let mut rejected = guest;
    rejected.operation = NativeRoomOperation::PendingProposals {};
    assert_eq!(service.execute(rejected).await.unwrap().status, 401);
    server.abort();
}

#[tokio::test]
async fn native_transport_plain_text_rejection_releases_pending_mutation() {
    use app_application::models::room_transport::*;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let router = axum::Router::new().route(
        "/rooms/{room}/rest",
        axum::routing::post(|| async {
            (
                axum::http::StatusCode::PAYLOAD_TOO_LARGE,
                "plain rejection, never expose this raw text",
            )
        }),
    );
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&db).await.unwrap();
    let local = app_server::AppState::new(Arc::new(Provider::default()), "fixture".into(), db);
    let service = app_server::native_room_transport::service(local).unwrap();
    let mut request = NativeRoomTransportRequest {
        address,
        room_id: Uuid::new_v4(),
        member_id: Uuid::new_v4(),
        capability: RoomSecret::generate().expose().into(),
        operation: NativeRoomOperation::Rest {
            request_id: "rejected".into(),
            expected_revision: 0,
            intent: app_application::models::rest::RestIntent::Wait { minutes: 1 },
        },
    };
    let result = service.execute(request.clone()).await.unwrap();
    assert_eq!(result.status, 413);
    assert_eq!(result.body.unwrap()["code"], "native_room_remote_rejected");
    request.operation = NativeRoomOperation::PendingAction {};
    assert!(service
        .execute(request.clone())
        .await
        .unwrap()
        .body
        .is_none());
    request.operation = NativeRoomOperation::Rest {
        request_id: "corrected".into(),
        expected_revision: 0,
        intent: app_application::models::rest::RestIntent::Wait { minutes: 2 },
    };
    assert_eq!(service.execute(request).await.unwrap().status, 413);
    server.abort();
}
