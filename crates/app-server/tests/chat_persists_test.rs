use std::sync::Arc;

use app_llm::{ChatChunk, ChatMessage, FinishReason, MockProvider};
use app_server::db;
use app_server::test_support::TestServer;
use futures::StreamExt;
use reqwest::Client;
use serde_json::json;

#[tokio::test]
async fn chat_persists_user_and_assistant_messages_when_session_id_provided() {
    let provider = Arc::new(MockProvider::new(vec![
        ChatChunk::TextDelta {
            text: "Hello, ".into(),
        },
        ChatChunk::TextDelta {
            text: "world".into(),
        },
        ChatChunk::Done {
            reason: FinishReason::Stop,
        },
    ]));
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("in-memory db");
    db::init_db(&pool).await.expect("migrate");
    let server = TestServer::start_with(provider, pool.clone()).await;

    let session_id = uuid::Uuid::new_v4().to_string();
    let body = json!({
        "session_id": session_id,
        "messages": [{"role": "user", "content": "hi"}],
        "model": "mock"
    });

    let resp = Client::new()
        .post(server.url("/chat"))
        .json(&body)
        .send()
        .await
        .expect("post");
    assert_eq!(resp.status(), 200);

    // Drain the stream so the Done handler fires + spawns the assistant
    // persist task.
    let mut s = resp.bytes_stream();
    while let Some(c) = s.next().await {
        let _ = c;
    }
    // The Done handler spawns a tokio task to write the assistant message;
    // give it a beat to finish.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let history = db::list_messages_by_session(&pool, &session_id)
        .await
        .unwrap();
    assert_eq!(history.len(), 2, "expected user + assistant rows");
    assert!(matches!(&history[0], ChatMessage::User { .. }));
    assert!(matches!(&history[1], ChatMessage::Assistant { content } if content == "Hello, world"));
}

#[tokio::test]
async fn chat_does_not_persist_when_session_id_omitted() {
    let provider = Arc::new(MockProvider::new(vec![
        ChatChunk::TextDelta {
            text: "ok".into(),
        },
        ChatChunk::Done {
            reason: FinishReason::Stop,
        },
    ]));
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("in-memory db");
    db::init_db(&pool).await.expect("migrate");
    let server = TestServer::start_with(provider, pool.clone()).await;

    let body = json!({
        "messages": [{"role": "user", "content": "hi"}],
        "model": "mock"
    });

    let resp = Client::new()
        .post(server.url("/chat"))
        .json(&body)
        .send()
        .await
        .expect("post");
    assert_eq!(resp.status(), 200);
    let mut s = resp.bytes_stream();
    while let Some(c) = s.next().await {
        let _ = c;
    }
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Since no session_id was provided, no rows should exist for any session.
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}
