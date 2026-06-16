use app_llm::{ChatChunk, FinishReason, MockProvider, ReasoningSpec, ToolCall};
use app_server::agent::orchestrator::AgentEvent;
use app_server::agent::orchestrator::{AgentConfig, AgentOrchestrator, AgentTurnRequest};
use app_server::agent::tool_executor::execute_tool;
use app_server::test_support::TestServer;
use reqwest::Client;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::mpsc;

async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    app_server::db::init_db(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn orchestrator_emits_text_events_from_mock() {
    let pool = test_pool().await;
    let mock = Arc::new(MockProvider::new(vec![
        ChatChunk::TextDelta {
            text: "Hello ".into(),
        },
        ChatChunk::TextDelta {
            text: "adventurer.".into(),
        },
        ChatChunk::Done {
            reason: FinishReason::Stop,
        },
    ]));

    let config = AgentConfig {
        model: "mock".into(),
        system_prompt: "You are a DM.".into(),
        temperature: 0.7,
        max_rounds: 8,
        embedding_model: "multilingual-e5-small".into(),
        tool_availability: app_server::agent::tools::ToolAvailability::all(),
        ..AgentConfig::default()
    };

    let req = AgentTurnRequest {
        campaign_id: uuid::Uuid::new_v4(),
        session_id: uuid::Uuid::new_v4(),
        player_message: "I search the room.".into(),
        history: vec![],
        images: vec![],
        board: None,
    };

    let (tx, mut rx) = mpsc::channel::<AgentEvent>(32);
    let orch = AgentOrchestrator::new(mock, pool, config, None, None);
    orch.run(req, tx).await.unwrap();

    let mut text = String::new();
    let mut done = false;
    while let Some(ev) = rx.recv().await {
        match ev {
            AgentEvent::TextDelta { text: t } => text.push_str(&t),
            AgentEvent::AgentDone { .. } => {
                done = true;
                break;
            }
            _ => {}
        }
    }
    assert_eq!(text, "Hello adventurer.");
    assert!(done);
}

#[tokio::test]
async fn orchestrator_executes_tool_call_and_continues() {
    let pool = test_pool().await;

    // Round 1: LLM emits a roll_dice tool-call.
    // Round 2: after tool result injected, the mock provider returns an empty
    // stream (chunks were drained on round 1), so the orchestrator records a
    // default Stop reason and exits gracefully.
    let mock = Arc::new(MockProvider::new(vec![
        ChatChunk::ToolCallStart {
            id: "c1".into(),
            name: "roll_dice".into(),
        },
        ChatChunk::ToolCallArgsDelta {
            id: "c1".into(),
            args_fragment: r#"{"dice":"1d20"}"#.into(),
        },
        ChatChunk::ToolCallDone { id: "c1".into() },
        ChatChunk::Done {
            reason: FinishReason::ToolUse,
        },
    ]));

    let config = AgentConfig {
        model: "mock".into(),
        system_prompt: "DM".into(),
        temperature: 0.7,
        max_rounds: 8,
        embedding_model: "multilingual-e5-small".into(),
        tool_availability: app_server::agent::tools::ToolAvailability::all(),
        ..AgentConfig::default()
    };
    let req = AgentTurnRequest {
        campaign_id: uuid::Uuid::new_v4(),
        session_id: uuid::Uuid::new_v4(),
        player_message: "roll".into(),
        history: vec![],
        images: vec![],
        board: None,
    };

    let (tx, mut rx) = mpsc::channel::<AgentEvent>(32);

    let orch = AgentOrchestrator::new(mock, pool, config, None, None);
    orch.run(req, tx).await.unwrap();

    let mut got_tool_call_result = false;
    let mut got_done = false;
    while let Some(ev) = rx.recv().await {
        match ev {
            AgentEvent::ToolCallResult { tool_name, .. } => {
                assert_eq!(tool_name, "roll_dice");
                got_tool_call_result = true;
            }
            AgentEvent::AgentDone { .. } => {
                got_done = true;
                break;
            }
            _ => {}
        }
    }
    assert!(got_tool_call_result, "expected tool call result event");
    assert!(got_done);
}

#[tokio::test]
async fn orchestrator_handles_unknown_tool_gracefully() {
    // Hallucinated tool name not in the validator dispatch.
    // validate_tool_call returns UnknownTool, executor returns is_error=true.
    let pool = test_pool().await;
    let mock = Arc::new(MockProvider::new(vec![
        ChatChunk::ToolCallStart {
            id: "c2".into(),
            name: "fly_dragon".into(),
        },
        ChatChunk::ToolCallArgsDelta {
            id: "c2".into(),
            args_fragment: r#"{"title":"The Tavern","mode":"exploration"}"#.into(),
        },
        ChatChunk::ToolCallDone { id: "c2".into() },
        ChatChunk::Done {
            reason: FinishReason::ToolUse,
        },
    ]));

    let config = AgentConfig {
        model: "mock".into(),
        system_prompt: "DM".into(),
        temperature: 0.7,
        max_rounds: 8,
        embedding_model: "multilingual-e5-small".into(),
        tool_availability: app_server::agent::tools::ToolAvailability::all(),
        ..AgentConfig::default()
    };
    let req = AgentTurnRequest {
        campaign_id: uuid::Uuid::new_v4(),
        session_id: uuid::Uuid::new_v4(),
        player_message: "look around".into(),
        history: vec![],
        images: vec![],
        board: None,
    };

    let (tx, mut rx) = mpsc::channel::<AgentEvent>(32);
    let orch = AgentOrchestrator::new(mock, pool, config, None, None);
    orch.run(req, tx).await.unwrap();

    let mut got_error_result = false;
    let mut got_done = false;
    while let Some(ev) = rx.recv().await {
        match ev {
            AgentEvent::ToolCallResult {
                tool_name,
                is_error,
                ..
            } if tool_name == "fly_dragon" && is_error => {
                got_error_result = true;
            }
            AgentEvent::ToolCallResult { .. } => {}
            AgentEvent::AgentDone { .. } => {
                got_done = true;
                break;
            }
            _ => {}
        }
    }
    assert!(
        got_error_result,
        "expected is_error=true result for unknown tool"
    );
    assert!(got_done);
}

#[tokio::test]
async fn agent_turn_endpoint_streams_text() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    app_server::db::init_db(&pool).await.unwrap();
    let mock = Arc::new(MockProvider::new(vec![
        ChatChunk::TextDelta {
            text: "The goblin growls.".into(),
        },
        ChatChunk::Done {
            reason: FinishReason::Stop,
        },
    ]));
    let server = TestServer::start_with(mock, pool).await;

    let client = Client::new();
    let resp = client
        .post(server.url("/agent/turn"))
        .header("content-type", "application/json")
        .body(
            r#"{"player_message":"I attack the goblin","history":[],"campaign_id":"00000000-0000-0000-0000-000000000001","session_id":"00000000-0000-0000-0000-000000000002"}"#,
        )
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(
        body.contains("text_delta"),
        "expected text_delta event in: {body}"
    );
    assert!(
        body.contains("The goblin growls."),
        "expected narration text in: {body}"
    );
    assert!(
        body.contains("agent_done"),
        "expected agent_done event in: {body}"
    );
}

