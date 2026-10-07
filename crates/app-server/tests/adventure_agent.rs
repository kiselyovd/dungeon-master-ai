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
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
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
    MockProvider::new(chunks).with_followup_rounds(vec![vec![ChatChunk::Done {
        reason: FinishReason::Stop,
    }]])
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
            vec![ChatChunk::Done {
                reason: FinishReason::Stop,
            }],
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
