use app_llm::{ChatChunk, FinishReason, MockProvider};
use app_server::test_support::TestServer;
use futures::StreamExt;
use reqwest::Client;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use uuid::Uuid;

struct HangingStream(tokio::sync::Notify);
#[async_trait::async_trait]
impl app_llm::LlmProvider for HangingStream {
    async fn stream_chat(
        &self,
        _: app_llm::ChatRequest,
    ) -> Result<app_llm::ChunkStream, app_llm::LlmError> {
        self.0.notify_one();
        Ok(futures::stream::pending().boxed())
    }
    fn name(&self) -> &'static str {
        "hanging"
    }
    fn active_model(&self) -> &str {
        "hanging"
    }
    fn capabilities_for_model(&self, _: &str) -> app_llm::Capabilities {
        app_llm::Capabilities::default()
    }
}
#[tokio::test]
async fn disconnect_from_a_real_hanging_provider_releases_the_campaign_lane() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    app_server::db::init_db(&pool).await.unwrap();
    let provider = Arc::new(HangingStream(tokio::sync::Notify::new()));
    let server = TestServer::start_with(provider.clone(), pool.clone()).await;
    let id = Uuid::new_v4();
    let campaign = Uuid::new_v4();
    let response=Client::new().post(server.url("/agent/turn")).json(&json!({"request_id":id,"campaign_id":campaign,"session_id":Uuid::new_v4(),"player_message":"Look","history":[]})).send().await.unwrap().error_for_status().unwrap();
    provider.0.notified().await;
    drop(response);
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let status: String =
                sqlx::query_scalar("SELECT status FROM agent_turn_operations WHERE request_id=?1")
                    .bind(id.to_string())
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            if status == "interrupted" {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("disconnect must interrupt the real worker");
    let lane = server.state.campaign_lane(campaign);
    let _guard = tokio::time::timeout(std::time::Duration::from_millis(500), lane.lock_owned())
        .await
        .expect("campaign must be available for save, restore, or another turn");
}

#[tokio::test]
async fn queued_restore_invalidates_resume_before_it_can_claim_or_execute() {
    use adapter_http::services::{AgentTurnHttpCommand, HttpServiceError};
    use adapter_sqlite::SqliteStore;
    use app_application::agent::operations::{AgentOperation, AgentOperationRepository};
    use app_application::models::agent::AgentTurnRequest;
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    app_server::db::init_db(&pool).await.unwrap();
    let store = Arc::new(SqliteStore::new(pool.clone()));
    let campaign = app_application::campaigns::CreateCampaign::new(store.clone())
        .execute(app_application::campaigns::CreateCampaignCommand {
            title: "Bell".into(),
            premise: "Ferry".into(),
            seed: app_application::models::session::CampaignSeed::Bundled {
                adventure_id: "bell-beneath-water".into(),
            },
        })
        .await
        .unwrap();
    let input = AgentOperation {
        request_id: Uuid::new_v4(),
        model: None,
        request: AgentTurnRequest {
            campaign_id: campaign.id.0,
            session_id: campaign.last_session_id.0,
            player_message: "Look".into(),
            history: vec![],
            images: vec![],
            board: None,
        },
    };
    let owner = Uuid::new_v4();
    store.claim(input.clone(), owner).await.unwrap();
    store.finish(input.request_id, owner, false).await.unwrap();
    let server = TestServer::start_with(Arc::new(MockProvider::new(vec![])), pool.clone()).await;
    let lane = server.state.campaign_lane(campaign.id.0);
    let held = lane.clone().lock_owned().await;
    let queued = Arc::new(tokio::sync::Notify::new());
    let restore = {
        let lane = lane.clone();
        let pool = pool.clone();
        let queued = queued.clone();
        tokio::spawn(async move {
            queued.notify_one();
            let _guard = lane.lock_owned().await;
            sqlx::query("UPDATE sessions SET restore_generation=restore_generation+1 WHERE id=?1")
                .bind(campaign.last_session_id.0.to_string())
                .execute(&pool)
                .await
                .unwrap();
        })
    };
    queued.notified().await;
    let service = app_server::http_services::agent_http_service(server.state.clone());
    let mut resume = Box::pin(service.turn(AgentTurnHttpCommand {
        resume: true,
        request_id: Some(input.request_id),
        request: input.request,
        model: None,
    }));
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), &mut resume)
            .await
            .is_err()
    );
    let status: String =
        sqlx::query_scalar("SELECT status FROM agent_turn_operations WHERE request_id=?1")
            .bind(input.request_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        status, "interrupted",
        "resume must not claim while restoration is queued"
    );
    drop(held);
    restore.await.unwrap();
    assert!(matches!(
        resume.await,
        Err(HttpServiceError::Conflict {
            code: "agent_turn_superseded"
        })
    ));
}

