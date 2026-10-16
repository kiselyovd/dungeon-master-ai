use app_application::models::campaign::StoredMessage;
use app_application::models::chat::{ChatMessage, MessagePart, ToolCall, ToolResult};
use app_application::ports::repositories::{MessageRepository, RepositoryError};
use async_trait::async_trait;
use sqlx::Row;
use uuid::Uuid;

use crate::SqliteStore;

pub struct OperationMessages {
    pub store: SqliteStore,
    pub request_id: Uuid,
}

#[async_trait]
impl MessageRepository for OperationMessages {
    async fn append(
        &self,
        session_id: Uuid,
        message: ChatMessage,
    ) -> Result<StoredMessage, RepositoryError> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM agent_turn_operations WHERE request_id=?1)",
        )
        .bind(self.request_id.to_string())
        .fetch_one(self.store.pool())
        .await
        .map_err(|_| repository_error("read_message_operation"))?;
        if exists {
            self.store
                .append_turn_message(self.request_id, session_id, message)
                .await
        } else {
            self.store.append(session_id, message).await
        }
    }
    async fn list(&self, session_id: Uuid) -> Result<Vec<StoredMessage>, RepositoryError> {
        self.store.list(session_id).await
    }
}

impl SqliteStore {
    pub async fn append_turn_message(
        &self,
        request_id: Uuid,
        session_id: Uuid,
        message: ChatMessage,
    ) -> Result<StoredMessage, RepositoryError> {
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| repository_error("begin_turn_message"))?;
        let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM agent_turn_operations WHERE request_id=?1 AND session_id=?2 AND status='running')").bind(request_id.to_string()).bind(session_id.to_string()).fetch_one(&mut *tx).await.map_err(|_|repository_error("validate_turn_message"))?;
        if !valid {
            return Err(repository_error("turn_message_not_active"));
        }
        let sequence: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(sequence)+1,0) FROM agent_turn_messages WHERE request_id=?1",
        )
        .bind(request_id.to_string())
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| repository_error("turn_message_sequence"))?;
        let id = Uuid::new_v4();
        let (role, parts, tool_calls, tool_call_id, is_error) = encode_message(&message);
        sqlx::query("INSERT INTO messages(id,session_id,role,parts,tool_calls,tool_call_id,is_error,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)")
            .bind(id.to_string()).bind(session_id.to_string()).bind(role).bind(parts).bind(tool_calls).bind(tool_call_id).bind(i64::from(is_error)).bind(chrono::Utc::now().to_rfc3339()).execute(&mut *tx).await.map_err(|_|repository_error("insert_turn_message"))?;
        sqlx::query(
            "INSERT INTO agent_turn_messages(request_id,sequence,message_json) VALUES(?1,?2,?3)",
        )
        .bind(request_id.to_string())
        .bind(sequence)
        .bind(serde_json::to_string(&message).map_err(|_| repository_error("encode_turn_message"))?)
        .execute(&mut *tx)
        .await
        .map_err(|_| repository_error("checkpoint_turn_message"))?;
        tx.commit()
            .await
            .map_err(|_| repository_error("commit_turn_message"))?;
        Ok(StoredMessage {
            id,
            session_id,
            sequence,
            message,
        })
    }
}

#[async_trait]
impl MessageRepository for SqliteStore {
    #[tracing::instrument(skip_all, fields(session_id = %session_id, repository = "messages"))]
    async fn append(
        &self,
        session_id: Uuid,
        message: ChatMessage,
    ) -> Result<StoredMessage, RepositoryError> {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now().to_rfc3339();
        let (role, parts, tool_calls, tool_call_id, is_error) = encode_message(&message);
        sqlx::query(
            "INSERT INTO messages \
             (id, session_id, role, parts, tool_calls, tool_call_id, is_error, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(id.to_string())
        .bind(session_id.to_string())
        .bind(role)
        .bind(parts)
        .bind(tool_calls)
        .bind(tool_call_id)
        .bind(i64::from(is_error))
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|_| repository_error("append_message"))?;
        Ok(StoredMessage {
            id,
            session_id,
            sequence: 0,
            message,
        })
    }

