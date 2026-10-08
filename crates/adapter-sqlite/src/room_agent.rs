use app_application::{
    agent::commands::AgentToolCommand,
    models::{room::RoomLease, room_agent::PreparedRoomTool},
    ports::repositories::RepositoryError,
};
use sqlx::Row;
use uuid::Uuid;
fn error(code: &'static str) -> RepositoryError {
    RepositoryError::Operation {
        operation: "room_agent",
        code,
    }
}

pub(crate) async fn guard(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    lease: &RoomLease,
) -> Result<(), RepositoryError> {
    let room = crate::rooms::read(tx, lease.room_id()).await?;
    crate::rooms::verify_lease(&room, lease)?;
    let party = crate::party::load(tx, room.campaign_id)
        .await?
        .ok_or_else(|| error("room_access_denied"))?;
    if party.session_id != room.session_id
        || !party
            .roster
            .members
            .iter()
            .any(|member| member.member_id == lease.member_id() && member.role == lease.role())
    {
        return Err(error("room_access_denied"));
    }
    Ok(())
}
pub(crate) async fn timeline(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    lease: &RoomLease,
    turn: Uuid,
) -> Result<i64, RepositoryError> {
    let reset:i64=sqlx::query_scalar("SELECT COALESCE(MAX(sequence),0) FROM room_changes WHERE room_id=?1 AND kind='timeline_reset'").bind(lease.room_id().to_string()).fetch_one(&mut **tx).await.map_err(read_error)?;
    let original:Option<i64>=sqlx::query_scalar("SELECT MIN(reset_cursor) FROM (SELECT reset_cursor FROM room_agent_messages WHERE room_id=?1 AND member_id=?2 AND turn_id=?3 UNION ALL SELECT reset_cursor FROM room_agent_tools WHERE room_id=?1 AND member_id=?2 AND turn_id=?3 UNION ALL SELECT reset_cursor FROM room_dm_turns WHERE room_id=?1 AND member_id=?2 AND turn_id=?3)").bind(lease.room_id().to_string()).bind(lease.member_id().to_string()).bind(turn.to_string()).fetch_one(&mut **tx).await.map_err(read_error)?;
    if original.is_some_and(|value| value != reset) {
        return Err(error("room_world_conflict"));
    }
    Ok(reset)
}
pub(crate) async fn checkpoint(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    lease: &RoomLease,
    turn: Uuid,
) -> Result<Vec<app_application::models::campaign::StoredMessage>, RepositoryError> {
    let rows=sqlx::query("SELECT sequence,message_id,message_json FROM room_agent_messages WHERE room_id=?1 AND member_id=?2 AND turn_id=?3 ORDER BY sequence").bind(lease.room_id().to_string()).bind(lease.member_id().to_string()).bind(turn.to_string()).fetch_all(&mut **tx).await.map_err(read_error)?;
    rows.into_iter()
        .map(|row| {
            Ok(app_application::models::campaign::StoredMessage {
                id: Uuid::parse_str(&row.get::<String, _>("message_id"))
                    .map_err(|_| error("room_agent_read_failed"))?,
                session_id: lease.session_id().0,
                sequence: row.get("sequence"),
                message: serde_json::from_str(&row.get::<String, _>("message_json"))
                    .map_err(|_| error("room_agent_read_failed"))?,
            })
        })
        .collect()
}
pub(crate) async fn load_checkpoint(
    store: &crate::SqliteStore,
    lease: &RoomLease,
    turn: Uuid,
) -> Result<Vec<app_application::models::campaign::StoredMessage>, RepositoryError> {
    let mut tx = store
        .pool
        .begin()
        .await
        .map_err(|_| error("room_agent_begin_failed"))?;
    guard(&mut tx, lease).await?;
    timeline(&mut tx, lease, turn).await?;
    let messages = checkpoint(&mut tx, lease, turn).await?;
    tx.commit()
        .await
        .map_err(|_| error("room_agent_commit_failed"))?;
    Ok(messages)
}
pub(crate) async fn append(
    store: &crate::SqliteStore,
    lease: &RoomLease,
    turn: Uuid,
    message: app_application::models::chat::ChatMessage,
) -> Result<app_application::models::campaign::StoredMessage, RepositoryError> {
    use app_application::models::chat::ChatMessage;
    if turn.is_nil() || matches!(message, ChatMessage::System { .. }) {
        return Err(error("room_access_denied"));
    }
    let mut tx = store
        .pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|_| error("room_agent_begin_failed"))?;
    guard(&mut tx, lease).await?;
    let reset_cursor = timeline(&mut tx, lease, turn).await?;
    let saved = checkpoint(&mut tx, lease, turn).await?;
    if matches!(message, ChatMessage::User { .. }) && !saved.is_empty() {
        let first = &saved[0];
        if first.message != message {
            return Err(error("room_world_conflict"));
        }
        let result = first.clone();
        tx.commit()
            .await
            .map_err(|_| error("room_agent_commit_failed"))?;
        return Ok(result);
    }
    let sequence = i64::try_from(saved.len()).map_err(|_| error("room_agent_write_failed"))?;
    let id = Uuid::new_v4();
    let (role, parts, tool_calls, tool_id, is_error) = crate::messages::encode_message(&message);
    sqlx::query("INSERT INTO messages(id,session_id,role,parts,tool_calls,tool_call_id,is_error,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)").bind(id.to_string()).bind(lease.session_id().0.to_string()).bind(role).bind(parts).bind(tool_calls).bind(tool_id).bind(i64::from(is_error)).bind(chrono::Utc::now().to_rfc3339()).execute(&mut *tx).await.map_err(|_|error("room_agent_write_failed"))?;
    sqlx::query("INSERT INTO room_agent_messages(room_id,member_id,turn_id,sequence,message_id,message_json,reset_cursor) VALUES(?1,?2,?3,?4,?5,?6,?7)").bind(lease.room_id().to_string()).bind(lease.member_id().to_string()).bind(turn.to_string()).bind(sequence).bind(id.to_string()).bind(serde_json::to_string(&message).map_err(|_|error("room_agent_write_failed"))?).bind(reset_cursor).execute(&mut *tx).await.map_err(|_|error("room_agent_write_failed"))?;
    tx.commit()
        .await
        .map_err(|_| error("room_agent_commit_failed"))?;
    Ok(app_application::models::campaign::StoredMessage {
        id,
        session_id: lease.session_id().0,
        sequence,
        message,
    })
}
pub(crate) async fn prepare(
    store: &crate::SqliteStore,
    lease: &RoomLease,
    turn_id: Uuid,
    tool_id: &str,
    command: AgentToolCommand,
) -> Result<PreparedRoomTool, RepositoryError> {
    if turn_id.is_nil()
        || tool_id.is_empty()
        || tool_id.len() > 256
        || tool_id.chars().any(char::is_control)
    {
        return Err(error("room_action_invalid"));
    }
    if !matches!(
        command,
        AgentToolCommand::AdventureAction(_)
            | AgentToolCommand::CombatAction(_)
            | AgentToolCommand::RestAction(_)
            | AgentToolCommand::RollDice(_)
    ) {
        return Err(error("room_access_denied"));
    }
    let request = match &command {
        AgentToolCommand::AdventureAction(value) => serde_json::json!({"adventure":value}),
        AgentToolCommand::CombatAction(value) => serde_json::json!({"combat":value}),
        AgentToolCommand::RestAction(value) => serde_json::json!({"rest":value}),
        AgentToolCommand::RollDice(value) if value.check.is_some() => {
            serde_json::json!({"check":value})
        }
        _ => return Err(error("room_access_denied")),
    }
    .to_string();
    let mut tx = store
        .pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|_| error("room_agent_begin_failed"))?;
    let room = crate::rooms::read(&mut tx, lease.room_id()).await?;
    crate::rooms::verify_lease(&room, lease)?;
    let party = crate::party::load(&mut tx, room.campaign_id)
        .await?
        .ok_or_else(|| error("room_access_denied"))?;
    if party.session_id != room.session_id
        || !party
            .roster
            .members
            .iter()
            .any(|member| member.member_id == lease.member_id() && member.role == lease.role())
    {
        return Err(error("room_access_denied"));
    }
    timeline(&mut tx, lease, turn_id).await?;
    if let Some(row)=sqlx::query("SELECT command_json,prepared_json FROM room_agent_tools WHERE room_id=?1 AND member_id=?2 AND turn_id=?3 AND tool_id=?4").bind(room.room_id.to_string()).bind(lease.member_id().to_string()).bind(turn_id.to_string()).bind(tool_id).fetch_optional(&mut *tx).await.map_err(read_error)? {
        if row.get::<String,_>("command_json")!=request {return Err(error("room_world_conflict"));}
        let prepared=serde_json::from_str(&row.get::<String,_>("prepared_json")).map_err(|_|error("room_agent_read_failed"))?;
        tx.commit().await.map_err(|_|error("room_agent_commit_failed"))?;return Ok(prepared);
    }
    let reset_cursor = timeline(&mut tx, lease, turn_id).await?;
    let current = crate::campaign_state::load_in_transaction(&mut tx, room.campaign_id).await?;
    let mut prepared = PreparedRoomTool {
        nonce: Uuid::new_v4().to_string(),
        expected_revision: current.campaign.revision.0,
        encounter_id: None,
        check_result: None,
    };
    if matches!(command, AgentToolCommand::CombatAction(_)) {
        let projection: app_application::models::combat::CombatProjection = serde_json::from_value(
            current
                .projection
                .combat
                .ok_or_else(|| error("room_combat_conflict"))?,
        )
        .map_err(|_| error("room_agent_read_failed"))?;
        prepared.expected_revision = projection.revision;
        prepared.encounter_id = Some(projection.encounter_id);
    }
    sqlx::query("INSERT INTO room_agent_tools(room_id,member_id,turn_id,tool_id,command_json,prepared_json,reset_cursor) VALUES(?1,?2,?3,?4,?5,?6,?7)").bind(room.room_id.to_string()).bind(lease.member_id().to_string()).bind(turn_id.to_string()).bind(tool_id).bind(request).bind(serde_json::to_string(&prepared).map_err(|_|error("room_agent_write_failed"))?).bind(reset_cursor).execute(&mut *tx).await.map_err(|_|error("room_agent_write_failed"))?;
    tx.commit()
        .await
        .map_err(|_| error("room_agent_commit_failed"))?;
    Ok(prepared)
}

pub(crate) async fn dm_party(
    store: &crate::SqliteStore,
    lease: &RoomLease,
) -> Result<app_application::models::party::PartyState, RepositoryError> {
    let mut tx = store
        .pool
        .begin()
        .await
        .map_err(|_| error("room_agent_begin_failed"))?;
    guard(&mut tx, lease).await?;
    let party = crate::party::load(&mut tx, lease.campaign_id())
        .await?
        .ok_or_else(|| error("room_access_denied"))?;
    tx.commit()
        .await
        .map_err(|_| error("room_agent_commit_failed"))?;
    Ok(party)
}

fn read_error(error: sqlx::Error) -> RepositoryError {
    let code = error
        .as_database_error()
        .and_then(|error| error.code())
        .map(|code| code.to_string());
    if code
        .as_deref()
        .and_then(|code| code.parse::<u32>().ok())
        .is_some_and(|code| matches!(code & 0xff, 5 | 6))
    {
        tracing::debug!(
            database_code = code,
            "room agent SQLite read temporarily locked"
        );
        return self::error("room_agent_busy");
    }
    tracing::warn!(database_code = code, "room agent SQLite read failed");
    self::error("room_agent_read_failed")
}