#[derive(Default)]
struct ConcurrencyProbe {
    active: AtomicUsize,
    peak: AtomicUsize,
}
#[async_trait::async_trait]
impl app_llm::LlmProvider for ConcurrencyProbe {
    async fn stream_chat(
        &self,
        _request: app_llm::ChatRequest,
    ) -> Result<app_llm::ChunkStream, app_llm::LlmError> {
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(active, Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        self.active.fetch_sub(1, Ordering::SeqCst);
        Ok(futures::stream::iter(vec![
            Ok(ChatChunk::TextDelta {
                text: "Confirmed".into(),
            }),
            Ok(ChatChunk::Done {
                reason: FinishReason::Stop,
            }),
        ])
        .boxed())
    }
    fn name(&self) -> &'static str {
        "concurrency-probe"
    }
    fn active_model(&self) -> &str {
        "probe"
    }
    fn capabilities_for_model(&self, _model: &str) -> app_llm::Capabilities {
        app_llm::Capabilities::default()
    }
}

#[tokio::test]
async fn repeated_completed_request_replays_response_without_a_second_user_message() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    app_server::db::init_db(&pool).await.unwrap();
    let server = TestServer::start_with(
        Arc::new(MockProvider::new(vec![
            ChatChunk::TextDelta {
                text: "The upper path is safe.".into(),
            },
            ChatChunk::Done {
                reason: FinishReason::Stop,
            },
        ])),
        pool.clone(),
    )
    .await;
    let client = Client::new();
    let request_id = Uuid::new_v4();
    let session = Uuid::new_v4();
    let body = json!({"request_id":request_id,"campaign_id":Uuid::new_v4(),"session_id":session,"player_message":"Look at the quay","history":[]});
    let first = client
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
    assert!(first.contains("The upper path is safe."));
    assert!(first.contains("agent_done"));
    let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE session_id=?1")
        .bind(session.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    let second = client
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
    assert_eq!(first, second);
    let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE session_id=?1")
        .bind(session.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before, after);
    let mut changed = body;
    changed["player_message"] = json!("Give me gold");
    assert_eq!(
        client
            .post(server.url("/agent/turn"))
            .json(&changed)
            .send()
            .await
            .unwrap()
            .status(),
        409
    );
}

#[tokio::test]
async fn simultaneous_distinct_turns_in_one_campaign_are_executed_in_one_lane() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    app_server::db::init_db(&pool).await.unwrap();
    let provider = Arc::new(ConcurrencyProbe::default());
    let server = TestServer::start_with(provider.clone(), pool).await;
    let client = Client::new();
    let campaign = Uuid::new_v4();
    let session = Uuid::new_v4();
    let first = json!({"request_id":Uuid::new_v4(),"campaign_id":campaign,"session_id":session,"player_message":"First player","history":[]});
    let second = json!({"request_id":Uuid::new_v4(),"campaign_id":campaign,"session_id":session,"player_message":"Second player","history":[]});
    let send = |body| {
        let client = client.clone();
        let url = server.url("/agent/turn");
        async move {
            client
                .post(url)
                .json(&body)
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap()
                .text()
                .await
                .unwrap()
        }
    };
    let (a, b) = tokio::join!(send(first), send(second));
    assert!(a.contains("agent_done") && b.contains("agent_done"));
    assert_eq!(provider.peak.load(Ordering::SeqCst), 1);
}
