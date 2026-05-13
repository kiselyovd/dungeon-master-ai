use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::StreamExt;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::convert::Infallible;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use app_llm::{ChatChunk, ChatMessage, ChatRequest, Tool};

use crate::error::AppError;
use crate::state::AppState;

const PROMPT_FIELD_EN: &str =
    include_str!("../../../../prompts/character_assist_field_en.txt");
const PROMPT_FIELD_RU: &str =
    include_str!("../../../../prompts/character_assist_field_ru.txt");
const PROMPT_FULL_EN: &str =
    include_str!("../../../../prompts/character_assist_full_en.txt");
const PROMPT_FULL_RU: &str =
    include_str!("../../../../prompts/character_assist_full_ru.txt");
const PROMPT_CHAT_EN: &str =
    include_str!("../../../../prompts/character_assist_chat_en.txt");
const PROMPT_CHAT_RU: &str =
    include_str!("../../../../prompts/character_assist_chat_ru.txt");

// ---------------------------------------------------------------------------
// Wire types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CharacterAssistReq {
    pub kind: AssistKind,
    pub context: serde_json::Value,
    pub params: AssistParams,
    pub locale: String,
}

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
    },
    TestChat {
        user_message: String,
        history: Vec<TestChatTurn>,
    },
    /// Empty object - MUST be last to avoid eating other variants.
    Full {},
}

// ---------------------------------------------------------------------------
// Tool schema
// ---------------------------------------------------------------------------

fn apply_character_patch_tool() -> Tool {
    Tool {
        name: "apply_character_patch".into(),
        description: "Apply a JSON patch to the character draft.".into(),
        parameters: json!({
            "type": "object",
            "additionalProperties": true,
        }),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn pick_prompt(kind: &AssistKind, locale: &str) -> &'static str {
    let is_ru = locale.starts_with("ru");
    match kind {
        AssistKind::Field => {
            if is_ru { PROMPT_FIELD_RU } else { PROMPT_FIELD_EN }
        }
        AssistKind::Full => {
            if is_ru { PROMPT_FULL_RU } else { PROMPT_FULL_EN }
        }
        AssistKind::TestChat => {
            if is_ru { PROMPT_CHAT_RU } else { PROMPT_CHAT_EN }
        }
    }
}

fn build_user_text(req: &CharacterAssistReq) -> Result<String, AppError> {
    let context_json = serde_json::to_string(&req.context)
        .map_err(|e| AppError::BadRequest(format!("invalid context: {e}")))?;
    match &req.params {
        AssistParams::Field { field } => {
            Ok(format!("Field: {field:?}\nDraft: {context_json}"))
        }
        AssistParams::Full {} => {
            Ok(format!("Draft: {context_json}\nFill in remaining fields."))
        }
        AssistParams::TestChat { user_message, history } => {
            let history_json = serde_json::to_string(
                &history
                    .iter()
                    .map(|t| json!({"role": t.role, "text": t.text}))
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_else(|_| "[]".into());
            Ok(format!(
                "Character: {context_json}\nHistory: {history_json}\nUser: {user_message}"
            ))
        }
    }
}

async fn send_event(
    tx: &mpsc::Sender<Result<Event, Infallible>>,
    name: &str,
    payload: &serde_json::Value,
) -> Result<(), mpsc::error::SendError<Result<Event, Infallible>>> {
    let data = serde_json::to_string(payload).unwrap_or_else(|_| "{}".into());
    tx.send(Ok(Event::default().event(name).data(data))).await
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn post_character_assist(
    State(state): State<AppState>,
    Json(req): Json<CharacterAssistReq>,
) -> Result<impl IntoResponse, AppError> {
    let provider = state.provider();
    let model = state.default_model();
    let prompt = pick_prompt(&req.kind, &req.locale);
    let user_text = build_user_text(&req)?;

    let is_full = req.kind == AssistKind::Full;
    let tools = if is_full {
        vec![apply_character_patch_tool()]
    } else {
        vec![]
    };

    let chat_req = ChatRequest {
        messages: vec![ChatMessage::user_text(user_text)],
        model,
        max_tokens: None,
        temperature: Some(0.7),
        tools,
        system_prompt: Some(prompt.into()),
    };

    let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(32);

    tokio::spawn(async move {
        let mut stream = match provider.stream_chat(chat_req).await {
            Ok(s) => s,
            Err(e) => {
                let _ = send_event(
                    &tx,
                    "error",
                    &json!({"type":"error","code":"provider_error","message": e.to_string()}),
                )
                .await;
                let _ = send_event(&tx, "done", &json!({"type":"done"})).await;
                return;
            }
        };

        // Tool-call accumulator: id -> (name, accumulated_args_str)
        let mut tc_buf: HashMap<String, (String, String)> = HashMap::new();
        // For field/test_chat we treat it as already satisfied (no patch needed).
        // For full we require apply_character_patch to be called.
        let mut saw_apply_patch = !is_full;

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(ChatChunk::TextDelta { text }) => {
                    if send_event(
                        &tx,
                        "token",
                        &json!({"type":"token","text":text}),
                    )
                    .await
                    .is_err()
                    {
                        return;
                    }
                }
                Ok(ChatChunk::ToolCallStart { id, name }) => {
                    tc_buf.insert(id, (name, String::new()));
                }
                Ok(ChatChunk::ToolCallArgsDelta { id, args_fragment }) => {
                    if let Some((_, buf)) = tc_buf.get_mut(&id) {
                        buf.push_str(&args_fragment);
                    }
                }
                Ok(ChatChunk::ToolCallDone { id }) => {
                    let Some((name, args_str)) = tc_buf.remove(&id) else {
                        continue;
                    };
                    if !is_full {
                        // Unexpected tool call in field/test_chat - ignore silently.
                        continue;
                    }
                    if name != "apply_character_patch" {
                        let _ = send_event(
                            &tx,
                            "error",
                            &json!({
                                "type":"error",
                                "code":"invalid_patch",
                                "message": format!("unexpected tool: {}", name)
                            }),
                        )
                        .await;
                        continue;
                    }
                    let patch: serde_json::Value =
                        serde_json::from_str(&args_str).unwrap_or_else(|_| json!({}));
                    saw_apply_patch = true;
                    if send_event(
                        &tx,
                        "draft_patch",
                        &json!({"type":"draft_patch","patch": patch}),
                    )
                    .await
                    .is_err()
                    {
                        return;
                    }
                }
                Ok(ChatChunk::Done { .. }) => break,
                Err(e) => {
                    let _ = send_event(
                        &tx,
                        "error",
                        &json!({"type":"error","code":"stream_error","message": e.to_string()}),
                    )
                    .await;
                    break;
                }
            }
        }

        if !saw_apply_patch {
            let _ = send_event(
                &tx,
                "error",
                &json!({
                    "type":"error",
                    "code":"invalid_patch",
                    "message":"full-mode response did not call apply_character_patch"
                }),
            )
            .await;
        }
        let _ = send_event(&tx, "done", &json!({"type":"done"})).await;
    });

    let stream = ReceiverStream::new(rx);
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}
