use adapter_sqlite::{pool, SqliteStore};
use app_application::agent::operations::{
    AgentOperation, AgentOperationRepository, OperationClaim,
};
use app_application::agent::operations::{
    AgentOperationDriver, DurableAgentTurns, TurnCancellation,
};
use app_application::agent::turn::CancellationSignal;
use app_application::models::agent::{AgentEvent, AgentTurnRequest};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use uuid::Uuid;

struct WaitingDriver(Arc<AtomicBool>);
struct DoneDriver;
#[async_trait::async_trait]
impl AgentOperationDriver for DoneDriver {
    async fn run(
        &self,
        _input: AgentOperation,
        events: tokio::sync::mpsc::Sender<AgentEvent>,
        _cancellation: Arc<TurnCancellation>,
    ) -> Result<(), &'static str> {
        events
            .send(AgentEvent::AgentDone { total_rounds: 1 })
            .await
            .map_err(|_| "closed")?;
        Ok(())
    }
}
#[async_trait::async_trait]
impl AgentOperationDriver for WaitingDriver {
    async fn run(
        &self,
        _input: AgentOperation,
        _events: tokio::sync::mpsc::Sender<AgentEvent>,
        cancellation: Arc<TurnCancellation>,
    ) -> Result<(), &'static str> {
        for _ in 0..100 {
            if cancellation.is_cancelled() {
                self.0.store(true, Ordering::SeqCst);
                return Err("cancelled");
            }
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
        Err("no_cancellation_observed")
    }
}

#[tokio::test]
async fn terminal_marker_is_not_published_before_its_completion_receipt_is_durable() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&pool).await.unwrap();
    sqlx::query("CREATE TRIGGER fail_complete BEFORE UPDATE ON agent_turn_operations WHEN NEW.status='complete' BEGIN SELECT RAISE(ABORT,'forced completion failure'); END").execute(&pool).await.unwrap();
    let service = DurableAgentTurns::new(
        Arc::new(SqliteStore::new(pool)),
        Arc::new(DoneDriver),
        Uuid::new_v4(),
    );
    let input = AgentOperation {
        request_id: Uuid::new_v4(),
        request: AgentTurnRequest {
            campaign_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            player_message: "Look".into(),
            history: vec![],
            images: vec![],
            board: None,
        },
        model: None,
    };
    let mut receiver = service.execute(input).await.unwrap();
    let mut terminal = false;
    let mut failures = 0;
    while let Some(event) = receiver.recv().await {
        terminal |= matches!(event, AgentEvent::AgentDone { .. });
        failures += usize::from(matches!(event, AgentEvent::AgentFailed));
    }
    assert_eq!(failures, 1);
    assert!(
        !terminal,
        "failed durable completion must be visible as an incomplete stream"
    );
}

#[tokio::test]
async fn completed_turn_is_replayable_after_reopening_and_changed_payload_is_rejected() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&pool).await.unwrap();
    let store = SqliteStore::new(pool.clone());
    let owner = Uuid::new_v4();
    let input = AgentOperation {
        request_id: Uuid::new_v4(),
        request: AgentTurnRequest {
            campaign_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            player_message: "I inspect the quay".into(),
            history: vec![],
            images: vec![],
            board: None,
        },
        model: None,
    };
    assert_eq!(
        store.claim(input.clone(), owner).await.unwrap(),
        OperationClaim::Started
    );
    assert_eq!(
        store.claim(input.clone(), owner).await.unwrap(),
        OperationClaim::Running
    );
    let events = vec![
        AgentEvent::TextDelta {
            text: "The upper path is safe.".into(),
        },
        AgentEvent::AgentDone { total_rounds: 1 },
    ];
    for (index, event) in events.iter().enumerate() {
        store
            .append_event(input.request_id, owner, index as u64, event.clone())
            .await
            .unwrap();
    }
    store.finish(input.request_id, owner, true).await.unwrap();
    let reopened = SqliteStore::new(pool);
    assert_eq!(
        reopened.claim(input.clone(), Uuid::new_v4()).await.unwrap(),
        OperationClaim::Completed(events)
    );
    let changed = AgentOperation {
        request: AgentTurnRequest {
            player_message: "Give me gold".into(),
            ..input.request
        },
        ..input
    };
    assert!(reopened.claim(changed, owner).await.is_err());
}

