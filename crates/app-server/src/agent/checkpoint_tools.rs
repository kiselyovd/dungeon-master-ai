use adapter_sqlite::SqliteStore;
use app_application::agent::operations::ToolStepClaim;
use app_application::agent::tool_dispatch::{
    ToolCapabilityHandler, ToolDispatchError, ToolDispatchRequest, ToolExecution,
};
use app_application::ports::repositories::{SessionRepository, SnapshotRepository};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

pub(super) struct CheckpointTools<T> {
    pub inner: Arc<T>,
    pub store: SqliteStore,
}

#[async_trait]
impl<T: ToolCapabilityHandler> ToolCapabilityHandler for CheckpointTools<T> {
    async fn execute_capability(
        &self,
        request: ToolDispatchRequest,
    ) -> Result<ToolExecution, ToolDispatchError> {
        let request_id = Uuid::parse_str(&request.request_id).map_err(|_| ToolDispatchError {
            code: "agent_request_invalid",
        })?;
        let registered: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM agent_turn_operations WHERE request_id=?1)",
        )
        .bind(request_id.to_string())
        .fetch_one(self.store.pool())
        .await
        .map_err(|_| ToolDispatchError {
            code: "tool_checkpoint_failed",
        })?;
        if !registered {
            return self.inner.execute_capability(request).await;
        }
        let name = request.command.tool_name();
        let command = json!({"tool_name":name,"args":request.command.to_args()});
        let claim = self
            .store
            .claim_tool_step(request_id, &request.tool_call_id, command)
            .await
            .map_err(|_| ToolDispatchError {
                code: "tool_checkpoint_failed",
            })?;
        if let ToolStepClaim::Completed(result) = claim {
            return Ok(result);
        }
        let pending = matches!(claim, ToolStepClaim::Pending);
        let result = if pending
            && !matches!(
                name,
                "adventure_action" | "quick_save" | "roll_dice" | "query_rules" | "recall_npc"
            ) {
            ToolExecution {
                result: json!({"error":"tool_recovery_requires_fresh_intention"}),
                is_error: true,
                handled_by: "engine".into(),
                media: vec![],
            }
        } else if name == "quick_save" {
            let session = self
                .store
                .get(app_application::models::identity::SessionId(
                    request.session_id,
                ))
                .await
                .map_err(|_| ToolDispatchError {
                    code: "save_session_failed",
                })?
                .ok_or(ToolDispatchError {
                    code: "save_session_missing",
                })?;
            if session.campaign_id.0 != request.campaign_id {
                return Err(ToolDispatchError {
                    code: "save_session_mismatch",
                });
            }
            let saved = self
                .store
                .create_snapshot(app_application::models::snapshot::CreateSnapshotRecord {
                    id: Uuid::new_v4(),
                    campaign_id: session.campaign_id,
                    session_id: session.id,
                    expected_revision: session.revision,
                    schema_version: 1,
                    ruleset_id: "srd-5.2.1".into(),
                    title: "Quick save".into(),
                    kind: "auto".into(),
                    summary: String::new(),
                    tag: "exploration".into(),
                    causation_id: format!("agent:{}:{}", request_id, request.tool_call_id),
                    character: json!({}),
                })
                .await
                .map_err(|_| ToolDispatchError {
                    code: "save_checkpoint_failed",
                })?;
            ToolExecution {
                result: json!({"snapshot_id":saved.snapshot.id,"save_id":saved.save_id,"revision":saved.snapshot.revision.0,"campaign_id":session.campaign_id.0,"session_id":session.id.0}),
                is_error: false,
                handled_by: "engine".into(),
                media: vec![],
            }
        } else {
            self.inner.execute_capability(request.clone()).await?
        };
        self.store
            .finish_tool_step(request_id, &request.tool_call_id, &result)
            .await
            .map_err(|_| ToolDispatchError {
                code: "tool_checkpoint_failed",
            })?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_application::agent::commands::{AgentToolCommand, RollDiceCommand};
    use app_application::agent::operations::{AgentOperation, AgentOperationRepository};
    use app_application::models::agent::AgentTurnRequest;
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct CountingDice(Arc<AtomicUsize>);
    #[async_trait]
    impl ToolCapabilityHandler for CountingDice {
        async fn execute_capability(
            &self,
            _request: ToolDispatchRequest,
        ) -> Result<ToolExecution, ToolDispatchError> {
            let roll = self.0.fetch_add(1, Ordering::SeqCst) + 1;
            Ok(ToolExecution {
                result: json!({"roll":roll}),
                is_error: false,
                handled_by: "engine".into(),
                media: vec![],
            })
        }
    }
    #[tokio::test]
    async fn replay_uses_recorded_roll_instead_of_invoking_the_dice_handler_again() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        crate::db::init_db(&pool).await.unwrap();
        let store = SqliteStore::new(pool);
        let id = Uuid::new_v4();
        let campaign = Uuid::new_v4();
        let session = Uuid::new_v4();
        store
            .claim(
                AgentOperation {
                    request_id: id,
                    request: AgentTurnRequest {
                        campaign_id: campaign,
                        session_id: session,
                        player_message: "Roll".into(),
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
        let calls = Arc::new(AtomicUsize::new(0));
        let handler = CheckpointTools {
            inner: Arc::new(CountingDice(calls.clone())),
            store,
        };
        let request = ToolDispatchRequest {
            request_id: id.to_string(),
            tool_call_id: "roll-1".into(),
            campaign_id: campaign,
            session_id: session,
            command: AgentToolCommand::RollDice(RollDiceCommand {
                dice: "1d20".into(),
                modifier: None,
                advantage: None,
                reason: None,
            }),
        };
        let first = handler.execute_capability(request.clone()).await.unwrap();
        let second = handler.execute_capability(request).await.unwrap();
        assert_eq!(first, second);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