#[tokio::test]
async fn agent_tool_call_result_contains_tool_name() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    app_server::db::init_db(&pool).await.unwrap();

    let mock = std::sync::Arc::new(MockProvider::new(vec![
        ChatChunk::ToolCallStart {
            id: "c1".into(),
            name: "roll_dice".into(),
        },
        ChatChunk::ToolCallArgsDelta {
            id: "c1".into(),
            args_fragment: r#"{"dice":"1d20","reason":"stealth check"}"#.into(),
        },
        ChatChunk::ToolCallDone { id: "c1".into() },
        ChatChunk::Done {
            reason: FinishReason::ToolUse,
        },
    ]));

    let server = TestServer::start_with(mock, pool).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(server.url("/agent/turn"))
        .header("content-type", "application/json")
        .body(
            serde_json::json!({
                "campaign_id": "00000000-0000-0000-0000-000000000001",
                "session_id": "00000000-0000-0000-0000-000000000002",
                "player_message": "I try to sneak past the guard",
                "history": []
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(
        body.contains("tool_call_start"),
        "missing tool_call_start in: {body}"
    );
    assert!(
        body.contains("tool_call_result"),
        "missing tool_call_result in: {body}"
    );
    assert!(body.contains("roll_dice"), "missing roll_dice in: {body}");
    assert!(body.contains("agent_done"), "missing agent_done in: {body}");
}

#[tokio::test]
async fn start_combat_executor_persists_passed_session_id() {
    let pool = test_pool().await;
    let session_id = uuid::Uuid::new_v4();
    let campaign_id = uuid::Uuid::new_v4();
    assert_ne!(session_id, campaign_id);

    let tc = ToolCall {
        id: "tc-start".into(),
        name: "start_combat".into(),
        args: serde_json::json!({ "initiative_entries": [] }),
    };
    let (val, is_err) =
        execute_tool(&tc, &pool, None, None, None, "", campaign_id, session_id).await;
    assert!(!is_err, "executor failed: {val}");

    let encounter_id = val["encounter_id"].as_str().expect("encounter_id");
    use sqlx::Row;
    let row = sqlx::query("SELECT session_id FROM combat_encounters WHERE id = ?1")
        .bind(encounter_id)
        .fetch_one(&pool)
        .await
        .expect("fetch encounter row");
    let stored: String = row.get("session_id");
    assert_eq!(
        stored,
        session_id.to_string(),
        "combat_encounters.session_id must equal the session_id passed to execute_tool"
    );
}

#[tokio::test]
async fn quick_save_executor_uses_session_id_not_campaign_id() {
    let pool = test_pool().await;
    let session_id = uuid::Uuid::new_v4();
    let campaign_id = uuid::Uuid::new_v4();
    assert_ne!(session_id, campaign_id);

    let tc = ToolCall {
        id: "tc-save".into(),
        name: "quick_save".into(),
        args: serde_json::json!({ "label": "before the boss" }),
    };
    let (val, is_err) =
        execute_tool(&tc, &pool, None, None, None, "", campaign_id, session_id).await;
    assert!(!is_err, "executor failed: {val}");

    let save_id = val["save_id"].as_str().expect("save_id");
    use sqlx::Row;
    let row = sqlx::query("SELECT session_id FROM snapshots WHERE id = ?1")
        .bind(save_id)
        .fetch_one(&pool)
        .await
        .expect("fetch snapshot row");
    let stored: String = row.get("session_id");
    assert_eq!(
        stored,
        session_id.to_string(),
        "snapshots.session_id must equal the session_id passed to execute_tool"
    );
    assert_ne!(
        stored,
        campaign_id.to_string(),
        "snapshots.session_id must NOT be the campaign_id"
    );
}

#[tokio::test]
async fn journal_endpoint_returns_entries_after_agent_appends() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    app_server::db::init_db(&pool).await.unwrap();

    let campaign_id = uuid::Uuid::new_v4();

    app_server::db::journal_insert(
        &pool,
        campaign_id,
        "<p>The party defeated the goblins.</p>",
        Some("Chapter 1"),
    )
    .await
    .unwrap();

    let server = TestServer::start_with(std::sync::Arc::new(MockProvider::new(vec![])), pool).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(server.url(&format!("/journal?campaign_id={campaign_id}")))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let entries: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["chapter"].as_str(), Some("Chapter 1"));
}

