use crate::{
    room_agent::{checkpoint, guard, timeline},
    SqliteStore,
};
use app_application::{
    agent::operations::{AgentOperation, OperationClaim},
    models::{agent::AgentEvent, room::RoomLease},
    ports::repositories::RepositoryError,
};
use sqlx::Row;
use uuid::Uuid;
fn error(code: &'static str) -> RepositoryError {
    RepositoryError::Operation {
        operation: "room_dm_turn",
        code,
    }
}
async fn row(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    lease: &RoomLease,
    turn: Uuid,
) -> Result<Option<sqlx::sqlite::SqliteRow>, RepositoryError> {
    sqlx::query("SELECT * FROM room_dm_turns WHERE room_id=?1 AND member_id=?2 AND turn_id=?3")
        .bind(lease.room_id().to_string())
        .bind(lease.member_id().to_string())
        .bind(turn.to_string())
        .fetch_optional(&mut **tx)
        .await
        .map_err(|_| error("room_dm_read_failed"))
}
fn valid_row(
    row: &sqlx::sqlite::SqliteRow,
    lease: &RoomLease,
    reset: i64,
) -> Result<(), RepositoryError> {
    if row.get::<i64, _>("room_epoch") as u64 != lease.epoch()
        || row.get::<i64, _>("reset_cursor") != reset
        || row.get::<String, _>("status") == "superseded"
    {
        return Err(error("agent_turn_superseded"));
    }
    Ok(())
}
async fn events(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    lease: &RoomLease,
    turn: Uuid,
) -> Result<Vec<AgentEvent>, RepositoryError> {
    let rows:Vec<String>=sqlx::query_scalar("SELECT event_json FROM room_dm_events WHERE room_id=?1 AND member_id=?2 AND turn_id=?3 ORDER BY sequence").bind(lease.room_id().to_string()).bind(lease.member_id().to_string()).bind(turn.to_string()).fetch_all(&mut **tx).await.map_err(|_|error("room_dm_read_failed"))?;
    rows.into_iter()
        .map(|raw| serde_json::from_str(&raw).map_err(|_| error("room_dm_read_failed")))
        .collect()
}
pub(crate) async fn request(
    store: &SqliteStore,
    lease: &RoomLease,
    turn: Uuid,
) -> Result<Option<AgentOperation>, RepositoryError> {
    let mut tx = store
        .pool
        .begin()
        .await
        .map_err(|_| error("room_dm_begin_failed"))?;
    guard(&mut tx, lease).await?;
    let reset = timeline(&mut tx, lease, turn).await?;
    let found = if let Some(row) = row(&mut tx, lease, turn).await? {
        valid_row(&row, lease, reset)?;
        Some(
            serde_json::from_str(&row.get::<String, _>("request_json"))
                .map_err(|_| error("room_dm_read_failed"))?,
        )
    } else {
        None
    };
    tx.commit()
        .await
        .map_err(|_| error("room_dm_commit_failed"))?;
    Ok(found)
}
#[tracing::instrument(skip_all,fields(turn_id=%input.request_id,room_id=%lease.room_id()))]
pub(crate) async fn claim(
    store: &SqliteStore,
    lease: &RoomLease,
    input: AgentOperation,
    owner: Uuid,
    resume: bool,
) -> Result<OperationClaim, RepositoryError> {
    let turn = input.request_id;
    if turn.is_nil()
        || owner.is_nil()
        || input.request.campaign_id != lease.campaign_id().0
        || input.request.session_id != lease.session_id().0
        || input.request.player_message.trim().is_empty()
        || input.request.player_message.len() > 16384
        || !input.request.images.is_empty()
        || input.request.board.is_some()
    {
        return Err(error("agent_request_mismatch"));
    }
    let raw = serde_json::to_string(&input).map_err(|_| error("room_dm_encode_failed"))?;
    let mut tx = store
        .pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|_| error("room_dm_begin_failed"))?;
    guard(&mut tx, lease).await?;
    let reset = timeline(&mut tx, lease, turn).await?;
    let (_, _, actor, _) = crate::room_world::context(&mut tx, lease).await?;
    let result = if let Some(row) = row(&mut tx, lease, turn).await? {
        valid_row(&row, lease, reset)?;
        if row.get::<String, _>("request_json") != raw {
            return Err(error("agent_request_mismatch"));
        }
        match row.get::<String, _>("status").as_str() {
            "complete" => OperationClaim::Completed(events(&mut tx, lease, turn).await?),
            "running" => OperationClaim::Running,
            "interrupted" if !resume => OperationClaim::Interrupted,
            "interrupted" => {
                if row.get::<Option<String>, _>("actor_id").as_deref()
                    != Some(actor.to_string().as_str())
                {
                    return Err(error("agent_turn_superseded"));
                }
                let messages = checkpoint(&mut tx, lease, turn)
                    .await?
                    .into_iter()
                    .map(|entry| entry.message)
                    .collect::<Vec<_>>();
                // Repair confirmed deliveries even if the transcript committed before its event.
                let recovered = confirmed_deliveries(turn, &messages)?;
                sqlx::query(
                    "DELETE FROM room_dm_events WHERE room_id=?1 AND member_id=?2 AND turn_id=?3",
                )
                .bind(lease.room_id().to_string())
                .bind(lease.member_id().to_string())
                .bind(turn.to_string())
                .execute(&mut *tx)
                .await
                .map_err(|_| error("room_dm_write_failed"))?;
                for (sequence, event) in recovered.iter().enumerate() {
                    insert_event(&mut tx, lease, turn, sequence as u64, event).await?;
                }
                sqlx::query("UPDATE room_dm_turns SET status='running',process_owner=?4 WHERE room_id=?1 AND member_id=?2 AND turn_id=?3").bind(lease.room_id().to_string()).bind(lease.member_id().to_string()).bind(turn.to_string()).bind(owner.to_string()).execute(&mut *tx).await.map_err(|_|error("room_dm_write_failed"))?;
                OperationClaim::Resuming {
                    messages,
                    events: recovered,
                }
            }
            _ => return Err(error("room_dm_state_invalid")),
        }
    } else {
        let busy:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM room_dm_turns WHERE campaign_id=?1 AND status IN('running','interrupted'))").bind(lease.campaign_id().0.to_string()).fetch_one(&mut *tx).await.map_err(|_|error("room_dm_read_failed"))?;
        if busy {
            return Err(error("agent_turn_running"));
        }
        sqlx::query("INSERT INTO room_dm_turns(room_id,member_id,turn_id,campaign_id,room_epoch,reset_cursor,process_owner,status,request_json,actor_id) VALUES(?1,?2,?3,?4,?5,?6,?7,'running',?8,?9)").bind(lease.room_id().to_string()).bind(lease.member_id().to_string()).bind(turn.to_string()).bind(lease.campaign_id().0.to_string()).bind(lease.epoch() as i64).bind(reset).bind(owner.to_string()).bind(raw).bind(actor.to_string()).execute(&mut *tx).await.map_err(|_|error("room_dm_write_failed"))?;
        OperationClaim::Started
    };
    tx.commit()
        .await
        .map_err(|_| error("room_dm_commit_failed"))?;
    tracing::debug!("room DM turn claim resolved");
    Ok(result)
}
async fn active(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    lease: &RoomLease,
    turn: Uuid,
    owner: Uuid,
) -> Result<(), RepositoryError> {
    guard(tx, lease).await?;
    let reset = timeline(tx, lease, turn).await?;
    let row = row(tx, lease, turn)
        .await?
        .ok_or_else(|| error("room_dm_missing"))?;
    valid_row(&row, lease, reset)?;
    if row.get::<String, _>("process_owner") != owner.to_string()
        || row.get::<String, _>("status") != "running"
    {
        return Err(error("room_dm_owner_conflict"));
    }
    Ok(())
}
async fn insert_event(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    lease: &RoomLease,
    turn: Uuid,
    sequence: u64,
    event: &AgentEvent,
) -> Result<(), RepositoryError> {
    let sequence = i64::try_from(sequence).map_err(|_| error("room_dm_sequence_invalid"))?;
    sqlx::query("INSERT INTO room_dm_events(room_id,member_id,turn_id,sequence,event_json) VALUES(?1,?2,?3,?4,?5)").bind(lease.room_id().to_string()).bind(lease.member_id().to_string()).bind(turn.to_string()).bind(sequence).bind(serde_json::to_string(event).map_err(|_|error("room_dm_encode_failed"))?).execute(&mut **tx).await.map_err(|_|error("room_dm_write_failed"))?;
    Ok(())
}
pub(crate) async fn append_event(
    store: &SqliteStore,
    lease: &RoomLease,
    turn: Uuid,
    owner: Uuid,
    sequence: u64,
    event: AgentEvent,
) -> Result<(), RepositoryError> {
    if matches!(
        event,
        AgentEvent::ReasoningText { .. }
            | AgentEvent::ImageGenerated { .. }
            | AgentEvent::VideoGenerated { .. }
    ) {
        return Err(error("room_dm_event_invalid"));
    }
    let mut tx = store
        .pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|_| error("room_dm_begin_failed"))?;
    active(&mut tx, lease, turn, owner).await?;
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM room_dm_events WHERE room_id=?1 AND member_id=?2 AND turn_id=?3",
    )
    .bind(lease.room_id().to_string())
    .bind(lease.member_id().to_string())
    .bind(turn.to_string())
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| error("room_dm_read_failed"))?;
    if i64::try_from(sequence).ok() != Some(count) {
        return Err(error("room_dm_sequence_invalid"));
    }
    insert_event(&mut tx, lease, turn, sequence, &event).await?;
    tx.commit()
        .await
        .map_err(|_| error("room_dm_commit_failed"))?;
    Ok(())
}
#[tracing::instrument(skip_all,fields(turn_id=%turn,complete))]
pub(crate) async fn finish(
    store: &SqliteStore,
    lease: &RoomLease,
    turn: Uuid,
    owner: Uuid,
    complete: bool,
) -> Result<(), RepositoryError> {
    let mut tx = store
        .pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|_| error("room_dm_begin_failed"))?;
    active(&mut tx, lease, turn, owner).await?;
    if complete {
        let transcript = checkpoint(&mut tx, lease, turn).await?;
        if !matches!(
            transcript.last().map(|entry| &entry.message),
            Some(app_application::models::chat::ChatMessage::Assistant { .. })
        ) || !matches!(
            events(&mut tx, lease, turn).await?.last(),
            Some(AgentEvent::AgentDone { .. })
        ) {
            return Err(error("room_dm_incomplete"));
        }
    }
    sqlx::query(
        "UPDATE room_dm_turns SET status=?4 WHERE room_id=?1 AND member_id=?2 AND turn_id=?3",
    )
    .bind(lease.room_id().to_string())
    .bind(lease.member_id().to_string())
    .bind(turn.to_string())
    .bind(if complete { "complete" } else { "interrupted" })
    .execute(&mut *tx)
    .await
    .map_err(|_| error("room_dm_write_failed"))?;
    tx.commit()
        .await
        .map_err(|_| error("room_dm_commit_failed"))?;
    Ok(())
}

fn confirmed_deliveries(
    turn: Uuid,
    messages: &[app_application::models::chat::ChatMessage],
) -> Result<Vec<AgentEvent>, RepositoryError> {
    use app_application::models::chat::ChatMessage;
    let mut events = Vec::new();
    let mut round = 0;
    for message in messages {
        if let ChatMessage::AssistantWithToolCalls { tool_calls, .. } = message {
            round += 1;
            for call in tool_calls {
                if let Some(result) = messages.iter().find_map(|message| match message {
                    ChatMessage::ToolResult(result) if result.tool_call_id == call.id => {
                        Some(result)
                    }
                    _ => None,
                }) {
                    let id = format!("{turn}:{}", call.id);
                    events.push(AgentEvent::ToolCallStart {
                        id: id.clone(),
                        tool_name: call.name.clone(),
                        round,
                    });
                    events.push(AgentEvent::ToolCallResult {
                        id,
                        tool_name: call.name.clone(),
                        args: call.args.clone(),
                        result: serde_json::from_str(&result.content)
                            .map_err(|_| error("room_dm_checkpoint_invalid"))?,
                        is_error: result.is_error,
                        round,
                        handled_by: "engine".into(),
                    });
                }
            }
        }
    }
    Ok(events)
}
