use app_llm::{ChatChunk, FinishReason, MockProvider, ToolCall};
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
        ChatChunk::TextDelta { text: "Hello ".into() },
        ChatChunk::TextDelta { text: "adventurer.".into() },
        ChatChunk::Done { reason: FinishReason::Stop },
    ]));

    let config = AgentConfig {
        model: "mock".into(),
        system_prompt: "You are a DM.".into(),
        temperature: 0.7,
        max_rounds: 8,
        embedding_model: "multilingual-e5-small".into(),
    };

    let req = AgentTurnRequest {
        campaign_id: uuid::Uuid::new_v4(),
        session_id: uuid::Uuid::new_v4(),
        player_message: "I search the room.".into(),
        history: vec![],
    };

    let (tx, mut rx) = mpsc::channel::<AgentEvent>(32);
    let orch = AgentOrchestrator::new(mock, pool, config, None);
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
    };
    let req = AgentTurnRequest {
        campaign_id: uuid::Uuid::new_v4(),
        session_id: uuid::Uuid::new_v4(),
        player_message: "roll".into(),
        history: vec![],
    };

    let (tx, mut rx) = mpsc::channel::<AgentEvent>(32);

    let orch = AgentOrchestrator::new(mock, pool, config, None);
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
        ChatChunk::ToolCallStart { id: "c2".into(), name: "fly_dragon".into() },
        ChatChunk::ToolCallArgsDelta {
            id: "c2".into(),
            args_fragment: r#"{"title":"The Tavern","mode":"exploration"}"#.into(),
        },
        ChatChunk::ToolCallDone { id: "c2".into() },
        ChatChunk::Done { reason: FinishReason::ToolUse },
    ]));

    let config = AgentConfig {
        model: "mock".into(),
        system_prompt: "DM".into(),
        temperature: 0.7,
        max_rounds: 8,
        embedding_model: "multilingual-e5-small".into(),
    };
    let req = AgentTurnRequest {
        campaign_id: uuid::Uuid::new_v4(),
        session_id: uuid::Uuid::new_v4(),
        player_message: "look around".into(),
        history: vec![],
    };

    let (tx, mut rx) = mpsc::channel::<AgentEvent>(32);
    let orch = AgentOrchestrator::new(mock, pool, config, None);
    orch.run(req, tx).await.unwrap();

    let mut got_error_result = false;
    let mut got_done = false;
    while let Some(ev) = rx.recv().await {
        match ev {
            AgentEvent::ToolCallResult { tool_name, is_error, .. } => {
                if tool_name == "fly_dragon" && is_error {
                    got_error_result = true;
                }
            }
            AgentEvent::AgentDone { .. } => {
                got_done = true;
                break;
            }
            _ => {}
        }
    }
    assert!(got_error_result, "expected is_error=true result for unknown tool");
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
    assert!(body.contains("tool_call_start"), "missing tool_call_start in: {body}");
    assert!(body.contains("tool_call_result"), "missing tool_call_result in: {body}");
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
    let (val, is_err) = execute_tool(&tc, &pool, campaign_id, session_id).await;
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
    let (val, is_err) = execute_tool(&tc, &pool, campaign_id, session_id).await;
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

    let server = TestServer::start_with(
        std::sync::Arc::new(MockProvider::new(vec![])),
        pool,
    )
    .await;

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