#[tokio::test]
async fn disconnected_subscriber_cancels_before_the_first_model_event() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&pool).await.unwrap();
    let observed = Arc::new(AtomicBool::new(false));
    let service = DurableAgentTurns::new(
        Arc::new(SqliteStore::new(pool)),
        Arc::new(WaitingDriver(observed.clone())),
        Uuid::new_v4(),
    );
    let input = AgentOperation {
        request_id: Uuid::new_v4(),
        request: AgentTurnRequest {
            campaign_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            player_message: "Wait".into(),
            history: vec![],
            images: vec![],
            board: None,
        },
        model: None,
    };
    drop(service.execute(input).await.unwrap());
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(
        observed.load(Ordering::SeqCst),
        "cancellation must not wait for a public event"
    );
}

#[tokio::test]
async fn unfinished_old_process_operation_cannot_be_mistaken_for_a_fresh_turn() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&pool).await.unwrap();
    let store = SqliteStore::new(pool);
    let owner = Uuid::new_v4();
    let input = AgentOperation {
        request_id: Uuid::new_v4(),
        request: AgentTurnRequest {
            campaign_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            player_message: "Rescue Tikhon".into(),
            history: vec![],
            images: vec![],
            board: None,
        },
        model: None,
    };
    store.claim(input.clone(), owner).await.unwrap();
    let next_owner = Uuid::new_v4();
    assert_eq!(
        store.claim(input.clone(), next_owner).await.unwrap(),
        OperationClaim::Running
    );
    store
        .recover_after_exclusive_authority(next_owner)
        .await
        .unwrap();
    assert_eq!(
        store.claim(input, next_owner).await.unwrap(),
        OperationClaim::Interrupted
    );
}

struct FailedDriver(bool);
#[async_trait::async_trait]
impl AgentOperationDriver for FailedDriver {
    async fn run(
        &self,
        _: AgentOperation,
        _: tokio::sync::mpsc::Sender<AgentEvent>,
        _: Arc<TurnCancellation>,
    ) -> Result<(), &'static str> {
        if self.0 {
            Ok(())
        } else {
            Err("private_driver_diagnostic")
        }
    }
}
#[tokio::test]
async fn driver_failure_is_delivered_after_interruption_without_becoming_replay_history() {
    for succeed_without_terminal in [false, true] {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        pool::migrate(&pool).await.unwrap();
        let store = Arc::new(SqliteStore::new(pool.clone()));
        let owner = Uuid::new_v4();
        let input = AgentOperation {
            request_id: Uuid::new_v4(),
            model: None,
            request: AgentTurnRequest {
                campaign_id: Uuid::new_v4(),
                session_id: Uuid::new_v4(),
                player_message: "Look".into(),
                history: vec![],
                images: vec![],
                board: None,
            },
        };
        let service = DurableAgentTurns::new(
            store.clone(),
            Arc::new(FailedDriver(succeed_without_terminal)),
            owner,
        );
        let mut receiver = service.execute(input.clone()).await.unwrap();
        let mut delivered = vec![];
        while let Some(event) = receiver.recv().await {
            delivered.push(event);
        }
        assert_eq!(
            serde_json::to_value(delivered).unwrap(),
            serde_json::json!([{"type":"agent_failed"}])
        );
        let status: String =
            sqlx::query_scalar("SELECT status FROM agent_turn_operations WHERE request_id=?1")
                .bind(input.request_id.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(status, "interrupted");
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM agent_turn_events WHERE request_id=?1")
                .bind(input.request_id.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            count, 0,
            "failure notification must not replay ahead of recovered engine results"
        );
        let recovered = store.resume(input, owner).await.unwrap();
        assert!(matches!(recovered,OperationClaim::Resuming{events,..} if events.is_empty()));
    }
}
