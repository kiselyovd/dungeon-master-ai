use adapter_sqlite::SqliteStore;
use app_application::campaigns::{CreateCampaign, CreateCampaignCommand};
use app_application::models::campaign_state::CommitCampaignCharacter;
use app_application::models::session::CampaignSeed;
use app_application::ports::repositories::CampaignStateRepository;
use app_llm::{ChatChunk, FinishReason, MockProvider};
use app_server::agent::orchestrator::{
    AgentConfig, AgentEvent, AgentOrchestrator, AgentTurnRequest,
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

async fn fixture() -> (
    sqlx::SqlitePool,
    app_application::models::session::CampaignDetail,
) {
    fixture_seed(CampaignSeed::Bundled {
        adventure_id: "bell-beneath-water".into(),
    })
    .await
}
async fn fixture_seed(
    seed: CampaignSeed,
) -> (
    sqlx::SqlitePool,
    app_application::models::session::CampaignDetail,
) {
    fixture_on_pool(
        sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap(),
        seed,
    )
    .await
}
async fn fixture_on_pool(
    pool: sqlx::SqlitePool,
    seed: CampaignSeed,
) -> (
    sqlx::SqlitePool,
    app_application::models::session::CampaignDetail,
) {
    app_server::db::init_db(&pool).await.unwrap();
    let store = Arc::new(SqliteStore::new(pool.clone()));
    let campaign = CreateCampaign::new(store.clone())
        .execute(CreateCampaignCommand {
            title: "Bell".into(),
            premise: "Ferry".into(),
            seed,
        })
        .await
        .unwrap();
    store.commit_character(CommitCampaignCharacter{campaign_id:campaign.id,session_id:campaign.last_session_id,character_id:Uuid::new_v4(),expected_revision:campaign.revision,causation_id:"hero".into(),character:json!({"name":"Mira","heroClass":"fighter","level":1,"hp":12,"hpMax":12,"ac":14,"abilities":{"str":15,"dex":14,"con":13,"int":12,"wis":10,"cha":8},"inventory":[],"portraitUrl":"data:image/png;base64,PRIVATE_PORTRAIT","backstory":"PRIVATE_CHARACTER_HISTORY"})}).await.unwrap();
    (pool, campaign)
}

async fn run(
    pool: sqlx::SqlitePool,
    campaign: &app_application::models::session::CampaignDetail,
    name: &str,
    args: serde_json::Value,
) -> Vec<AgentEvent> {
    let provider = Arc::new(
        finishing_mock(vec![
            ChatChunk::ToolCallStart {
                id: "call-1".into(),
                name: name.into(),
            },
            ChatChunk::ToolCallArgsDelta {
                id: "call-1".into(),
                args_fragment: args.to_string(),
            },
            ChatChunk::ToolCallDone {
                id: "call-1".into(),
            },
            ChatChunk::Done {
                reason: FinishReason::ToolUse,
            },
        ])
        .with_thinking_chunks(vec![
            "PRIVATE_DM_REASONING: the merchant concealed the stolen clapper".into(),
        ]),
    );
    let orchestrator = AgentOrchestrator::new(
        provider,
        pool,
        AgentConfig {
            model: "mock".into(),
            max_rounds: 3,
            ..AgentConfig::default()
        },
        None,
        None,
    );
    let (tx, mut rx) = tokio::sync::mpsc::channel(64);
    orchestrator
        .run(
            AgentTurnRequest {
                campaign_id: campaign.id.0,
                session_id: campaign.last_session_id.0,
                player_message: "Я иду к причалу".into(),
                history: vec![],
                images: vec![],
                board: None,
            },
            tx,
        )
        .await
        .unwrap();
    let mut events = vec![];
    while let Some(event) = rx.recv().await {
        events.push(event);
    }
    events
}

#[tokio::test]
async fn model_adventure_tool_commits_real_state_and_returns_no_private_sheet_blobs() {
    let (pool, campaign) = fixture().await;
    let events = run(
        pool.clone(),
        &campaign,
        "adventure_action",
        json!({"action":{"kind":"move","location":"quay"}}),
    )
    .await;
    let result = events
        .iter()
        .find_map(|event| match event {
            AgentEvent::ToolCallResult {
                result, is_error, ..
            } => {
                assert!(!is_error);
                Some(result)
            }
            _ => None,
        })
        .expect("confirmed tool outcome");
    assert_eq!(result["adventure"]["state"]["location"], "quay");
    assert!(!result.to_string().contains("PRIVATE_PORTRAIT"));
    assert!(!result.to_string().contains("PRIVATE_CHARACTER_HISTORY"));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, AgentEvent::ReasoningText { .. })),
        "DM-only reasoning must not enter player events"
    );
    let current = SqliteStore::new(pool)
        .load_current(campaign.id)
        .await
        .unwrap();
    assert_eq!(current.campaign.revision.0, 3);
}

