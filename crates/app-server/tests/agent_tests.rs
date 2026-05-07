use app_llm::{ChatChunk, FinishReason, MockProvider};
use app_server::agent::orchestrator::AgentEvent;
use app_server::agent::orchestrator::{AgentConfig, AgentOrchestrator, AgentTurnRequest};
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
