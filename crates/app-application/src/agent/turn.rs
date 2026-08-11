use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use futures::stream::{self, BoxStream};
use futures::StreamExt;
use serde_json::{json, Value};
use thiserror::Error;
use tokio::sync::mpsc;

use crate::models::agent::{AgentEvent, AgentTurnRequest};
use crate::models::chat::{
    ChatChunk, ChatMessage, ChatRequest, FinishReason, MessagePart, ReasoningSpec, ToolCall,
    ToolResult,
};
use crate::ports::events::{ApplicationEvent, ApplicationEventSink};
use crate::ports::llm::LlmProvider;
use crate::ports::repositories::MessageRepository;

use super::context::{AgentContextPort, AgentContextRequest};
use super::tool_decoder::decode_tool_call;
use super::tool_dispatch::{
    GeneratedAgentMedia, RuntimeCoordinator, ToolDispatchRequest, ToolDispatcher,
};
use super::tools::{classify_handler, is_media_tool, AgentToolCatalog, ToolAvailability};

#[derive(Debug, Clone)]
pub struct AgentTurnConfig {
    pub model: String,
    pub system_prompt: String,
    pub temperature: f32,
    pub max_rounds: usize,
    pub embedding_model: String,
    pub reasoning_enabled: bool,
    pub reasoning_budget: ReasoningSpec,
    pub tool_availability: ToolAvailability,
    pub tools: AgentToolCatalog,
}

#[derive(Debug, Clone)]
pub struct AgentTurnCommand {
    pub request_id: String,
    pub request: AgentTurnRequest,
}

pub trait CancellationSignal: Send + Sync {
    fn is_cancelled(&self) -> bool;
}

pub struct NeverCancelled;