#[tokio::test]
async fn hallucinated_scene_mutation_is_rejected_for_an_authored_campaign() {
    let (pool, campaign) = fixture().await;
    let events = run(
        pool.clone(),
        &campaign,
        "set_scene",
        json!({"title":"Invented castle","mode":"exploration"}),
    )
    .await;
    assert!(events
        .iter()
        .any(|event| matches!(event, AgentEvent::ToolCallResult { is_error: true, .. })));
    let current = SqliteStore::new(pool)
        .load_current(campaign.id)
        .await
        .unwrap();
    assert!(current.projection.scene.is_none());
    assert_eq!(current.campaign.revision.0, 2);
}

#[tokio::test]
async fn resume_after_committed_action_finishes_narration_without_repeating_effect_or_user_message()
{
    use app_application::agent::operations::{AgentOperation, AgentOperationRepository};
    use app_application::models::adventure::{Action, AdventureIntent, Location};
    use app_application::models::chat::{ChatMessage, ToolCall};
    use app_application::ports::repositories::AdventureRepository;
    let (pool, campaign) = fixture().await;
    let store = SqliteStore::new(pool.clone());
    let id = Uuid::new_v4();
    let owner = Uuid::new_v4();
    let input = AgentOperation {
        request_id: id,
        request: AgentTurnRequest {
            campaign_id: campaign.id.0,
            session_id: campaign.last_session_id.0,
            player_message: "I go to the quay".into(),
            history: vec![],
            images: vec![],
            board: None,
        },
        model: None,
    };
    store.claim(input.clone(), owner).await.unwrap();
    store
        .append_turn_message(
            id,
            campaign.last_session_id.0,
            ChatMessage::user_text("I go to the quay"),
        )
        .await
        .unwrap();
    let args = json!({"action":{"kind":"move","location":"quay"}});
    store
        .append_turn_message(
            id,
            campaign.last_session_id.0,
            ChatMessage::AssistantWithToolCalls {
                content: None,
                tool_calls: vec![ToolCall {
                    id: "call-1".into(),
                    name: "adventure_action".into(),
                    args: args.clone(),
                }],
            },
        )
        .await
        .unwrap();
    store
        .claim_tool_step(
            id,
            "call-1",
            json!({"tool_name":"adventure_action","args":args}),
        )
        .await
        .unwrap();
    store
        .act_as_dm(AdventureIntent {
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            causation_id: format!("agent:{id}:call-1"),
            action: Action::Move(Location::Quay),
        })
        .await
        .unwrap();
    store.finish(id, owner, false).await.unwrap();
    let provider = Arc::new(finishing_mock(vec![
        ChatChunk::TextDelta {
            text: "Your step onto the quay is confirmed. The upper path is safe.".into(),
        },
        ChatChunk::Done {
            reason: FinishReason::Stop,
        },
    ]));
    let server = app_server::test_support::TestServer::start_with(provider, pool.clone()).await;
    let client = reqwest::Client::new();
    let body = json!({"request_id":id,"campaign_id":campaign.id.0,"session_id":campaign.last_session_id.0,"player_message":"I go to the quay","history":[]});
    assert_eq!(
        client
            .post(server.url("/agent/turn"))
            .json(&body)
            .send()
            .await
            .unwrap()
            .status(),
        409
    );
    let mut resume = body;
    resume["resume"] = json!(true);
    let response = client
        .post(server.url("/agent/turn"))
        .json(&resume)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(response.contains("Your step onto the quay is confirmed"));
    assert!(response.contains("agent_done"));
    assert_eq!(
        store
            .load_current(campaign.id)
            .await
            .unwrap()
            .campaign
            .revision
            .0,
        3
    );
    let users: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE session_id=?1 AND role='user'")
            .bind(campaign.last_session_id.0.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(users, 1);
}

fn finishing_mock(chunks: Vec<ChatChunk>) -> MockProvider {
    MockProvider::new(chunks).with_followup_rounds(vec![vec![
        ChatChunk::TextDelta {
            text: "Confirmed tool outcome.".into(),
        },
        ChatChunk::Done {
            reason: FinishReason::Stop,
        },
    ]])
}

struct CapturingHeroProvider {
    inner: MockProvider,
    requests: std::sync::Mutex<Vec<app_llm::ChatRequest>>,
}

#[tokio::test]
async fn collaborative_start_combat_refreshes_catalog_for_the_next_canonical_intention() {
    let (pool, campaign) = fixture_seed(CampaignSeed::Collaborative {
        answers: Default::default(),
    })
    .await;
    let provider = Arc::new(CapturingHeroProvider {
        inner: MockProvider::new(hero_tool_round(
            "start",
            "start_combat",
            json!({"initiative_entries":[{"name":"Echo","role":"npc","hp":6,"ac":11,"roll":1}]}),
        ))
        .with_followup_rounds(vec![
            hero_tool_round("turn", "combat_action", json!({"kind":"end_turn"})),
            vec![
                ChatChunk::TextDelta {
                    text: "The turn is confirmed.".into(),
                },
                ChatChunk::Done {
                    reason: FinishReason::Stop,
                },
            ],
        ]),
        requests: Default::default(),
    });
    let orchestrator = AgentOrchestrator::new(
        provider.clone(),
        pool,
        AgentConfig {
            model: "mock".into(),
            max_rounds: 4,
            ..AgentConfig::default()
        },
        None,
        None,
    );
    let (tx, mut rx) = tokio::sync::mpsc::channel(128);
    orchestrator
        .run(
            AgentTurnRequest {
                campaign_id: campaign.id.0,
                session_id: campaign.last_session_id.0,
                player_message: "Begin the fight and end my first turn.".into(),
                history: vec![],
                images: vec![],
                board: None,
            },
            tx,
        )
        .await
        .unwrap();
    while let Some(event) = rx.recv().await {
        if let AgentEvent::ToolCallResult {
            result, is_error, ..
        } = event
        {
            assert!(!is_error, "{result}");
        }
    }
    let requests = provider.requests.lock().unwrap().clone();
    assert!(!requests[0]
        .tools
        .iter()
        .any(|tool| tool.name == "combat_action"));
    assert!(requests[1]
        .tools
        .iter()
        .any(|tool| tool.name == "combat_action"));
    assert!(requests[1]
        .system_prompt
        .as_ref()
        .unwrap()
        .contains("\"active\":true"));
}

#[tokio::test]
async fn model_retreat_returns_terminal_projection_story_and_rest_status() {
    use app_application::models::adventure::{Action, AdventureIntent, Location};
    use app_application::ports::repositories::AdventureRepository;
    let (pool, campaign) = fixture().await;
    let store = SqliteStore::new(pool.clone());
    for (index, action) in [
        Action::Move(Location::Quay),
        Action::Move(Location::Archive),
        Action::LocateTikhon,
        Action::StartDefense,
    ]
    .into_iter()
    .enumerate()
    {
        store
            .act_as_dm(AdventureIntent {
                campaign_id: campaign.id,
                session_id: campaign.last_session_id,
                causation_id: format!("setup-{index}"),
                action,
            })
            .await
            .unwrap();
    }
    let provider = Arc::new(
        MockProvider::new(hero_tool_round(
            "exit",
            "combat_action",
            json!({"kind":"move","x":1,"y":3}),
        ))
        .with_followup_rounds(vec![
            hero_tool_round("retreat", "combat_action", json!({"kind":"retreat"})),
            vec![
                ChatChunk::TextDelta {
                    text: "You withdraw to the quay; Tikhon still needs rescue.".into(),
                },
                ChatChunk::Done {
                    reason: FinishReason::Stop,
                },
            ],
        ]),
    );
    let orchestrator = AgentOrchestrator::new(
        provider,
        pool,
        AgentConfig {
            model: "mock".into(),
            max_rounds: 4,
            ..AgentConfig::default()
        },
        None,
        None,
    );
    let (tx, mut rx) = tokio::sync::mpsc::channel(128);
    orchestrator
        .run(
            AgentTurnRequest {
                campaign_id: campaign.id.0,
                session_id: campaign.last_session_id.0,
                player_message: "Move to the exit and retreat.".into(),
                history: vec![],
                images: vec![],
                board: None,
            },
            tx,
        )
        .await
        .unwrap();
    let mut terminal = None;
    while let Some(event) = rx.recv().await {
        if let AgentEvent::ToolCallResult {
            result,
            is_error,
            id,
            ..
        } = event
        {
            assert!(!is_error, "{result}");
            if id == "retreat" {
                terminal = Some(result);
            }
        }
    }
    let terminal = terminal.unwrap();
    assert_eq!(terminal["projection"]["snapshot"]["active"], false);
    assert_eq!(
        terminal["projection"]["adventure"]["state"]["defense"],
        "retreated"
    );
    assert_eq!(terminal["rest_status"]["combat_active"], false);
    assert!(store
        .load_current(campaign.id)
        .await
        .unwrap()
        .projection
        .combat
        .is_none());
}
#[async_trait::async_trait]
impl app_llm::LlmProvider for CapturingHeroProvider {
    async fn stream_chat(
        &self,
        request: app_llm::ChatRequest,
    ) -> Result<app_llm::ChunkStream, app_llm::LlmError> {
        self.requests.lock().unwrap().push(request.clone());
        self.inner.stream_chat(request).await
    }
    fn name(&self) -> &'static str {
        "hero-intention-test"
    }
    fn capabilities_for_model(&self, model: &str) -> app_llm::Capabilities {
        self.inner.capabilities_for_model(model)
    }
    fn active_model(&self) -> &str {
        "mock"
    }
}
fn hero_tool_round(id: &str, name: &str, args: serde_json::Value) -> Vec<ChatChunk> {
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
        ChatChunk::Done {
            reason: FinishReason::ToolUse,
        },
    ]
}

