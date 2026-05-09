//! F.1: GET /sessions/{id}/messages integration tests.

use std::sync::Arc;

use app_llm::{ChatMessage, MessagePart, MockProvider};
use app_server::db;
use app_server::test_support::TestServer;
use reqwest::Client;
use serde_json::Value;

#[tokio::test]
async fn list_messages_returns_inserted_history_in_order() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    db::init_db(&pool).await.unwrap();

    let session_id = uuid::Uuid::new_v4().to_string();
    db::insert_message(&pool, &session_id, &ChatMessage::user_text("first"))
        .await
        .unwrap();
    db::insert_message(
        &pool,
        &session_id,
        &ChatMessage::Assistant {
            content: "ok".into(),
        },
    )
    .await
    .unwrap();

    let server = TestServer::start_with(Arc::new(MockProvider::new(vec![])), pool).await;
    let body = Client::new()
        .get(server.url(&format!("/sessions/{session_id}/messages")))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let v: Value = serde_json::from_str(&body).unwrap();
    let arr = v["messages"].as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["role"], "user");
    assert_eq!(arr[1]["role"], "assistant");
}

#[tokio::test]
async fn list_messages_empty_session_returns_empty_array() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    db::init_db(&pool).await.unwrap();
    let server = TestServer::start_with(Arc::new(MockProvider::new(vec![])), pool).await;

    let body = Client::new()
        .get(server.url("/sessions/00000000-0000-0000-0000-000000000099/messages"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["messages"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn list_messages_round_trips_image_part() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    db::init_db(&pool).await.unwrap();
    let session_id = uuid::Uuid::new_v4().to_string();
    let msg = ChatMessage::User {
        parts: vec![
            MessagePart::Text {
                text: "see this".into(),
            },
            MessagePart::Image {
                mime: "image/png".into(),
                data_b64: "aGk=".into(),
                name: Some("p.png".into()),
            },
        ],
    };
    db::insert_message(&pool, &session_id, &msg).await.unwrap();
    let server = TestServer::start_with(Arc::new(MockProvider::new(vec![])), pool).await;

    let body = Client::new()
        .get(server.url(&format!("/sessions/{session_id}/messages")))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let v: Value = serde_json::from_str(&body).unwrap();
    let parts = v["messages"][0]["parts"].as_array().unwrap();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0]["type"], "text");
    assert_eq!(parts[1]["type"], "image");
    assert_eq!(parts[1]["mime"], "image/png");
    assert_eq!(parts[1]["data_b64"], "aGk=");
}