impl CancellationSignal for NeverCancelled {
    fn is_cancelled(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum AgentTurnError {
    #[error("duplicate agent request")]
    DuplicateRequest,
    #[error("agent request cancelled")]
    Cancelled,
    #[error("agent context failed with code {0}")]
    Context(&'static str),
    #[error("LLM provider failed")]
    Provider,
    #[error("message persistence failed")]
    Persistence,
    #[error("tool dispatch failed with code {0}")]
    ToolDispatch(&'static str),
    #[error("runtime coordination failed with code {0}")]
    Runtime(&'static str),
    #[error("event publication failed with code {0}")]
    EventPublication(&'static str),
}

pub struct AgentTurnService {
    provider: Arc<dyn LlmProvider>,
    context: Arc<dyn AgentContextPort>,
    messages: Arc<dyn MessageRepository>,
    tools: Arc<dyn ToolDispatcher>,
    runtime: Arc<dyn RuntimeCoordinator>,
    events: Arc<dyn ApplicationEventSink>,
    cancellation: Arc<dyn CancellationSignal>,
    config: AgentTurnConfig,
    claimed_requests: Mutex<HashSet<String>>,
}

impl AgentTurnService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        provider: Arc<dyn LlmProvider>,
        context: Arc<dyn AgentContextPort>,
        messages: Arc<dyn MessageRepository>,
        tools: Arc<dyn ToolDispatcher>,
        runtime: Arc<dyn RuntimeCoordinator>,
        events: Arc<dyn ApplicationEventSink>,
        cancellation: Arc<dyn CancellationSignal>,
        config: AgentTurnConfig,
    ) -> Self {
        Self {
            provider,
            context,
            messages,
            tools,
            runtime,
            events,
            cancellation,
            config,
            claimed_requests: Mutex::new(HashSet::new()),
        }
    }

    pub fn execute(
        self: Arc<Self>,
        command: AgentTurnCommand,
    ) -> BoxStream<'static, Result<AgentEvent, AgentTurnError>> {
        let claimed = self
            .claimed_requests
            .lock()
            .expect("agent request lock poisoned")
            .insert(command.request_id.clone());
        if !claimed {
            return stream::once(async { Err(AgentTurnError::DuplicateRequest) }).boxed();
        }

        let (tx, rx) = mpsc::channel(64);
        tokio::spawn(async move {
            let request_id = command.request_id.clone();
            if let Err(error) = self.run(command, &tx).await {
                tracing::error!(
                    request_id,
                    error_code = error_code(&error),
                    "agent turn failed"
                );
                self.claimed_requests
                    .lock()
                    .expect("agent request lock poisoned")
                    .remove(&request_id);
                let _ = tx.send(Err(error)).await;
            }
        });
        stream::unfold(rx, |mut receiver| async move {
            receiver.recv().await.map(|item| (item, receiver))
        })
        .boxed()
    }

    #[tracing::instrument(skip_all, fields(
        request_id = %command.request_id,
        session_id = %command.request.session_id,
        campaign_id = %command.request.campaign_id,
    ))]
    async fn run(
        &self,
        command: AgentTurnCommand,
        tx: &mpsc::Sender<Result<AgentEvent, AgentTurnError>>,
    ) -> Result<(), AgentTurnError> {
        let started_at = Instant::now();
        self.ensure_not_cancelled()?;
        let request = command.request;
        let user_message = user_message(&request);
        self.messages
            .append(request.session_id, user_message.clone())
            .await
            .map_err(|_| AgentTurnError::Persistence)?;

        let context = self
            .context
            .build(AgentContextRequest {
                campaign_id: request.campaign_id,
                player_message: request.player_message.clone(),
                base_system_prompt: self.config.system_prompt.clone(),
                embedding_model: self.config.embedding_model.clone(),
            })
            .await
            .map_err(|error| AgentTurnError::Context(error.code))?;
        let system_prompt = append_board(context.system_prompt, request.board.as_deref());
        let tools = self.config.tools.for_combat_state(context.combat_active);
        let mut messages = request.history;
        messages.push(user_message);
        let mut sequence = 0_u64;
        let mut total_rounds = 0_usize;

        for round_index in 0..self.config.max_rounds {
            self.ensure_not_cancelled()?;
            total_rounds = round_index + 1;
            let chat_request = ChatRequest {
                messages: messages.clone(),
                model: self.config.model.clone(),
                max_tokens: Some(2048),
                temperature: Some(self.config.temperature),
                tools: tools.clone(),
                system_prompt: Some(system_prompt.clone()),
                reasoning: self
                    .config
                    .reasoning_enabled
                    .then_some(self.config.reasoning_budget),
            };
            let mut chunks = self
                .provider
                .stream_chat(chat_request)
                .await
                .map_err(|_| AgentTurnError::Provider)?;
            let mut round_text = String::new();
            let mut tool_buffers: HashMap<String, (String, String)> = HashMap::new();
            let mut tool_calls = Vec::new();
            let mut finish_reason = FinishReason::Stop;

            while let Some(chunk) = chunks.next().await {
                self.ensure_not_cancelled()?;
                match chunk.map_err(|_| AgentTurnError::Provider)? {
                    ChatChunk::ThinkingDelta { text } => {
                        self.emit(
                            request.session_id,
                            &mut sequence,
                            AgentEvent::ReasoningText { text },
                            tx,
                        )
                        .await?;
                    }
                    ChatChunk::TextDelta { text } => {
                        round_text.push_str(&text);
                        self.emit(
                            request.session_id,
                            &mut sequence,
                            AgentEvent::TextDelta { text },
                            tx,
                        )
                        .await?;
                    }
                    ChatChunk::ToolCallStart { id, name } => {
                        tool_buffers.insert(id.clone(), (name.clone(), String::new()));
                        self.emit(
                            request.session_id,
                            &mut sequence,
                            AgentEvent::ToolCallStart {
                                id,
                                tool_name: name,
                                round: total_rounds,
                            },
                            tx,
                        )
                        .await?;
                    }
                    ChatChunk::ToolCallArgsDelta { id, args_fragment } => {
                        if let Some((_, args)) = tool_buffers.get_mut(&id) {
                            args.push_str(&args_fragment);
                        }
                    }
                    ChatChunk::ToolCallDone { id } => {
                        if let Some((name, args)) = tool_buffers.remove(&id) {
                            tool_calls.push(ToolCall {
                                id,
                                name,
                                args: serde_json::from_str(&args)
                                    .unwrap_or_else(|_| Value::Object(Default::default())),
                            });
                        }
                    }
                    ChatChunk::Done { reason } => {
                        finish_reason = reason;
                        break;
                    }
                }
            }

            let assistant_message = if tool_calls.is_empty() {
                (!round_text.is_empty()).then(|| ChatMessage::Assistant {
                    content: round_text.clone(),
                })
            } else {
                Some(ChatMessage::AssistantWithToolCalls {
                    content: (!round_text.is_empty()).then_some(round_text.clone()),
                    tool_calls: tool_calls.clone(),
                })
            };
            if let Some(message) = assistant_message {
                self.messages
                    .append(request.session_id, message.clone())
                    .await
                    .map_err(|_| AgentTurnError::Persistence)?;
                messages.push(message);
            }

            for tool_call in &tool_calls {
                self.ensure_not_cancelled()?;
                let execution = match decode_tool_call(&tool_call.name, &tool_call.args) {
                    Ok(decoded) => {
                        if is_media_tool(&tool_call.name) {
                            self.runtime
                                .acquire_for(&tool_call.name)
                                .await
                                .map_err(|error| AgentTurnError::Runtime(error.code))?;
                        }
                        let result = self
                            .tools
                            .execute(ToolDispatchRequest {
                                request_id: command.request_id.clone(),
                                tool_call_id: tool_call.id.clone(),
                                campaign_id: request.campaign_id,
                                session_id: request.session_id,
                                command: decoded,
                            })
                            .await;
                        if is_media_tool(&tool_call.name) {
                            self.runtime
                                .release_for(&tool_call.name)
                                .await
                                .map_err(|error| AgentTurnError::Runtime(error.code))?;
                        }
                        result.map_err(|error| AgentTurnError::ToolDispatch(error.code))?
                    }
                    Err(error) => {
                        tracing::warn!(
                            tool_call_id = tool_call.id,
                            tool_name = tool_call.name,
                            "rejected invalid model tool call"
                        );
                        super::tool_dispatch::ToolExecution {
                            result: json!({ "error": error.to_string() }),
                            is_error: true,
                            handled_by: classify_handler(&tool_call.name).to_string(),
                            media: Vec::new(),
                        }
                    }
                };

                for media in execution.media {
                    let event = match media {
                        GeneratedAgentMedia::Image {
                            mime_type,
                            data_b64,
                            kind,
                        } => AgentEvent::ImageGenerated {
                            tool_call_id: tool_call.id.clone(),
                            round: total_rounds,
                            mime_type,
                            image_b64: data_b64,
                            kind,
                        },
                        GeneratedAgentMedia::Video {
                            mime_type,
                            data_b64,
                            kind,
                        } => AgentEvent::VideoGenerated {
                            tool_call_id: tool_call.id.clone(),
                            round: total_rounds,
                            mime_type,
                            video_b64: data_b64,
                            kind,
                        },
                    };
                    self.emit(request.session_id, &mut sequence, event, tx)
                        .await?;
                }

                self.emit(
                    request.session_id,
                    &mut sequence,
                    AgentEvent::ToolCallResult {
                        id: tool_call.id.clone(),
                        tool_name: tool_call.name.clone(),
                        args: tool_call.args.clone(),
                        result: execution.result.clone(),
                        is_error: execution.is_error,
                        round: total_rounds,
                        handled_by: execution.handled_by,
                    },
                    tx,
                )
                .await?;
                let tool_result = ChatMessage::ToolResult(ToolResult {
                    tool_call_id: tool_call.id.clone(),
                    content: serde_json::to_string(&execution.result).unwrap_or_default(),
                    is_error: execution.is_error,
                });
                self.messages
                    .append(request.session_id, tool_result.clone())
                    .await
                    .map_err(|_| AgentTurnError::Persistence)?;
                messages.push(tool_result);
            }

            if !matches!(finish_reason, FinishReason::ToolUse) || tool_calls.is_empty() {
                break;
            }
        }

        self.emit(
            request.session_id,
            &mut sequence,
            AgentEvent::AgentDone { total_rounds },
            tx,
        )
        .await?;
        tracing::info!(
            total_rounds,
            event_count = sequence,
            elapsed_ms = started_at.elapsed().as_millis(),
            "agent turn completed"
        );
        Ok(())
    }