// M8-DM: orchestrator ThinkingDelta -> AgentEvent::ReasoningText e2e
#[tokio::test]
async fn orchestrator_emits_reasoning_text_from_thinking_chunks() {
    let pool = test_pool().await;
    let mock = Arc::new(
        MockProvider::new(vec![
            ChatChunk::TextDelta {
                text: "The answer is 42.".into(),
            },
            ChatChunk::Done {
                reason: FinishReason::Stop,
            },
        ])
        .with_thinking_chunks(vec!["Step 1: think.".into(), " Step 2: conclude.".into()]),
    );

    let config = AgentConfig {
        model: "mock".into(),
        system_prompt: "DM".into(),
        temperature: 0.7,
        max_rounds: 8,
        embedding_model: "multilingual-e5-small".into(),
        tool_availability: app_server::agent::tools::ToolAvailability::all(),
        reasoning_enabled: true,
        reasoning_budget: ReasoningSpec::Medium,
    };

    let req = AgentTurnRequest {
        campaign_id: uuid::Uuid::new_v4(),
        session_id: uuid::Uuid::new_v4(),
        player_message: "What is the meaning of life?".into(),
        history: vec![],
        images: vec![],
        board: None,
    };

    let (tx, mut rx) = mpsc::channel::<AgentEvent>(64);
    let orch = AgentOrchestrator::new(mock, pool, config, None, None);
    orch.run(req, tx).await.unwrap();

    let mut reasoning_texts: Vec<String> = Vec::new();
    let mut text = String::new();
    let mut done = false;
    while let Some(ev) = rx.recv().await {
        match ev {
            AgentEvent::ReasoningText { text: t } => reasoning_texts.push(t),
            AgentEvent::TextDelta { text: t } => text.push_str(&t),
            AgentEvent::AgentDone { .. } => {
                done = true;
                break;
            }
            _ => {}
        }
    }
    assert_eq!(
        reasoning_texts,
        vec!["Step 1: think.", " Step 2: conclude."]
    );
    assert_eq!(text, "The answer is 42.");
    assert!(done);
}

