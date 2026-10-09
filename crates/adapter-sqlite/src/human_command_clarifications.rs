//! Metadata helpers run only inside an already owner/lease-guarded transaction.
use app_application::{agent::human_commands::*, ports::repositories::RepositoryError};
use sqlx::{Row, Sqlite, Transaction};
fn error() -> RepositoryError {
    RepositoryError::Operation {
        operation: "human_command_clarification",
        code: "human_command_data_invalid",
    }
}
pub(crate) async fn load(
    tx: &mut Transaction<'_, Sqlite>,
    scope: &CommandScope,
) -> Result<Vec<HumanCommandClarification>, RepositoryError> {
    let (query,room,member)=match scope.authority {
        CommandAuthority::Solo {..}=>("SELECT clarifications_json FROM solo_human_command_manifests WHERE request_id=?1",None,None),
        CommandAuthority::Room {room_id,member_id,..}=>("SELECT clarifications_json FROM room_human_command_manifests WHERE turn_id=?1 AND room_id=?2 AND member_id=?3",Some(room_id),Some(member_id)),
    };
    let mut query = sqlx::query_scalar::<_, String>(query).bind(scope.request_id.to_string());
    if let Some(room) = room {
        query = query
            .bind(room.to_string())
            .bind(member.expect("room member").to_string());
    }
    let raw = query.fetch_one(&mut **tx).await.map_err(|_| error())?;
    serde_json::from_str(&raw).map_err(|_| error())
}
pub(crate) async fn persist(
    tx: &mut Transaction<'_, Sqlite>,
    message: &str,
    ledger: &StoredHumanCommandLedger,
) -> Result<(), RepositoryError> {
    ledger
        .manifest
        .validate_structure(message, &ledger.manifest.scope)
        .map_err(|_| error())?;
    ledger.validate_clarifications().map_err(|_| error())?;
    let (query,room,member)=match ledger.manifest.scope.authority {
        CommandAuthority::Solo {..}=>("UPDATE solo_human_command_manifests SET manifest_json=?1,clarifications_json=?2,final_narration_json=?3 WHERE request_id=?4",None,None),
        CommandAuthority::Room {room_id,member_id,..}=>("UPDATE room_human_command_manifests SET manifest_json=?1,clarifications_json=?2,final_narration_json=?3 WHERE turn_id=?4 AND room_id=?5 AND member_id=?6",Some(room_id),Some(member_id)),
    };
    let mut query = sqlx::query(query)
        .bind(serde_json::to_string(&ledger.manifest).map_err(|_| error())?)
        .bind(serde_json::to_string(&ledger.clarifications).map_err(|_| error())?)
        .bind(
            ledger
                .final_narration
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(|_| error())?,
        )
        .bind(ledger.manifest.scope.request_id.to_string());
    if let Some(room) = room {
        query = query
            .bind(room.to_string())
            .bind(member.expect("room member").to_string());
    }
    if query
        .execute(&mut **tx)
        .await
        .map_err(|_| error())?
        .rows_affected()
        != 1
    {
        return Err(error());
    }
    for question in &ledger.clarifications {
        append_bound_assistant(tx, &ledger.manifest.scope, question.id, &question.question).await?;
    }
    if let Some(final_text) = &ledger.final_narration {
        append_bound_assistant(tx, &ledger.manifest.scope, final_text.id, &final_text.text).await?;
    }
    Ok(())
}
pub(crate) async fn load_final(
    tx: &mut Transaction<'_, Sqlite>,
    scope: &CommandScope,
) -> Result<Option<HumanCommandFinalNarration>, RepositoryError> {
    let (query,room,member)=match scope.authority {
        CommandAuthority::Solo {..}=>("SELECT final_narration_json FROM solo_human_command_manifests WHERE request_id=?1",None,None),
        CommandAuthority::Room {room_id,member_id,..}=>("SELECT final_narration_json FROM room_human_command_manifests WHERE turn_id=?1 AND room_id=?2 AND member_id=?3",Some(room_id),Some(member_id)),
    };
    let mut query =
        sqlx::query_scalar::<_, Option<String>>(query).bind(scope.request_id.to_string());
    if let Some(room) = room {
        query = query
            .bind(room.to_string())
            .bind(member.expect("room member").to_string());
    }
    query
        .fetch_one(&mut **tx)
        .await
        .map_err(|_| error())?
        .map(|raw| serde_json::from_str(&raw).map_err(|_| error()))
        .transpose()
}
async fn append_bound_assistant(
    tx: &mut Transaction<'_, Sqlite>,
    scope: &CommandScope,
    id: uuid::Uuid,
    text: &str,
) -> Result<(), RepositoryError> {
    use app_application::models::chat::ChatMessage;
    let message = ChatMessage::Assistant {
        content: text.into(),
    };
    let raw = serde_json::to_string(&message).map_err(|_| error())?;
    let (select,room,member)=match scope.authority {
        CommandAuthority::Solo {..}=>("SELECT message_json FROM agent_turn_messages WHERE request_id=?1 AND bound_message_id=?2",None,None),
        CommandAuthority::Room {room_id,member_id,..}=>("SELECT message_json FROM room_agent_messages WHERE turn_id=?1 AND message_id=?2 AND room_id=?3 AND member_id=?4",Some(room_id),Some(member_id)),
    };
    let mut query = sqlx::query_scalar::<_, String>(select)
        .bind(scope.request_id.to_string())
        .bind(id.to_string());
    if let Some(room) = room {
        query = query
            .bind(room.to_string())
            .bind(member.expect("room member").to_string());
    }
    let saved = query.fetch_optional(&mut **tx).await.map_err(|_| error())?;
    if saved.as_ref().is_some_and(|saved| *saved != raw) {
        return Err(error());
    }
    let (role, parts, tool_calls, tool_call_id, is_error) =
        crate::messages::encode_message(&message);
    if let Some(row) = sqlx::query("SELECT session_id,role,parts FROM messages WHERE id=?1")
        .bind(id.to_string())
        .fetch_optional(&mut **tx)
        .await
        .map_err(|_| error())?
    {
        if row.get::<String, _>("session_id") != scope.session_id.to_string()
            || row.get::<String, _>("role") != role
            || row.get::<String, _>("parts") != parts
        {
            return Err(error());
        }
    } else {
        sqlx::query("INSERT INTO messages(id,session_id,role,parts,tool_calls,tool_call_id,is_error,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)").bind(id.to_string()).bind(scope.session_id.to_string()).bind(role).bind(parts).bind(tool_calls).bind(tool_call_id).bind(i64::from(is_error)).bind(chrono::Utc::now().to_rfc3339()).execute(&mut **tx).await.map_err(|_|error())?;
    }
    if saved.is_some() {
        return Ok(());
    }
    match scope.authority {
        CommandAuthority::Solo { .. } => {
            sqlx::query("INSERT INTO agent_turn_messages(request_id,sequence,message_json,bound_message_id) SELECT ?1,COALESCE(MAX(sequence)+1,0),?2,?3 FROM agent_turn_messages WHERE request_id=?1").bind(scope.request_id.to_string()).bind(raw).bind(id.to_string()).execute(&mut **tx).await.map_err(|_|error())?;
        }
        CommandAuthority::Room {
            room_id,
            member_id,
            reset_cursor,
            ..
        } => {
            sqlx::query("INSERT INTO room_agent_messages(room_id,member_id,turn_id,sequence,message_id,message_json,reset_cursor) SELECT ?1,?2,?3,COALESCE(MAX(sequence)+1,0),?4,?5,?6 FROM room_agent_messages WHERE room_id=?1 AND member_id=?2 AND turn_id=?3").bind(room_id.to_string()).bind(member_id.to_string()).bind(scope.request_id.to_string()).bind(id.to_string()).bind(raw).bind(i64::try_from(reset_cursor).map_err(|_|error())?).execute(&mut **tx).await.map_err(|_|error())?;
        }
    }
    Ok(())
}
