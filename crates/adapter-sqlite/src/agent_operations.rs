use crate::{messages::repository_error, SqliteStore};
use app_application::agent::operations::{
    AgentOperation, AgentOperationRepository, OperationClaim,
};
use app_application::models::agent::AgentEvent;
use app_application::ports::repositories::RepositoryError;
use async_trait::async_trait;
use sqlx::Row;
use uuid::Uuid;

fn rejected(code: &'static str) -> RepositoryError {
    RepositoryError::Operation {
        operation: "agent_operation",
        code,
    }
}

#[async_trait]
impl AgentOperationRepository for SqliteStore {
    async fn bind_environment(
        &self,
        request: Uuid,
        owner: Uuid,
        environment: &app_application::agent::operations::ExecutionEnvironment,
        resuming: bool,
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| repository_error("begin_agent_environment"))?;
        let row = sqlx::query("SELECT execution_provider,execution_model FROM agent_turn_operations WHERE request_id=?1 AND owner_id=?2 AND status='running'")
            .bind(request.to_string()).bind(owner.to_string()).fetch_optional(&mut *tx).await.map_err(|_| repository_error("read_agent_environment"))?.ok_or_else(|| rejected("agent_turn_superseded"))?;
        if resuming {
            super::agent_environment::validate(&row, environment)?;
        } else {
            sqlx::query("UPDATE agent_turn_operations SET execution_provider=?3,execution_model=?4 WHERE request_id=?1 AND owner_id=?2 AND execution_model IS NULL")
                .bind(request.to_string()).bind(owner.to_string()).bind(&environment.provider).bind(&environment.model).execute(&mut *tx).await.map_err(|_| repository_error("write_agent_environment"))?;
        }
        tx.commit()
            .await
            .map_err(|_| repository_error("commit_agent_environment"))?;
        Ok(())
    }
    async fn resume(
        &self,
        input: AgentOperation,
        owner: Uuid,
    ) -> Result<OperationClaim, RepositoryError> {
        let raw =
            serde_json::to_string(&input).map_err(|_| repository_error("encode_agent_resume"))?;
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| repository_error("begin_agent_resume"))?;
        let row=sqlx::query("SELECT request_json,status,restore_generation,checkpoint_version,actor_id FROM agent_turn_operations WHERE request_id=?1")
            .bind(input.request_id.to_string()).fetch_optional(&mut *tx).await.map_err(|_|repository_error("read_agent_resume"))?;
        let Some(row) = row else {
            drop(tx);
            return self.claim(input, owner).await;
        };
        if row.get::<String, _>("request_json") != raw {
            return Err(rejected("agent_request_mismatch"));
        }
        match row.get::<String, _>("status").as_str() {
            "complete" => {
                drop(tx);
                return self.claim(input, owner).await;
            }
            "running" => return Ok(OperationClaim::Running),
            "interrupted" => {}
            _ => return Err(rejected("agent_operation_state_invalid")),
        }
        if row.get::<i64, _>("checkpoint_version") != 1 {
            return Err(rejected("agent_turn_recovery_unavailable"));
        }
        let generation: Option<i64> =
            sqlx::query_scalar("SELECT restore_generation FROM sessions WHERE id=?1")
                .bind(input.request.session_id.to_string())
                .fetch_optional(&mut *tx)
                .await
                .map_err(|_| repository_error("read_agent_timeline"))?;
        if generation.unwrap_or(0) != row.get::<i64, _>("restore_generation") {
            return Err(rejected("agent_turn_superseded"));
        }
        let actor: Option<String> =
            sqlx::query_scalar("SELECT character_id FROM campaign_characters WHERE campaign_id=?1")
                .bind(input.request.campaign_id.to_string())
                .fetch_optional(&mut *tx)
                .await
                .map_err(|_| repository_error("read_agent_actor"))?;
        if actor != row.get::<Option<String>, _>("actor_id") {
            return Err(rejected("agent_turn_superseded"));
        }
        let messages: Vec<String> = sqlx::query_scalar(
            "SELECT message_json FROM agent_turn_messages WHERE request_id=?1 ORDER BY sequence",
        )
        .bind(input.request_id.to_string())
        .fetch_all(&mut *tx)
        .await
        .map_err(|_| repository_error("read_agent_checkpoint_messages"))?;
        let messages: Vec<app_application::models::chat::ChatMessage> = messages
            .iter()
            .map(|raw| {
                serde_json::from_str(raw)
                    .map_err(|_| repository_error("decode_agent_checkpoint_message"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT event_json FROM agent_turn_events WHERE request_id=?1 ORDER BY sequence",
        )
        .bind(input.request_id.to_string())
        .fetch_all(&mut *tx)
        .await
        .map_err(|_| repository_error("read_agent_checkpoint_events"))?;
        let mut events = rows
            .iter()
            .map(|raw| {
                serde_json::from_str::<AgentEvent>(raw)
                    .map_err(|_| repository_error("decode_agent_checkpoint_event"))
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter(|event| {
                !matches!(
                    event,
                    AgentEvent::TextDelta { .. }
                        | AgentEvent::ReasoningText { .. }
                        | AgentEvent::AgentDone { .. }
                        | AgentEvent::AgentNeedsInput { .. }
                )
            })
            .collect::<Vec<_>>();
        // A transcript result may commit before its delivery event. Repair the
        // delivery log from the immutable tool receipt, without executing it.
        use app_application::agent::tool_dispatch::{GeneratedAgentMedia, ToolExecution};
        use app_application::models::chat::ChatMessage;
        let mut round = 0;
        for message in &messages {
            if let ChatMessage::AssistantWithToolCalls { tool_calls, .. } = message {
                round += 1;
                for call in tool_calls {
                    if !messages.iter().any(|message| matches!(message, ChatMessage::ToolResult(result) if result.tool_call_id == call.id)) {
                        continue;
                    }
                    let id = format!("{}:{}", input.request_id, call.id);
                    let decoded = app_application::agent::tool_decoder::decode_tool_call(
                        &call.name, &call.args,
                    );
                    let args = decoded
                        .as_ref()
                        .map(|command| command.to_args())
                        .unwrap_or_else(|_| call.args.clone());
                    let signature =
                        serde_json::json!({"tool_name":call.name,"args":args}).to_string();
                    let recorded_signature: Option<String> = sqlx::query_scalar(
                        "SELECT command_json FROM agent_tool_steps WHERE request_id=?1 AND tool_call_id=?2",
                    )
                    .bind(input.request_id.to_string()).bind(&call.id)
                    .fetch_optional(&mut *tx).await
                    .map_err(|_| repository_error("read_confirmed_tool_delivery"))?;
                    if recorded_signature.is_some_and(|recorded| recorded != signature) {
                        return Err(rejected("agent_tool_signature_mismatch"));
                    }
                    let raw: Option<String> = sqlx::query_scalar("SELECT result_json FROM agent_tool_steps WHERE request_id=?1 AND tool_call_id=?2 AND result_json IS NOT NULL")
                        .bind(input.request_id.to_string()).bind(&call.id).fetch_optional(&mut *tx).await.map_err(|_|repository_error("read_confirmed_tool_delivery"))?;
                    let execution: ToolExecution = if let Some(raw) = raw {
                        serde_json::from_str(&raw)
                            .map_err(|_| repository_error("decode_confirmed_tool_delivery"))?
                    } else {
                        let transcript = messages
                            .iter()
                            .find_map(|message| match message {
                                ChatMessage::ToolResult(result)
                                    if result.tool_call_id == call.id =>
                                {
                                    Some(result)
                                }
                                _ => None,
                            })
                            .ok_or_else(|| repository_error("confirmed_tool_receipt_missing"))?;
                        let value: serde_json::Value = serde_json::from_str(&transcript.content)
                            .map_err(|_| repository_error("decode_confirmed_tool_delivery"))?;
                        let mut recovered = None;
                        if let Ok(command) = &decoded {
                            if matches!(
                                call.name.as_str(),
                                "adventure_action"
                                    | "combat_action"
                                    | "rest_action"
                                    | "roll_dice"
                                    | "end_combat"
                            ) {
                                let request =
                                    app_application::agent::tool_dispatch::ToolDispatchRequest {
                                        request_id: input.request_id.to_string(),
                                        tool_call_id: call.id.clone(),
                                        campaign_id: input.request.campaign_id,
                                        session_id: input.request.session_id,
                                        command: command.clone(),
                                    };
                                recovered =
                                    crate::agent_replay::replay_in_transaction(&mut tx, &request)
                                        .await?;
                            }
                        }
                        let execution = if let Some(result) = recovered {
                            result
                        } else if transcript.is_error
                            && (decoded.is_err()
                                || app_application::agent::intentions::recognized_resolution(
                                    true, &value,
                                )
                                .is_some()
                                || matches!(
                                    value["error"].as_str(),
                                    Some(
                                        "tool_not_available_in_authored_adventure"
                                            | "canonical_player_intention_required"
                                            | "player_intention_not_requested"
                                            | "player_intention_already_resolved"
                                    )
                                ))
                        {
                            ToolExecution {
                                result: value,
                                is_error: true,
                                handled_by: "engine".into(),
                                media: vec![],
                            }
                        } else {
                            return Err(repository_error("confirmed_tool_receipt_missing"));
                        };
                        let raw = serde_json::to_string(&execution)
                            .map_err(|_| repository_error("encode_confirmed_tool_delivery"))?;
                        sqlx::query("INSERT INTO agent_tool_steps(request_id,tool_call_id,command_json,result_json) VALUES(?1,?2,?3,?4) ON CONFLICT(request_id,tool_call_id) DO UPDATE SET result_json=excluded.result_json WHERE agent_tool_steps.result_json IS NULL AND agent_tool_steps.command_json=excluded.command_json")
                            .bind(input.request_id.to_string()).bind(&call.id).bind(signature).bind(raw).execute(&mut *tx).await.map_err(|_|repository_error("repair_confirmed_tool_delivery"))?;
                        execution
                    };
                    if !events.iter().any(|event| matches!(event,AgentEvent::ToolCallStart{id:existing,..} if existing==&id)) {
                        let start=AgentEvent::ToolCallStart{id:id.clone(),tool_name:call.name.clone(),round};
                        let first_delivery=events.iter().position(|event| match event {
                            AgentEvent::ToolCallResult{id:existing,..}=>existing==&id,
                            AgentEvent::ImageGenerated{tool_call_id,..}|AgentEvent::VideoGenerated{tool_call_id,..}=>tool_call_id==&id,
                            _=>false,
                        }).unwrap_or(events.len());
                        events.insert(first_delivery,start);
                    }
                    for media in execution.media {
                        let event = match media {
                            GeneratedAgentMedia::Image {
                                media_ref,
                                mime_type,
                                data_b64,
                                kind,
                                source,
                            } => AgentEvent::ImageGenerated {
                                media_ref,
                                tool_call_id: id.clone(),
                                round,
                                mime_type,
                                image_b64: data_b64,
                                kind,
                                source,
                            },
                            GeneratedAgentMedia::Video {
                                mime_type,
                                data_b64,
                                kind,
                            } => AgentEvent::VideoGenerated {
                                tool_call_id: id.clone(),
                                round,
                                mime_type,
                                video_b64: data_b64,
                                kind,
                            },
                        };
                        if !events.contains(&event) {
                            events.push(event);
                        }
                    }
                    if !events.iter().any(|event| matches!(event, AgentEvent::ToolCallResult {id:existing,..} if existing == &id)) {
                        events.push(AgentEvent::ToolCallResult {id, tool_name:call.name.clone(), args:call.args.clone(), result:execution.result,is_error:execution.is_error,round,handled_by:execution.handled_by});
                    }
                }
            }
        }
        sqlx::query("DELETE FROM agent_turn_events WHERE request_id=?1")
            .bind(input.request_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|_| repository_error("clear_unfinished_agent_events"))?;
        for (index, event) in events.iter().enumerate() {
            sqlx::query(
                "INSERT INTO agent_turn_events(request_id,sequence,event_json) VALUES(?1,?2,?3)",
            )
            .bind(input.request_id.to_string())
            .bind(index as i64)
            .bind(
                serde_json::to_string(event)
                    .map_err(|_| repository_error("encode_agent_checkpoint_event"))?,
            )
            .execute(&mut *tx)
            .await
            .map_err(|_| repository_error("retain_agent_checkpoint_event"))?;
        }
        sqlx::query("UPDATE agent_turn_operations SET status='running',owner_id=?2,updated_at=?3 WHERE request_id=?1").bind(input.request_id.to_string()).bind(owner.to_string()).bind(chrono::Utc::now().to_rfc3339()).execute(&mut *tx).await.map_err(|_|repository_error("claim_agent_resume"))?;
        tx.commit()
            .await
            .map_err(|_| repository_error("commit_agent_resume"))?;
        Ok(OperationClaim::Resuming { messages, events })
    }
    async fn claim(
        &self,
        input: AgentOperation,
        owner: Uuid,
    ) -> Result<OperationClaim, RepositoryError> {
        if input.request_id.is_nil() || owner.is_nil() {
            return Err(rejected("agent_request_invalid"));
        }
        let encoded = serde_json::to_string(&input)
            .map_err(|_| repository_error("encode_agent_operation"))?;
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| repository_error("begin_agent_operation"))?;
        let row = sqlx::query(
            "SELECT request_json,owner_id,status FROM agent_turn_operations WHERE request_id=?1",
        )
        .bind(input.request_id.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| repository_error("read_agent_operation"))?;
        let claim = if let Some(row) = row {
            let original: String = row
                .try_get("request_json")
                .map_err(|_| repository_error("decode_agent_operation"))?;
            if original != encoded {
                return Err(rejected("agent_request_mismatch"));
            }
            let status: String = row
                .try_get("status")
                .map_err(|_| repository_error("decode_agent_operation"))?;
            match status.as_str() {
                "complete" => {
                    let rows:Vec<String>=sqlx::query_scalar("SELECT event_json FROM agent_turn_events WHERE request_id=?1 ORDER BY sequence")
                        .bind(input.request_id.to_string()).fetch_all(&mut *tx).await.map_err(|_|repository_error("read_agent_operation_events"))?;
                    let events = rows
                        .iter()
                        .map(|raw| {
                            serde_json::from_str(raw)
                                .map_err(|_| repository_error("decode_agent_operation_event"))
                        })
                        .collect::<Result<Vec<AgentEvent>, _>>()?;
                    if !matches!(events.last(), Some(AgentEvent::AgentDone { .. })) {
                        return Err(rejected("agent_operation_state_invalid"));
                    }
                    OperationClaim::Completed(events)
                }
                "running" => OperationClaim::Running,
                "interrupted" => OperationClaim::Interrupted,
                _ => return Err(rejected("agent_operation_state_invalid")),
            }
        } else {
            let now = chrono::Utc::now().to_rfc3339();
            sqlx::query("INSERT INTO agent_turn_operations(request_id,campaign_id,session_id,request_json,owner_id,status,created_at,updated_at,checkpoint_version,restore_generation,actor_id) VALUES(?1,?2,?3,?4,?5,'running',?6,?6,1,COALESCE((SELECT restore_generation FROM sessions WHERE id=?3),0),(SELECT character_id FROM campaign_characters WHERE campaign_id=?2))")
                .bind(input.request_id.to_string()).bind(input.request.campaign_id.to_string()).bind(input.request.session_id.to_string()).bind(encoded).bind(owner.to_string()).bind(now)
                .execute(&mut *tx).await.map_err(|_|repository_error("insert_agent_operation"))?;
            OperationClaim::Started
        };
        tx.commit()
            .await
            .map_err(|_| repository_error("commit_agent_operation_claim"))?;
        Ok(claim)
    }

    async fn append_event(
        &self,
        request_id: Uuid,
        owner: Uuid,
        sequence: u64,
        event: AgentEvent,
    ) -> Result<(), RepositoryError> {
        let sequence =
            i64::try_from(sequence).map_err(|_| rejected("agent_event_sequence_invalid"))?;
        let raw = serde_json::to_string(&event)
            .map_err(|_| repository_error("encode_agent_operation_event"))?;
        if raw.len() > 32 * 1024 * 1024 {
            return Err(rejected("agent_event_too_large"));
        }
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| repository_error("begin_agent_event"))?;
        let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM agent_turn_operations WHERE request_id=?1 AND owner_id=?2 AND status='running')")
            .bind(request_id.to_string()).bind(owner.to_string()).fetch_one(&mut *tx).await.map_err(|_|repository_error("read_agent_operation_owner"))?;
        if !active {
            return Err(rejected("agent_operation_not_owned"));
        }
        let existing: Option<String> = sqlx::query_scalar(
            "SELECT event_json FROM agent_turn_events WHERE request_id=?1 AND sequence=?2",
        )
        .bind(request_id.to_string())
        .bind(sequence)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| repository_error("read_agent_event_retry"))?;
        if let Some(existing) = existing {
            if existing != raw {
                return Err(rejected("agent_event_mismatch"));
            }
        } else {
            let next: i64 = sqlx::query_scalar(
                "SELECT COALESCE(MAX(sequence)+1,0) FROM agent_turn_events WHERE request_id=?1",
            )
            .bind(request_id.to_string())
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| repository_error("read_agent_event_sequence"))?;
            if sequence != next {
                return Err(rejected("agent_event_sequence_invalid"));
            }
            sqlx::query(
                "INSERT INTO agent_turn_events(request_id,sequence,event_json) VALUES(?1,?2,?3)",
            )
            .bind(request_id.to_string())
            .bind(sequence)
            .bind(raw)
            .execute(&mut *tx)
            .await
            .map_err(|_| repository_error("insert_agent_operation_event"))?;
        }
        tx.commit()
            .await
            .map_err(|_| repository_error("commit_agent_event"))?;
        Ok(())
    }

    async fn finish(
        &self,
        request_id: Uuid,
        owner: Uuid,
        complete: bool,
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| repository_error("begin_agent_operation_finish"))?;
        if complete {
            let last:Option<String>=sqlx::query_scalar("SELECT event_json FROM agent_turn_events WHERE request_id=?1 ORDER BY sequence DESC LIMIT 1")
                .bind(request_id.to_string()).fetch_optional(&mut *tx).await.map_err(|_|repository_error("read_agent_terminal_event"))?;
            let last = last.and_then(|raw| serde_json::from_str::<AgentEvent>(&raw).ok());
            if !matches!(last, Some(AgentEvent::AgentDone { .. })) {
                return Err(rejected("agent_operation_not_complete"));
            }
        }
        let changed=sqlx::query("UPDATE agent_turn_operations SET status=?3,updated_at=?4 WHERE request_id=?1 AND owner_id=?2 AND status='running'")
            .bind(request_id.to_string()).bind(owner.to_string()).bind(if complete{"complete"}else{"interrupted"}).bind(chrono::Utc::now().to_rfc3339()).execute(&mut *tx).await.map_err(|_|repository_error("finish_agent_operation"))?;
        if changed.rows_affected() != 1 {
            return Err(rejected("agent_operation_not_owned"));
        }
        tx.commit()
            .await
            .map_err(|_| repository_error("commit_agent_operation_finish"))?;
        Ok(())
    }
}