// M8-DM: SSE stream carries reasoning_text event type when mock emits ThinkingDelta
#[tokio::test]
async fn agent_endpoint_streams_reasoning_text_event() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    app_server::db::init_db(&pool).await.unwrap();

    let mock = Arc::new(
        MockProvider::new(vec![
            ChatChunk::TextDelta {
                text: "Narration.".into(),
            },
            ChatChunk::Done {
                reason: FinishReason::Stop,
            },
        ])
        .with_thinking_chunks(vec!["Internal reasoning.".into()]),
    );
    let server = TestServer::start_with(mock, pool).await;

    let client = Client::new();
    let resp = client
        .post(server.url("/agent/turn"))
        .header("content-type", "application/json")
        .body(
            r#"{"player_message":"Go!","history":[],"campaign_id":"00000000-0000-0000-0000-000000000001","session_id":"00000000-0000-0000-0000-000000000002"}"#,
        )
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(
        body.contains("reasoning_text"),
        "expected reasoning_text event in: {body}"
    );
    assert!(
        body.contains("Internal reasoning."),
        "expected thinking text in: {body}"
    );
    assert!(
        body.contains("text_delta"),
        "expected text_delta event in: {body}"
    );
}

#[tokio::test]
async fn execute_tool_generate_illustration_calls_provider_and_returns_bytes() {
    use app_server::image::provider::ImageProvider;
    use app_server::image::stub::LocalImageSidecarProvider;
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine;
    use std::sync::Arc;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    let png = b"PNG-IMAGE-BYTES";
    let encoded = B64.encode(png);
    Mock::given(method("POST"))
        .and(path("/generate"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({ "image_b64": encoded, "mime": "image/png" })),
        )
        .mount(&server)
        .await;

    let provider: Arc<dyn ImageProvider> = Arc::new(LocalImageSidecarProvider::new(server.uri()));
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let tc = app_llm::ToolCall {
        id: "tc-img-1".into(),
        name: "generate_illustration".into(),
        args: serde_json::json!({ "prompt": "a torchlit dungeon corridor" }),
    };

    let (result, is_error) = execute_tool(
        &tc,
        &pool,
        Some(provider),
        None,
        None,
        "",
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
    )
    .await;

    assert!(
        !is_error,
        "generate_illustration should succeed, got: {result:?}"
    );
    assert_eq!(result["status"], "generated");
    assert_eq!(result["mime_type"], "image/png");
    let returned = result["image_b64"].as_str().expect("image_b64 present");
    assert_eq!(B64.decode(returned).unwrap(), png);
}