#[tokio::test]
async fn model_can_start_defense_then_move_with_refreshed_canonical_context() {
    use app_application::models::adventure::{Action, AdventureIntent, Location};
    use app_application::ports::repositories::AdventureRepository;
    let (pool, campaign) = fixture().await;
    let store = SqliteStore::new(pool.clone());
    for (index, action) in [
        Action::Move(Location::Quay),
        Action::Move(Location::Archive),
        Action::LocateTikhon,
    ]
    .into_iter()
    .enumerate()
    {
        store
            .act_as_dm(AdventureIntent {
                campaign_id: campaign.id,
                session_id: campaign.last_session_id,
                causation_id: format!("setup-{index}"),
                action,
            })
            .await
            .unwrap();
    }
    let provider = Arc::new(CapturingHeroProvider {
        inner: MockProvider::new(hero_tool_round(
            "defense",
            "adventure_action",
            json!({"action":{"kind":"start_defense"}}),
        ))
        .with_followup_rounds(vec![
            hero_tool_round("move", "combat_action", json!({"kind":"move","x":3,"y":4})),
            vec![
                ChatChunk::TextDelta {
                    text: "You reach the dry landing.".into(),
                },
                ChatChunk::Done {
                    reason: FinishReason::Stop,
                },
            ],
        ]),
        requests: Default::default(),
    });
    let orchestrator = AgentOrchestrator::new(
        provider.clone(),
        pool,
        AgentConfig {
            model: "mock".into(),
            max_rounds: 4,
            ..AgentConfig::default()
        },
        None,
        None,
    );
    let (tx, mut rx) = tokio::sync::mpsc::channel(128);
    orchestrator
        .run(
            AgentTurnRequest {
                campaign_id: campaign.id.0,
                session_id: campaign.last_session_id.0,
                player_message: "Hold the passage and move to the dry landing.".into(),
                history: vec![],
                images: vec![],
                board: None,
            },
            tx,
        )
        .await
        .unwrap();
    let mut results = Vec::new();
    while let Some(event) = rx.recv().await {
        if let AgentEvent::ToolCallResult {
            result, is_error, ..
        } = event
        {
            assert!(!is_error, "{result}");
            assert!(!result.to_string().contains("PRIVATE_"));
            results.push(result);
        }
    }
    assert_eq!(results.len(), 2);
    let requests = provider.requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 3);
    assert!(!requests[0]
        .tools
        .iter()
        .any(|tool| tool.name == "combat_action"));
    assert!(requests[1]
        .tools
        .iter()
        .any(|tool| tool.name == "combat_action"));
    assert!(requests[1]
        .system_prompt
        .as_ref()
        .unwrap()
        .contains("bell_archive_defense"));
    assert!(requests[2]
        .system_prompt
        .as_ref()
        .unwrap()
        .contains("\"y\":4"));
    let state = store.load_current(campaign.id).await.unwrap();
    assert_eq!(state.campaign.revision.0, 7);
}