    fn ensure_not_cancelled(&self) -> Result<(), AgentTurnError> {
        if self.cancellation.is_cancelled() {
            Err(AgentTurnError::Cancelled)
        } else {
            Ok(())
        }
    }

    async fn emit(
        &self,
        session_id: uuid::Uuid,
        sequence: &mut u64,
        event: AgentEvent,
        tx: &mpsc::Sender<Result<AgentEvent, AgentTurnError>>,
    ) -> Result<(), AgentTurnError> {
        *sequence += 1;
        tracing::debug!(
            sequence = *sequence,
            event_kind = agent_event_kind(&event),
            "publishing agent event"
        );
        self.events
            .publish(ApplicationEvent::Agent {
                session_id,
                sequence: *sequence,
                event: event.clone(),
            })
            .await
            .map_err(|error| AgentTurnError::EventPublication(error.code))?;
        tx.send(Ok(event))
            .await
            .map_err(|_| AgentTurnError::Cancelled)
    }
}

fn agent_event_kind(event: &AgentEvent) -> &'static str {
    match event {
        AgentEvent::TextDelta { .. } => "text_delta",
        AgentEvent::ToolCallStart { .. } => "tool_call_start",
        AgentEvent::ToolCallResult { .. } => "tool_call_result",
        AgentEvent::ReasoningText { .. } => "reasoning_text",
        AgentEvent::ImageGenerated { .. } => "image_generated",
        AgentEvent::VideoGenerated { .. } => "video_generated",
        AgentEvent::AgentDone { .. } => "agent_done",
    }
}

fn error_code(error: &AgentTurnError) -> &'static str {
    match error {
        AgentTurnError::DuplicateRequest => "duplicate_request",
        AgentTurnError::Cancelled => "cancelled",
        AgentTurnError::Context(_) => "context",
        AgentTurnError::Provider => "provider",
        AgentTurnError::Persistence => "persistence",
        AgentTurnError::ToolDispatch(_) => "tool_dispatch",
        AgentTurnError::Runtime(_) => "runtime",
        AgentTurnError::EventPublication(_) => "event_publication",
    }
}

fn user_message(request: &AgentTurnRequest) -> ChatMessage {
    if request.images.is_empty() {
        ChatMessage::user_text(request.player_message.clone())
    } else {
        let mut parts = vec![MessagePart::Text {
            text: request.player_message.clone(),
        }];
        parts.extend(request.images.iter().cloned());
        ChatMessage::User { parts }
    }
}

fn append_board(mut system_prompt: String, board: Option<&str>) -> String {
    if let Some(board) = board.map(str::trim).filter(|board| !board.is_empty()) {
        system_prompt.push_str("\n\n## Current battlefield\n");
        system_prompt.push_str(board);
    }
    system_prompt
}