#[tokio::test]
async fn orchestrator_strips_image_b64_from_tool_result_into_dedicated_event() {
    use app_server::image::provider::ImageProvider;
    use app_server::image::stub::LocalImageSidecarProvider;
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // Stand up a wiremock that returns a minimal image response.
    let server = MockServer::start().await;
    let png = b"FAKE-PNG-BYTES";
    let encoded = B64.encode(png);
    Mock::given(method("POST"))
        .and(path("/generate"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({ "image_b64": encoded, "mime": "image/png" })),
        )
        .mount(&server)
        .await;

    let image_provider: Arc<dyn ImageProvider> =
        Arc::new(LocalImageSidecarProvider::new(server.uri()));

    let pool = test_pool().await;

    // MockProvider emits a generate_illustration tool-call then a ToolUse finish,
    // followed by an empty round that exits with a default Stop reason.
    let mock = Arc::new(MockProvider::new(vec![
        ChatChunk::ToolCallStart {
            id: "img-tc-1".into(),
            name: "generate_illustration".into(),
        },
        ChatChunk::ToolCallArgsDelta {
            id: "img-tc-1".into(),
            args_fragment: r#"{"prompt":"a torchlit dungeon"}"#.into(),
        },
        ChatChunk::ToolCallDone {
            id: "img-tc-1".into(),
        },
        ChatChunk::Done {
            reason: FinishReason::ToolUse,
        },
    ]));

    let config = AgentConfig {
        model: "mock".into(),
        system_prompt: "DM".into(),
        temperature: 0.7,
        max_rounds: 8,
        embedding_model: "multilingual-e5-small".into(),
        tool_availability: app_server::agent::tools::ToolAvailability::all(),
        ..AgentConfig::default()
    };

    let req = AgentTurnRequest {
        campaign_id: uuid::Uuid::new_v4(),
        session_id: uuid::Uuid::new_v4(),
        player_message: "draw the dungeon".into(),
        history: vec![],
        images: vec![],
        board: None,
    };

    let (tx, mut rx) = mpsc::channel::<AgentEvent>(64);
    // Pass the wiremock-backed image provider as the 5th argument.
    let orch = AgentOrchestrator::new(mock, pool, config, None, Some(image_provider));
    orch.run(req, tx).await.unwrap();

    // Drain all events.
    let mut image_generated: Option<AgentEvent> = None;
    let mut tool_call_result: Option<AgentEvent> = None;
    let mut got_done = false;
    while let Some(ev) = rx.recv().await {
        match &ev {
            AgentEvent::ImageGenerated { .. } => image_generated = Some(ev),
            AgentEvent::ToolCallResult { tool_name, .. }
                if tool_name == "generate_illustration" =>
            {
                tool_call_result = Some(ev)
            }
            AgentEvent::AgentDone { .. } => {
                got_done = true;
                break;
            }
            _ => {}
        }
    }
    assert!(got_done, "expected AgentDone");

    // Assert the ImageGenerated event carries the original bytes.
    let img_ev = image_generated.expect("expected ImageGenerated event");
    if let AgentEvent::ImageGenerated {
        image_b64,
        mime_type,
        kind,
        ..
    } = img_ev
    {
        assert_eq!(
            B64.decode(&image_b64).unwrap(),
            png,
            "ImageGenerated image_b64 should decode to the original bytes"
        );
        assert_eq!(mime_type, "image/png");
        assert_eq!(kind, "chat", "generate_illustration routes to chat");
    } else {
        panic!("unexpected event type for image_generated");
    }

    // Assert the ToolCallResult does NOT contain image_b64 (strip fired),
    // but DOES still carry status and mime_type.
    let result_ev = tool_call_result.expect("expected ToolCallResult for generate_illustration");
    if let AgentEvent::ToolCallResult { result, .. } = result_ev {
        assert!(
            result.get("image_b64").is_none(),
            "image_b64 must be stripped from ToolCallResult; got: {result}"
        );
        assert_eq!(
            result["status"].as_str(),
            Some("generated"),
            "status field must remain after strip"
        );
        assert_eq!(
            result["mime_type"].as_str(),
            Some("image/png"),
            "mime_type field must remain after strip"
        );
    } else {
        panic!("unexpected event type for tool_call_result");
    }
}

#[tokio::test]
async fn execute_tool_generate_illustration_without_provider_is_a_clean_error() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let tc = app_llm::ToolCall {
        id: "tc-img-2".into(),
        name: "generate_illustration".into(),
        args: serde_json::json!({ "prompt": "anything" }),
    };
    let (result, is_error) = execute_tool(
        &tc,
        &pool,
        None,
        None,
        None,
        "",
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
    )
    .await;
    assert!(is_error);
    assert!(result["error"].is_string());
}

