use app_llm::{
    Capabilities, ChatChunk, ChatRequest, ChunkStream, FinishReason, LlmError, LlmProvider,
};
use app_server::test_support::TestServer;
use futures::StreamExt;
use reqwest::Client;
use serde_json::json;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use uuid::Uuid;

struct Endpoint {
    origin: &'static str,
    calls: AtomicUsize,
}
#[async_trait::async_trait]
impl LlmProvider for Endpoint {
    async fn stream_chat(&self, request: ChatRequest) -> Result<ChunkStream, LlmError> {
        assert_eq!(
            request.model, "model-a",
            "old request must never use model B"
        );
        let first = self.calls.fetch_add(1, Ordering::SeqCst) == 0;
        Ok(futures::stream::iter(vec![
            Ok(ChatChunk::TextDelta {
                text: if first {
                    "Private unfinished draft"
                } else {
                    "Public continuation"
                }
                .into(),
            }),
            Ok(ChatChunk::Done {
                reason: if first {
                    FinishReason::Length
                } else {
                    FinishReason::Stop
                },
            }),
        ])
        .boxed())
    }
    fn name(&self) -> &'static str {
        "recovery-fixture"
    }
    fn execution_origin(&self) -> Option<String> {
        Some(self.origin.into())
    }
    fn active_model(&self) -> &str {
        "model-a"
    }
    fn capabilities_for_model(&self, _: &str) -> Capabilities {
        Capabilities::default()
    }
}

async fn start() -> (TestServer, Arc<Endpoint>, serde_json::Value) {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    app_server::db::init_db(&pool).await.unwrap();
    let provider = Arc::new(Endpoint {
        origin: "endpoint-a",
        calls: AtomicUsize::new(0),
    });
    let server = TestServer::start_with(provider.clone(), pool).await;
    let mut config = server.state.agent_config();
    config.model = "model-a".into();
    server.state.set_agent_config(config);
    let body = json!({"request_id":Uuid::new_v4(),"campaign_id":Uuid::new_v4(),"session_id":Uuid::new_v4(),"player_message":"Look","history":[]});
    let response = Client::new()
        .post(server.url("/agent/turn"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    assert!(!response.text().await.unwrap().contains("event: agent_done"));
    (server, provider, body)
}
async fn blocked(server: &TestServer, body: &serde_json::Value, code: &str) {
    let response = Client::new()
        .post(server.url("/agent/turn"))
        .json(body)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 409);
    assert_eq!(
        response.json::<serde_json::Value>().await.unwrap()["error"]["code"],
        code
    );
    let status: String =
        sqlx::query_scalar("SELECT status FROM agent_turn_operations WHERE request_id=?1")
            .bind(body["request_id"].as_str().unwrap())
            .fetch_one(server.state.db())
            .await
            .unwrap();
    assert_eq!(
        status, "interrupted",
        "a rejected environment remains recoverable"
    );
}

#[tokio::test]
async fn model_and_endpoint_changes_block_before_inference_and_restoring_a_resumes_once() {
    let (server, original, mut body) = start().await;
    body["resume"] = json!(true);
    let mut config = server.state.agent_config();
    config.model = "model-b".into();
    server.state.set_agent_config(config.clone());
    blocked(&server, &body, "agent_original_environment_changed").await;
    assert_eq!(original.calls.load(Ordering::SeqCst), 1);
    config.model = "model-a".into();
    server.state.set_agent_config(config);
    let other = Arc::new(Endpoint {
        origin: "endpoint-b",
        calls: AtomicUsize::new(0),
    });
    server.state.set_provider(other.clone());
    blocked(&server, &body, "agent_original_environment_changed").await;
    assert_eq!(other.calls.load(Ordering::SeqCst), 0);
    server.state.set_provider(original.clone());
    let response = Client::new()
        .post(server.url("/agent/turn"))
        .json(&body)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(response.contains("Public continuation"));
    assert!(response.contains("event: agent_done"));
    assert!(!response.contains("Private unfinished draft"));
    assert_eq!(original.calls.load(Ordering::SeqCst), 2);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE session_id=?1")
        .bind(body["session_id"].as_str().unwrap())
        .fetch_one(server.state.db())
        .await
        .unwrap();
    assert_eq!(
        count, 2,
        "one original human message plus one public response"
    );
}

#[tokio::test]
async fn legacy_missing_provider_identity_stays_blocked_without_guessing() {
    let (server, original, mut body) = start().await;
    sqlx::query("UPDATE agent_turn_operations SET execution_provider=NULL,execution_model=NULL")
        .execute(server.state.db())
        .await
        .unwrap();
    body["resume"] = json!(true);
    blocked(&server, &body, "agent_original_environment_unavailable").await;
    assert_eq!(original.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn original_environment_receipt_survives_backend_recomposition() {
    let (server, original, mut body) = start().await;
    let next = TestServer::start_with(original.clone(), server.state.db().clone()).await;
    let mut config = next.state.agent_config();
    config.model = "model-a".into();
    next.state.set_agent_config(config);
    body["resume"] = json!(true);
    let response = Client::new()
        .post(next.url("/agent/turn"))
        .json(&body)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(response.contains("event: agent_done"));
    assert_eq!(original.calls.load(Ordering::SeqCst), 2);
}