#[tokio::test]
async fn model_wait_intention_updates_real_time_and_returns_public_status() {
    let (pool, campaign) = fixture().await;
    let events = run(
        pool.clone(),
        &campaign,
        "rest_action",
        json!({"kind":"wait","minutes":60}),
    )
    .await;
    let result = events
        .iter()
        .find_map(|event| {
            if let AgentEvent::ToolCallResult {
                result, is_error, ..
            } = event
            {
                assert!(!is_error, "{result}");
                Some(result)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(result["rest_status"]["elapsed_seconds"], 3600);
    assert!(!result.to_string().contains("PRIVATE_"));
    assert_eq!(
        SqliteStore::new(pool)
            .load_current(campaign.id)
            .await
            .unwrap()
            .projection
            .adventuring
            .unwrap()
            .elapsed_seconds,
        3600
    );
}

fn replay_request(
    campaign: &app_application::models::session::CampaignDetail,
    request_id: Uuid,
    tool_id: &str,
    args: serde_json::Value,
) -> app_application::agent::tool_dispatch::ToolDispatchRequest {
    app_application::agent::tool_dispatch::ToolDispatchRequest {
        request_id: request_id.to_string(),
        tool_call_id: tool_id.into(),
        campaign_id: campaign.id.0,
        session_id: campaign.last_session_id.0,
        command: app_application::agent::tool_decoder::decode_tool_call("adventure_action", &args)
            .unwrap(),
    }
}

#[tokio::test]
async fn denied_raw_hero_mutation_recovers_delivery_without_executing_or_losing_signature() {
    use app_application::agent::operations::{
        AgentOperation, AgentOperationRepository, OperationClaim,
    };
    use app_application::models::chat::{ChatMessage, ToolCall, ToolResult};
    let (pool, campaign) = fixture().await;
    let store = SqliteStore::new(pool.clone());
    let id = Uuid::new_v4();
    let owner = Uuid::new_v4();
    let operation = AgentOperation {
        request_id: id,
        request: AgentTurnRequest {
            campaign_id: campaign.id.0,
            session_id: campaign.last_session_id.0,
            player_message: "Look around; do not move me".into(),
            history: vec![],
            images: vec![],
            board: None,
        },
        model: None,
    };
    store.claim(operation.clone(), owner).await.unwrap();
    let args = json!({"id":Uuid::new_v4(),"x":5,"y":2});
    let call = ToolCall {
        id: "denied-raw".into(),
        name: "update_token".into(),
        args: args.clone(),
    };
    store
        .append_turn_message(
            id,
            campaign.last_session_id.0,
            ChatMessage::AssistantWithToolCalls {
                content: None,
                tool_calls: vec![call],
            },
        )
        .await
        .unwrap();
    store
        .append_turn_message(
            id,
            campaign.last_session_id.0,
            ChatMessage::ToolResult(ToolResult {
                tool_call_id: "denied-raw".into(),
                content: json!({"error":"canonical_player_intention_required"}).to_string(),
                is_error: true,
            }),
        )
        .await
        .unwrap();
    let before = store.load_current(campaign.id).await.unwrap();
    store.finish(id, owner, false).await.unwrap();
    let claim = store.resume(operation.clone(), owner).await.unwrap();
    let OperationClaim::Resuming { events, .. } = claim else {
        panic!("expected recovery")
    };
    assert_eq!(events.iter().filter(|event|matches!(event,AgentEvent::ToolCallResult{id:delivery,is_error:true,..} if delivery.ends_with("denied-raw"))).count(),1);
    assert_eq!(store.load_current(campaign.id).await.unwrap(), before);
    let raw: String = sqlx::query_scalar("SELECT result_json FROM agent_tool_steps WHERE request_id=?1 AND tool_call_id='denied-raw'")
        .bind(id.to_string()).fetch_one(&pool).await.unwrap();
    let execution: app_application::agent::tool_dispatch::ToolExecution =
        serde_json::from_str(&raw).unwrap();
    assert!(execution.is_error);
    assert_eq!(
        execution.result["error"],
        "canonical_player_intention_required"
    );
    // Both completed and pending records must reject a transcript/receipt mismatch.
    store.finish(id, owner, false).await.unwrap();
    for pending in [false, true] {
        sqlx::query("UPDATE agent_tool_steps SET command_json='{}',result_json=CASE WHEN ?2 THEN NULL ELSE result_json END WHERE request_id=?1")
            .bind(id.to_string()).bind(pending).execute(&pool).await.unwrap();
        assert!(store.resume(operation.clone(), owner).await.is_err());
        assert_eq!(store.load_current(campaign.id).await.unwrap(), before);
    }
}
#[tokio::test]
async fn replay_only_reads_canonical_effect_without_allocating_or_executing_a_missing_tool() {
    use app_application::{
        models::{
            adventure::{Action, AdventureIntent, Location},
            identity::CampaignId,
        },
        ports::repositories::AdventureRepository,
    };
    let (pool, campaign) = fixture().await;
    let store = SqliteStore::new(pool.clone());
    let id = Uuid::new_v4();
    let mut request = replay_request(
        &campaign,
        id,
        "call-1",
        json!({"action":{"kind":"move","location":"quay"}}),
    );
    let before = store.load_current(campaign.id).await.unwrap();
    assert!(store.replay_player_tool(&request).await.unwrap().is_none());
    assert_eq!(store.load_current(campaign.id).await.unwrap(), before);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_tool_steps")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    store
        .act_as_dm(AdventureIntent {
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            causation_id: format!("agent:{id}:call-1"),
            action: Action::Move(Location::Quay),
        })
        .await
        .unwrap();
    let committed = store.load_current(campaign.id).await.unwrap();
    for _ in 0..2 {
        let result = store.replay_player_tool(&request).await.unwrap().unwrap();
        assert!(!result.is_error);
        assert_eq!(result.result["revision"], committed.campaign.revision.0);
    }
    assert_eq!(store.load_current(campaign.id).await.unwrap(), committed);
    request.command = app_application::agent::tool_decoder::decode_tool_call(
        "adventure_action",
        &json!({"action":{"kind":"inspect_quay"}}),
    )
    .unwrap();
    assert!(store.replay_player_tool(&request).await.is_err());
    request.campaign_id = CampaignId(Uuid::new_v4()).0;
    assert!(store.replay_player_tool(&request).await.is_err());
}
#[tokio::test]
async fn pending_tool_recovery_survives_file_reopen_and_rejects_changed_scope_or_timeline() {
    use app_application::agent::operations::{AgentOperation, AgentOperationRepository};
    use app_application::{
        models::adventure::{Action, AdventureIntent, Location},
        ports::repositories::AdventureRepository,
    };
    let dir = tempfile::tempdir().unwrap();
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(dir.path().join("replay.db"))
        .create_if_missing(true);
    let (pool, campaign) = fixture_on_pool(
        sqlx::SqlitePool::connect_with(options.clone())
            .await
            .unwrap(),
        CampaignSeed::Bundled {
            adventure_id: "bell-beneath-water".into(),
        },
    )
    .await;
    let store = SqliteStore::new(pool.clone());
    let id = Uuid::new_v4();
    let args = json!({"action":{"kind":"move","location":"quay"}});
    let request = replay_request(&campaign, id, "pending", args.clone());
    store
        .claim(
            AgentOperation {
                request_id: id,
                request: AgentTurnRequest {
                    campaign_id: campaign.id.0,
                    session_id: campaign.last_session_id.0,
                    player_message: "go quay".into(),
                    history: vec![],
                    images: vec![],
                    board: None,
                },
                model: None,
            },
            Uuid::new_v4(),
        )
        .await
        .unwrap();
    store
        .claim_tool_step(
            id,
            "pending",
            json!({"tool_name":"adventure_action","args":args}),
        )
        .await
        .unwrap();
    assert!(store.replay_player_tool(&request).await.unwrap().is_none());
    store
        .act_as_dm(AdventureIntent {
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            causation_id: format!("agent:{id}:pending"),
            action: Action::Move(Location::Quay),
        })
        .await
        .unwrap();
    let committed = store.load_current(campaign.id).await.unwrap();
    drop(store);
    pool.close().await;
    let pool = sqlx::SqlitePool::connect_with(options).await.unwrap();
    let store = SqliteStore::new(pool.clone());
    assert!(
        !store
            .replay_player_tool(&request)
            .await
            .unwrap()
            .unwrap()
            .is_error
    );
    assert_eq!(store.load_current(campaign.id).await.unwrap(), committed);
    let pending: Option<String> =
        sqlx::query_scalar("SELECT result_json FROM agent_tool_steps WHERE request_id=?1")
            .bind(id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        pending.is_none(),
        "read-only recovery never writes a new checkpoint"
    );
    let recovered = store.recover_player_tool(&request).await.unwrap().unwrap();
    assert!(!recovered.is_error);
    let repaired: String =
        sqlx::query_scalar("SELECT result_json FROM agent_tool_steps WHERE request_id=?1")
            .bind(id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    let repaired: app_application::agent::tool_dispatch::ToolExecution =
        serde_json::from_str(&repaired).unwrap();
    assert_eq!(repaired.result, recovered.result);
    assert!(!repaired.is_error);
    assert_eq!(store.load_current(campaign.id).await.unwrap(), committed);
    let mut wrong = request.clone();
    wrong.session_id = Uuid::new_v4();
    assert!(store.replay_player_tool(&wrong).await.is_err());
    sqlx::query("UPDATE sessions SET restore_generation=restore_generation+1 WHERE id=?1")
        .bind(request.session_id.to_string())
        .execute(&pool)
        .await
        .unwrap();
    assert!(store.replay_player_tool(&request).await.is_err());
}
#[tokio::test]
async fn rest_receipt_lookup_preserves_resources_and_clock() {
    use app_application::agent::hero_intentions::{HeroIntention, HeroIntentionRepository};
    use app_application::models::rest::RestIntent;
    let (pool, campaign) = fixture().await;
    let store = SqliteStore::new(pool);
    let id = Uuid::new_v4();
    let mut request = replay_request(
        &campaign,
        id,
        "rest",
        json!({"action":{"kind":"move","location":"quay"}}),
    );
    request.command = app_application::agent::tool_decoder::decode_tool_call(
        "rest_action",
        &json!({"kind":"wait","minutes":5}),
    )
    .unwrap();
    assert!(store.replay_player_tool(&request).await.unwrap().is_none());
    store
        .rest_as_dm(
            HeroIntention {
                campaign_id: campaign.id,
                session_id: campaign.last_session_id,
                causation_id: format!("agent:{id}:rest"),
                action: RestIntent::Wait { minutes: 5 },
            },
            1,
        )
        .await
        .unwrap();
    let before = store.load_current(campaign.id).await.unwrap();
    assert!(
        !store
            .replay_player_tool(&request)
            .await
            .unwrap()
            .unwrap()
            .is_error
    );
    assert_eq!(store.load_current(campaign.id).await.unwrap(), before);
    request.command = app_application::agent::tool_decoder::decode_tool_call(
        "rest_action",
        &json!({"kind":"wait","minutes":6}),
    )
    .unwrap();
    assert!(store.replay_player_tool(&request).await.is_err());
}