#[tokio::test]
async fn execute_set_scene_persists_and_is_retrievable() {
    use app_server::db::scene_latest;
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    app_server::db::init_db(&pool).await.unwrap();

    let campaign_id = uuid::Uuid::new_v4();
    let session_id = uuid::Uuid::new_v4();

    let tc = app_llm::ToolCall {
        id: "tc-scene-1".into(),
        name: "set_scene".into(),
        args: serde_json::json!({
            "title": "The Dragon's Lair",
            "subtitle": "A cave of fire",
            "mode": "combat"
        }),
    };

    let (val, is_err) =
        execute_tool(&tc, &pool, None, None, None, "", campaign_id, session_id).await;
    assert!(!is_err, "execute_set_scene failed: {val}");
    assert_eq!(val["title"].as_str(), Some("The Dragon's Lair"));
    assert_eq!(val["mode"].as_str(), Some("combat"));

    // Verify a row was written.
    let scene = scene_latest(&pool, campaign_id).await.unwrap().unwrap();
    assert_eq!(scene.title, "The Dragon's Lair");
    assert_eq!(scene.subtitle.as_deref(), Some("A cave of fire"));
    assert_eq!(scene.mode, "combat");
}

// ---------------------------------------------------------------------------
// Damage resistance / immunity / vulnerability integration tests (W2.4a)
// ---------------------------------------------------------------------------

/// Helper: spin up an in-memory pool, run migrations, start an encounter via
/// execute_tool, and return (pool, session_id, campaign_id).
async fn setup_encounter_pool() -> (SqlitePool, uuid::Uuid, uuid::Uuid) {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    app_server::db::init_db(&pool).await.unwrap();

    let campaign_id = uuid::Uuid::new_v4();
    let session_id = uuid::Uuid::new_v4();

    // Start an encounter so add_token has an active encounter to attach to.
    let tc = ToolCall {
        id: "tc-start-enc".into(),
        name: "start_combat".into(),
        args: serde_json::json!({ "initiative_entries": [{ "name": "Hero" }] }),
    };
    let (val, is_err) =
        execute_tool(&tc, &pool, None, None, None, "", campaign_id, session_id).await;
    assert!(!is_err, "start_combat failed: {val}");

    (pool, session_id, campaign_id)
}

#[tokio::test]
async fn apply_damage_fire_resistance_halves_damage() {
    let (pool, session_id, campaign_id) = setup_encounter_pool().await;

    // Add a token with fire resistance; max_hp = 20, current_hp = 20.
    let add_tc = ToolCall {
        id: "tc-add-1".into(),
        name: "add_token".into(),
        args: serde_json::json!({
            "id": "tok-resist",
            "name": "FireResistantOrc",
            "x": 0, "y": 0,
            "hp": 20, "max_hp": 20, "ac": 12,
            "resistances": ["fire"]
        }),
    };
    let (val, is_err) = execute_tool(
        &add_tc,
        &pool,
        None,
        None,
        None,
        "",
        campaign_id,
        session_id,
    )
    .await;
    assert!(!is_err, "add_token failed: {val}");

    // Apply 10 fire damage; resistant -> effective = 5.
    let dmg_tc = ToolCall {
        id: "tc-dmg-1".into(),
        name: "apply_damage".into(),
        args: serde_json::json!({
            "token_id": "tok-resist",
            "amount": 10,
            "type": "fire"
        }),
    };
    let (val, is_err) = execute_tool(
        &dmg_tc,
        &pool,
        None,
        None,
        None,
        "",
        campaign_id,
        session_id,
    )
    .await;
    assert!(!is_err, "apply_damage failed: {val}");
    assert_eq!(
        val["new_hp"].as_i64(),
        Some(15),
        "fire-resistant token: 20 - 5 = 15; got {val}"
    );
    assert_eq!(val["raw_damage"].as_i64(), Some(10));
    assert_eq!(val["effective_damage"].as_i64(), Some(5));
}

