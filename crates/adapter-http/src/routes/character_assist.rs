use std::collections::HashMap;
use std::convert::Infallible;

use app_application::models::chat::{ChatChunk, ChatMessage, ChatRequest, Tool};
use axum::extract::Extension;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::Json;
use futures::{Stream, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::services::{HttpServiceError, HttpServices};

const FIELD_EN: &str = include_str!("../../../../prompts/character_assist_field_en.txt");
const FIELD_RU: &str = include_str!("../../../../prompts/character_assist_field_ru.txt");
const FULL_EN: &str = include_str!("../../../../prompts/character_assist_full_en.txt");
const FULL_RU: &str = include_str!("../../../../prompts/character_assist_full_ru.txt");
const CHAT_EN: &str = include_str!("../../../../prompts/character_assist_chat_en.txt");
const CHAT_RU: &str = include_str!("../../../../prompts/character_assist_chat_ru.txt");
const FLAG_EN: &str = include_str!("../../../../prompts/character_assist_flag_en.txt");
const FLAG_RU: &str = include_str!("../../../../prompts/character_assist_flag_ru.txt");

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssistKind {
    Field,
    Full,
    TestChat,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssistField {
    Name,
    Backstory,
    Ideals,
    Bonds,
    Flaws,
    PortraitPrompt,
    PersonalityFlag,
    ItemName,
}

#[derive(Debug, Deserialize)]
pub struct TestChatTurn {
    pub role: String,
    pub text: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum AssistParams {
    Field {
        field: AssistField,
        #[serde(default)]
        slot_id: Option<String>,
        #[serde(default)]
        source: Option<String>,
        #[serde(default)]
        source_label: Option<String>,
        #[serde(default)]
        pool: Option<Vec<String>>,
    },
    TestChat {
        user_message: String,
        history: Vec<TestChatTurn>,
    },
    Full {},
}

#[derive(Debug, Deserialize)]
pub struct CharacterAssistReq {
    pub kind: AssistKind,
    pub context: Value,
    pub params: AssistParams,
    pub locale: String,
}

fn system_prompt(request: &CharacterAssistReq) -> &'static str {
    let ru = request.locale.starts_with("ru");
    match (&request.kind, &request.params) {
        (
            AssistKind::Field,
            AssistParams::Field {
                field: AssistField::PersonalityFlag,
                ..
            },
        ) => {
            if ru {
                FLAG_RU
            } else {
                FLAG_EN
            }
        }
        (AssistKind::Field, _) => {
            if ru {
                FIELD_RU
            } else {
                FIELD_EN
            }
        }
        (AssistKind::Full, _) => {
            if ru {
                FULL_RU
            } else {
                FULL_EN
            }
        }
        (AssistKind::TestChat, _) => {
            if ru {
                CHAT_RU
            } else {
                CHAT_EN
            }
        }
    }
}

fn user_text(request: &CharacterAssistReq) -> Result<String, HttpServiceError> {
    let context =
        serde_json::to_string(&request.context).map_err(|_| HttpServiceError::BadRequest {
            code: "context_invalid",
        })?;
    match &request.params {
        AssistParams::Field {
            field: AssistField::PersonalityFlag,
            slot_id,
            source,
            source_label,
            pool,
        } => {
            let slot = slot_id.as_ref().ok_or(HttpServiceError::BadRequest {
                code: "missing_slot_id",
            })?;
            let source = source.as_deref().unwrap_or("unknown");
            let label = source_label.as_deref().unwrap_or("");
            let pool = pool
                .as_ref()
                .filter(|items| !items.is_empty())
                .map(|items| {
                    format!(
                        "Pool:\n{}",
                        items
                            .iter()
                            .map(|item| format!("- {item}"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                })
                .unwrap_or_else(|| "Pool: (empty - generate a fresh entry)".into());
            let source = if label.is_empty() {
                format!("Source: {source}")
            } else {
                format!("Source: {source} ({label})")
            };
            Ok(format!("Slot: {slot}\n{source}\n{pool}\nDraft: {context}"))
        }
        AssistParams::Field { field, .. } => Ok(format!("Field: {field:?}\nDraft: {context}")),
        AssistParams::Full {} => Ok(format!("Draft: {context}\nFill in remaining fields.")),
        AssistParams::TestChat {
            user_message,
            history,
        } => {
            let history = serde_json::to_string(
                &history
                    .iter()
                    .map(|turn| json!({"role": turn.role, "text": turn.text}))
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_else(|_| "[]".into());
            Ok(format!(
                "Character: {context}\nHistory: {history}\nUser: {user_message}"
            ))
        }
    }
}

async fn send(sender: &mpsc::Sender<Result<Event, Infallible>>, name: &str, value: Value) -> bool {
    sender
        .send(Ok(Event::default().event(name).data(
            serde_json::to_string(&value).unwrap_or_else(|_| "{}".into()),
        )))
        .await
        .is_ok()
}

pub async fn post_character_assist(
    Extension(services): Extension<HttpServices>,
    Json(request): Json<CharacterAssistReq>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, HttpServiceError> {
    let is_full = request.kind == AssistKind::Full;
    let prompt = system_prompt(&request).to_string();
    let text = user_text(&request)?;
    let tools = if is_full {
        vec![Tool {
            name: "apply_character_patch".into(),
            description: "Apply a JSON patch to the character draft.".into(),
            parameters: json!({"type":"object","additionalProperties":true}),
        }]
    } else {
        Vec::new()
    };
    let mut chunks = services
        .character_assist
        .stream(ChatRequest {
            messages: vec![ChatMessage::user_text(text)],
            model: String::new(),
            max_tokens: None,
            temperature: Some(0.7),
            tools,
            system_prompt: Some(prompt),
            reasoning: None,
        })
        .await?;
    let (sender, receiver) = mpsc::channel(32);
    tokio::spawn(async move {
        let mut calls: HashMap<String, (String, String)> = HashMap::new();
        let mut saw_patch = !is_full;
        while let Some(chunk) = chunks.next().await {
            match chunk {
                Ok(ChatChunk::TextDelta { text }) => {
                    if !send(&sender, "token", json!({"type":"token","text":text})).await {
                        return;
                    }
                }
                Ok(ChatChunk::ToolCallStart { id, name }) => {
                    calls.insert(id, (name, String::new()));
                }
                Ok(ChatChunk::ToolCallArgsDelta { id, args_fragment }) => {
                    if let Some((_, args)) = calls.get_mut(&id) {
                        args.push_str(&args_fragment);
                    }
                }
                Ok(ChatChunk::ToolCallDone { id }) => {
                    if let Some((name, args)) = calls.remove(&id) {
                        if is_full && name == "apply_character_patch" {
                            saw_patch = true;
                            let patch = serde_json::from_str(&args).unwrap_or_else(|_| json!({}));
                            if !send(
                                &sender,
                                "draft_patch",
                                json!({"type":"draft_patch","patch":patch}),
                            )
                            .await
                            {
                                return;
                            }
                        }
                    }
                }
                Ok(ChatChunk::Done { .. }) => break,
                Ok(ChatChunk::ThinkingDelta { .. }) => {}
                Err(_) => {
                    let _ = send(
                        &sender,
                        "error",
                        json!({"type":"error","code":"stream_error","message":"stream_error"}),
                    )
                    .await;
                    break;
                }
            }
        }
        if !saw_patch {
            let _ = send(&sender, "error", json!({"type":"error","code":"invalid_patch","message":"full-mode response did not call apply_character_patch"})).await;
        }
        let _ = send(&sender, "done", json!({"type":"done"})).await;
    });
    Ok(Sse::new(ReceiverStream::new(receiver)).keep_alive(KeepAlive::default()))
}