    #[tracing::instrument(skip_all, fields(session_id = %session_id, repository = "messages"))]
    async fn list(&self, session_id: Uuid) -> Result<Vec<StoredMessage>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT id, role, parts, tool_calls, tool_call_id, is_error \
             FROM messages WHERE session_id = ?1 ORDER BY created_at ASC, id ASC",
        )
        .bind(session_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|_| repository_error("list_messages"))?;
        rows.into_iter()
            .enumerate()
            .map(|(index, row)| {
                let raw_id = row
                    .try_get::<String, _>("id")
                    .map_err(|_| repository_error("decode_message_id"))?;
                let id = parse_uuid(raw_id, "message_id")?;
                let role: String = row
                    .try_get("role")
                    .map_err(|_| repository_error("decode"))?;
                let parts: String = row
                    .try_get("parts")
                    .map_err(|_| repository_error("decode"))?;
                let tool_calls: Option<String> = row
                    .try_get("tool_calls")
                    .map_err(|_| repository_error("decode"))?;
                let tool_call_id: Option<String> = row
                    .try_get("tool_call_id")
                    .map_err(|_| repository_error("decode"))?;
                let is_error: i64 = row
                    .try_get("is_error")
                    .map_err(|_| repository_error("decode"))?;
                Ok(StoredMessage {
                    id,
                    session_id,
                    sequence: index as i64,
                    message: decode_message(
                        &role,
                        &parts,
                        tool_calls.as_deref(),
                        tool_call_id.as_deref(),
                        is_error != 0,
                    )?,
                })
            })
            .collect()
    }
}

pub(crate) fn encode_message(
    message: &ChatMessage,
) -> (&'static str, String, Option<String>, Option<String>, bool) {
    match message {
        ChatMessage::System { content } => ("system", text_parts(content), None, None, false),
        ChatMessage::User { parts } => ("user", json(parts), None, None, false),
        ChatMessage::Assistant { content } => ("assistant", text_parts(content), None, None, false),
        ChatMessage::AssistantWithToolCalls {
            content,
            tool_calls,
        } => (
            "assistant_with_tool_calls",
            text_parts(content.as_deref().unwrap_or_default()),
            Some(json(tool_calls)),
            None,
            false,
        ),
        ChatMessage::ToolResult(result) => (
            "tool_result",
            text_parts(&result.content),
            None,
            Some(result.tool_call_id.clone()),
            result.is_error,
        ),
    }
}

pub(crate) fn decode_message(
    role: &str,
    parts_json: &str,
    tool_calls_json: Option<&str>,
    tool_call_id: Option<&str>,
    is_error: bool,
) -> Result<ChatMessage, RepositoryError> {
    let parts: Vec<MessagePart> =
        serde_json::from_str(parts_json).map_err(|_| repository_error("decode_parts"))?;
    let text = parts
        .iter()
        .find_map(|part| match part {
            MessagePart::Text { text } => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_default();
    match role {
        "system" => Ok(ChatMessage::System { content: text }),
        "user" => Ok(ChatMessage::User { parts }),
        "assistant" => Ok(ChatMessage::Assistant { content: text }),
        "assistant_with_tool_calls" => Ok(ChatMessage::AssistantWithToolCalls {
            content: (!text.is_empty()).then_some(text),
            tool_calls: serde_json::from_str::<Vec<ToolCall>>(tool_calls_json.unwrap_or("[]"))
                .map_err(|_| repository_error("decode_tool_calls"))?,
        }),
        "tool_result" => Ok(ChatMessage::ToolResult(ToolResult {
            tool_call_id: tool_call_id.unwrap_or_default().to_string(),
            content: text,
            is_error,
        })),
        _ => Err(repository_error("unknown_role")),
    }
}

fn text_parts(content: &str) -> String {
    json(&vec![MessagePart::Text {
        text: content.to_string(),
    }])
}

fn json(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).expect("application message serialization")
}

fn parse_uuid(value: String, operation: &'static str) -> Result<Uuid, RepositoryError> {
    Uuid::parse_str(&value).map_err(|_| repository_error(operation))
}

pub(crate) fn repository_error(operation: &'static str) -> RepositoryError {
    RepositoryError::Operation {
        operation,
        code: "sqlite",
    }
}