impl SqliteStore {
    pub async fn claim_tool_step(
        &self,
        request_id: Uuid,
        tool_call_id: &str,
        command: serde_json::Value,
    ) -> Result<app_application::agent::operations::ToolStepClaim, RepositoryError> {
        use app_application::agent::operations::ToolStepClaim;
        let encoded = serde_json::to_string(&command)
            .map_err(|_| repository_error("encode_tool_checkpoint"))?;
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| repository_error("begin_tool_checkpoint"))?;
        let row=sqlx::query("SELECT command_json,result_json FROM agent_tool_steps WHERE request_id=?1 AND tool_call_id=?2").bind(request_id.to_string()).bind(tool_call_id).fetch_optional(&mut *tx).await.map_err(|_|repository_error("read_tool_checkpoint"))?;
        let claim = if let Some(row) = row {
            if row.get::<String, _>("command_json") != encoded {
                return Err(rejected("agent_tool_signature_mismatch"));
            }
            match row.get::<Option<String>, _>("result_json") {
                Some(raw) => ToolStepClaim::Completed(
                    serde_json::from_str(&raw)
                        .map_err(|_| repository_error("decode_tool_checkpoint"))?,
                ),
                None => ToolStepClaim::Pending,
            }
        } else {
            sqlx::query("INSERT INTO agent_tool_steps(request_id,tool_call_id,command_json) VALUES(?1,?2,?3)").bind(request_id.to_string()).bind(tool_call_id).bind(encoded).execute(&mut *tx).await.map_err(|_|repository_error("insert_tool_checkpoint"))?;
            ToolStepClaim::Started
        };
        tx.commit()
            .await
            .map_err(|_| repository_error("commit_tool_checkpoint_claim"))?;
        Ok(claim)
    }
    pub async fn finish_tool_step(
        &self,
        request_id: Uuid,
        tool_call_id: &str,
        result: &app_application::agent::tool_dispatch::ToolExecution,
    ) -> Result<(), RepositoryError> {
        let raw = serde_json::to_string(result)
            .map_err(|_| repository_error("encode_tool_checkpoint_result"))?;
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| repository_error("begin_tool_checkpoint_result"))?;
        let bound:Option<String>=sqlx::query_scalar("SELECT binding_json FROM solo_human_command_bindings WHERE request_id=?1 AND tool_call_id=?2").bind(request_id.to_string()).bind(tool_call_id).fetch_optional(&mut *tx).await.map_err(|_|repository_error("read_tool_checkpoint_binding"))?;
        if let Some(bound) = bound {
            let binding: app_application::agent::human_commands::PreparedCommandBinding =
                serde_json::from_str(&bound)
                    .map_err(|_| repository_error("decode_tool_checkpoint_binding"))?;
            if result.is_error
                && !result.result["error"]
                    .as_str()
                    .is_some_and(app_application::agent::intentions::definitive_rejection)
            {
                return Err(rejected("human_command_outcome_uncertain"));
            }
            if result.is_error
                || matches!(
                    binding.signature,
                    app_application::agent::human_commands::HumanAction::Dice(_)
                )
            {
                crate::human_commands::require_prepared_context(
                    &mut tx,
                    &format!("agent:{request_id}:{tool_call_id}"),
                    binding.scope.campaign_id,
                    binding.scope.session_id,
                )
                .await?;
            }
        }
        let changed=sqlx::query("UPDATE agent_tool_steps SET result_json=?3 WHERE request_id=?1 AND tool_call_id=?2 AND result_json IS NULL").bind(request_id.to_string()).bind(tool_call_id).bind(raw).execute(&mut *tx).await.map_err(|_|repository_error("finish_tool_checkpoint"))?;
        if changed.rows_affected() != 1 {
            return Err(rejected("agent_tool_checkpoint_conflict"));
        }
        for media in &result.media {
            if let app_application::agent::tool_dispatch::GeneratedAgentMedia::Image {
                media_ref: Some(reference),
                kind,
                ..
            } = media
            {
                if kind != "map" || result.is_error {
                    continue;
                }
                let op=sqlx::query("SELECT a.campaign_id,a.session_id,a.restore_generation,a.status,s.restore_generation AS current_generation,s.revision FROM agent_turn_operations a JOIN sessions s ON s.id=a.session_id WHERE a.request_id=?1").bind(request_id.to_string()).fetch_one(&mut *tx).await.map_err(|_|repository_error("media_scope_read_failed"))?;
                if op.get::<i64, _>("restore_generation") != op.get::<i64, _>("current_generation")
                    || op.get::<String, _>("status") != "running"
                {
                    return Err(rejected("media_scope_changed"));
                };
                let campaign = Uuid::parse_str(&op.get::<String, _>("campaign_id"))
                    .map_err(|_| rejected("media_scope_invalid"))?;
                let session = Uuid::parse_str(&op.get::<String, _>("session_id"))
                    .map_err(|_| rejected("media_scope_invalid"))?;
                crate::media_assets::select_map(
                    &mut tx,
                    campaign,
                    session,
                    Some(reference),
                    &request_id.to_string(),
                    false,
                )
                .await?;
                crate::campaigns::advance_session_revision(&mut tx,app_application::models::identity::CampaignId(campaign),app_application::models::identity::SessionId(session),app_application::models::identity::Revision(op.get::<i64,_>("revision") as u64),"map_media_selected",&format!("media:{request_id}:{tool_call_id}")).await?;
            }
        }
        tx.commit()
            .await
            .map_err(|_| repository_error("commit_tool_checkpoint_result"))?;
        Ok(())
    }
    /// Bootstrap-only reconciliation, after acquiring the OS-backed exclusive
    /// authority lock. A changed UUID alone is not proof an old worker stopped.
    pub async fn recover_after_exclusive_authority(
        &self,
        owner: Uuid,
    ) -> Result<u64, RepositoryError> {
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| repository_error("recover_agent_operations"))?;
        let room_result=sqlx::query("UPDATE room_dm_turns SET status='interrupted' WHERE status='running' AND process_owner!=?1").bind(owner.to_string()).execute(&mut *tx).await.map_err(|_|repository_error("recover_room_dm_turns"))?;
        let result=sqlx::query("UPDATE agent_turn_operations SET status='interrupted',updated_at=?2 WHERE status='running' AND owner_id!=?1")
            .bind(owner.to_string()).bind(chrono::Utc::now().to_rfc3339()).execute(&mut *tx).await.map_err(|_|repository_error("recover_agent_operations"))?;
        tx.commit()
            .await
            .map_err(|_| repository_error("recover_agent_operations"))?;
        Ok(result.rows_affected() + room_result.rows_affected())
    }
}