#[tokio::test]
async fn apply_damage_fire_immunity_deals_zero() {
    let (pool, session_id, campaign_id) = setup_encounter_pool().await;

    let add_tc = ToolCall {
        id: "tc-add-2".into(),
        name: "add_token".into(),
        args: serde_json::json!({
            "id": "tok-immune",
            "name": "FireImmuneDragon",
            "x": 0, "y": 0,
            "hp": 100, "max_hp": 100, "ac": 18,
            "immunities": ["fire"]
        }),
    };
    let (val, is_err) = execute_tool(
        &add_tc,
        &pool,
        None,
        None,
        None,
        "",
        campaign_id,
        session_id,
    )
    .await;
    assert!(!is_err, "add_token failed: {val}");

    let dmg_tc = ToolCall {
        id: "tc-dmg-2".into(),
        name: "apply_damage".into(),
        args: serde_json::json!({
            "token_id": "tok-immune",
            "amount": 10,
            "type": "fire"
        }),
    };
    let (val, is_err) = execute_tool(
        &dmg_tc,
        &pool,
        None,
        None,
        None,
        "",
        campaign_id,
        session_id,
    )
    .await;
    assert!(!is_err, "apply_damage failed: {val}");
    assert_eq!(
        val["new_hp"].as_i64(),
        Some(100),
        "fire-immune token: 100 - 0 = 100; got {val}"
    );
    assert_eq!(val["raw_damage"].as_i64(), Some(10));
    assert_eq!(val["effective_damage"].as_i64(), Some(0));
}

#[tokio::test]
async fn apply_damage_fire_vulnerability_doubles_damage() {
    let (pool, session_id, campaign_id) = setup_encounter_pool().await;

    let add_tc = ToolCall {
        id: "tc-add-3".into(),
        name: "add_token".into(),
        args: serde_json::json!({
            "id": "tok-vuln",
            "name": "FireVulnTroll",
            "x": 0, "y": 0,
            "hp": 30, "max_hp": 30, "ac": 11,
            "vulnerabilities": ["fire"]
        }),
    };
    let (val, is_err) = execute_tool(
        &add_tc,
        &pool,
        None,
        None,
        None,
        "",
        campaign_id,
        session_id,
    )
    .await;
    assert!(!is_err, "add_token failed: {val}");

    let dmg_tc = ToolCall {
        id: "tc-dmg-3".into(),
        name: "apply_damage".into(),
        args: serde_json::json!({
            "token_id": "tok-vuln",
            "amount": 10,
            "type": "fire"
        }),
    };
    let (val, is_err) = execute_tool(
        &dmg_tc,
        &pool,
        None,
        None,
        None,
        "",
        campaign_id,
        session_id,
    )
    .await;
    assert!(!is_err, "apply_damage failed: {val}");
    assert_eq!(
        val["new_hp"].as_i64(),
        Some(10),
        "fire-vulnerable token: 30 - 20 = 10; got {val}"
    );
    assert_eq!(val["raw_damage"].as_i64(), Some(10));
    assert_eq!(val["effective_damage"].as_i64(), Some(20));
}

#[tokio::test]
async fn apply_damage_no_relation_applies_full_damage() {
    let (pool, session_id, campaign_id) = setup_encounter_pool().await;

    let add_tc = ToolCall {
        id: "tc-add-4".into(),
        name: "add_token".into(),
        args: serde_json::json!({
            "id": "tok-plain",
            "name": "PlainGoblin",
            "x": 0, "y": 0,
            "hp": 20, "max_hp": 20, "ac": 10
        }),
    };
    let (val, is_err) = execute_tool(
        &add_tc,
        &pool,
        None,
        None,
        None,
        "",
        campaign_id,
        session_id,
    )
    .await;
    assert!(!is_err, "add_token failed: {val}");

    let dmg_tc = ToolCall {
        id: "tc-dmg-4".into(),
        name: "apply_damage".into(),
        args: serde_json::json!({
            "token_id": "tok-plain",
            "amount": 10,
            "type": "slashing"
        }),
    };
    let (val, is_err) = execute_tool(
        &dmg_tc,
        &pool,
        None,
        None,
        None,
        "",
        campaign_id,
        session_id,
    )
    .await;
    assert!(!is_err, "apply_damage failed: {val}");
    assert_eq!(
        val["new_hp"].as_i64(),
        Some(10),
        "no-resistance token: 20 - 10 = 10; got {val}"
    );
    assert_eq!(val["raw_damage"].as_i64(), Some(10));
    assert_eq!(val["effective_damage"].as_i64(), Some(10));
}
