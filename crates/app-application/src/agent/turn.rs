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
    pub player_verifier: Option<Arc<dyn super::intentions::PlayerIntentionVerifier>>,
}

#[derive(Debug, Clone)]
pub struct AgentTurnCommand {
    pub request_id: String,
    pub request: AgentTurnRequest,
}

#[async_trait::async_trait]
pub trait CancellationSignal: Send + Sync {
    fn is_cancelled(&self) -> bool;
    async fn cancelled(&self) {
        while !self.is_cancelled() {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }
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
        self.execute_recovered(command, None)
    }
    pub fn execute_recovered(
        self: Arc<Self>,
        command: AgentTurnCommand,
        checkpoint: Option<Vec<ChatMessage>>,
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
            if let Err(error) = self.run(command, &tx, checkpoint).await {
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
        checkpoint: Option<Vec<ChatMessage>>,
    ) -> Result<(), AgentTurnError> {
        let started_at = Instant::now();
        self.ensure_not_cancelled()?;
        let request = command.request;
        let user_message = user_message(&request);
        let checkpoint = checkpoint.map(|mut messages| {
            // A whitespace draft is not a completed DM answer, including older checkpoints.
            while matches!(messages.last(),Some(ChatMessage::Assistant {content}) if content.trim().is_empty()) {messages.pop();}
            messages
        }).filter(|messages| !messages.is_empty());
        if let Some(ChatMessage::Assistant { content }) =
            checkpoint.as_ref().and_then(|saved| saved.last())
        {
            let mut sequence = 0;
            self.emit(
                request.session_id,
                &mut sequence,
                AgentEvent::TextDelta {
                    text: content.clone(),
                },
                tx,
            )
            .await?;
            self.emit(
                request.session_id,
                &mut sequence,
                AgentEvent::AgentDone { total_rounds: 0 },
                tx,
            )
            .await?;
            return Ok(());
        }
        if checkpoint.is_none() {
            self.messages
                .append(request.session_id, user_message.clone())
                .await
                .map_err(|_| AgentTurnError::Persistence)?;
        }

        let mut context = self
            .context
            .build(AgentContextRequest {
                campaign_id: request.campaign_id,
                player_message: request.player_message.clone(),
                base_system_prompt: self.config.system_prompt.clone(),
                embedding_model: self.config.embedding_model.clone(),
            })
            .await
            .map_err(|error| AgentTurnError::Context(error.code))?;
        let mut system_prompt =
            append_board(context.system_prompt.clone(), request.board.as_deref());
        let controlled_turn = self.config.player_verifier.is_some();
        let mut resolved_intentions = if controlled_turn {
            checkpoint
                .as_ref()
                .map(|saved| resolved_player_intentions(saved))
                .transpose()?
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let mut player_intention_resolved = false;
        if controlled_turn && !resolved_intentions.is_empty() {
            let decision = self
                .assess_player(
                    &request.player_message,
                    None,
                    &resolved_intentions,
                    &request.history,
                    &context.public_intention_context,
                )
                .await?;
            player_intention_resolved = !decision.further_requested;
        }
        let continuing = checkpoint.is_some();
        if continuing {
            system_prompt.push_str("\nThis is an interrupted turn continuation. Recorded tool results are already confirmed. Finish the original intention from those results; do not repeat confirmed effects. Provide a coherent reply without replaying an unfinished draft.");
        }
        let mut tools = self
            .config
            .tools
            .for_context(context.combat_active, context.adventure_active);
        restrict_to_narration(&mut tools, &mut system_prompt, player_intention_resolved);
        append_rejection_brief(&mut system_prompt, &resolved_intentions);
        let mut pending_recovered = Vec::new();
        if let Some(saved) = &checkpoint {
            let completed = saved
                .iter()
                .filter_map(|message| match message {
                    ChatMessage::ToolResult(result) => Some(result.tool_call_id.clone()),
                    _ => None,
                })
                .collect::<HashSet<_>>();
            for message in saved {
                if let ChatMessage::AssistantWithToolCalls { tool_calls, .. } = message {
                    pending_recovered.extend(
                        tool_calls
                            .iter()
                            .filter(|call| !completed.contains(&call.id))
                            .cloned(),
                    );
                }
            }
        }
        let mut messages = request.history;
        if let Some(saved) = checkpoint {
            messages.extend(saved);
        } else {
            messages.push(user_message);
        }
        let mut sequence = 0_u64;
        let mut total_rounds = 0_usize;
        let mut final_reply_confirmed = false;

        for round_index in 0..self.config.max_rounds {
            self.ensure_not_cancelled()?;
            total_rounds = round_index + 1;
            let mut tool_calls = if round_index == 0 {
                std::mem::take(&mut pending_recovered)
            } else {
                Vec::new()
            };
            let recovering_tools = !tool_calls.is_empty();
            let mut finish_reason = if recovering_tools {
                FinishReason::ToolUse
            } else {
                FinishReason::Stop
            };
            if !recovering_tools {
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
                let mut chunks = tokio::select! {
                    biased;
                    _ = self.cancellation.cancelled() => return Err(AgentTurnError::Cancelled),
                    chunks = self.provider.stream_chat(chat_request) => chunks.map_err(|error| {
                        tracing::warn!(error_code = safe_provider_error_code(&error), "LLM response unavailable");
                        AgentTurnError::Provider
                    })?,
                };
                let mut provider_done = false;
                let mut round_text = String::new();
                let mut tool_buffers: HashMap<String, (String, String)> = HashMap::new();
                let mut pending_delta: Option<AgentEvent> = None;

                loop {
                    let chunk = tokio::select! {
                        biased;
                        _ = self.cancellation.cancelled() => return Err(AgentTurnError::Cancelled),
                        chunk = chunks.next() => match chunk {Some(chunk)=>chunk,None=>break},
                    };
                    self.ensure_not_cancelled()?;
                    match chunk.map_err(|_| AgentTurnError::Provider)? {
                        ChatChunk::ThinkingDelta { text } => {
                            if !context.reasoning_is_private {
                                self.push_delta(
                                    request.session_id,
                                    &mut sequence,
                                    &mut pending_delta,
                                    AgentEvent::ReasoningText { text },
                                    tx,
                                )
                                .await?;
                            }
                        }
                        ChatChunk::TextDelta { text } => {
                            round_text.push_str(&text);
                            if !controlled_turn {
                                self.push_delta(
                                    request.session_id,
                                    &mut sequence,
                                    &mut pending_delta,
                                    AgentEvent::TextDelta { text },
                                    tx,
                                )
                                .await?;
                            }
                        }
                        ChatChunk::ToolCallStart { id, name } => {
                            self.flush_delta(
                                request.session_id,
                                &mut sequence,
                                &mut pending_delta,
                                tx,
                            )
                            .await?;
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
                            provider_done = true;
                            self.flush_delta(
                                request.session_id,
                                &mut sequence,
                                &mut pending_delta,
                                tx,
                            )
                            .await?;
                            finish_reason = reason;
                            break;
                        }
                    }
                }
                let valid_terminal_round = match finish_reason {
                    FinishReason::Stop => tool_calls.is_empty() && !round_text.trim().is_empty(),
                    FinishReason::ToolUse => !tool_calls.is_empty(),
                    FinishReason::Length | FinishReason::Error => false,
                };
                if !provider_done || !tool_buffers.is_empty() || !valid_terminal_round {
                    tracing::warn!(
                        error_code = "provider_terminal_round_invalid",
                        "LLM round incomplete"
                    );
                    return Err(AgentTurnError::Provider);
                }
                self.flush_delta(request.session_id, &mut sequence, &mut pending_delta, tx)
                    .await?;

                if controlled_turn && matches!(finish_reason, FinishReason::Stop) {
                    let decision = self
                        .assess_player(
                            &request.player_message,
                            None,
                            &resolved_intentions,
                            &messages,
                            &context.public_intention_context,
                        )
                        .await?;
                    if decision.further_requested {
                        player_intention_resolved = false;
                        // A pre-execution forecast cannot truncate a compound
                        // request. Restore the current canonical catalog when
                        // the outcome ledger still has a requested action.
                        tools = self
                            .config
                            .tools
                            .for_context(context.combat_active, context.adventure_active);
                        system_prompt =
                            append_board(context.system_prompt.clone(), request.board.as_deref());
                        append_rejection_brief(&mut system_prompt, &resolved_intentions);
                        system_prompt.push_str("\nThe current human's mechanical request is NOT resolved. Do not claim travel, combat, rescue, repair or completion through prose. Use the available canonical tool for the remaining requested step; ask a concise clarification if it cannot be determined. Do not add a different action.");
                        continue;
                    }
                    player_intention_resolved = true;
                }
                let assistant_message = if tool_calls.is_empty() {
                    (!round_text.trim().is_empty()).then(|| ChatMessage::Assistant {
                        content: round_text.clone(),
                    })
                } else {
                    Some(ChatMessage::AssistantWithToolCalls {
                        content: (!round_text.trim().is_empty()).then_some(round_text.clone()),
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
                if controlled_turn && matches!(finish_reason, FinishReason::Stop) {
                    self.emit(
                        request.session_id,
                        &mut sequence,
                        AgentEvent::TextDelta {
                            text: round_text.clone(),
                        },
                        tx,
                    )
                    .await?;
                }
            }

            for tool_call in &tool_calls {
                self.ensure_not_cancelled()?;
                let mut interception = None;
                let mut permission = None;
                if controlled_turn && legacy_world_mechanics(&tool_call.name) {
                    if let Ok(decoded) = decode_tool_call(&tool_call.name, &tool_call.args) {
                        interception = self
                            .tools
                            .replay(ToolDispatchRequest {
                                request_id: command.request_id.clone(),
                                tool_call_id: tool_call.id.clone(),
                                campaign_id: request.campaign_id,
                                session_id: request.session_id,
                                command: decoded,
                            })
                            .await
                            .map_err(|error| AgentTurnError::ToolDispatch(error.code))?;
                    }
                }
                if interception.is_none()
                    && controlled_turn
                    && legacy_hero_mutation(
                        &tool_call.name,
                        &tool_call.args,
                        &context.public_intention_context,
                    )
                {
                    interception = Some(super::tool_dispatch::ToolExecution {
                        result: json!({"error":"canonical_player_intention_required"}),
                        is_error: true,
                        handled_by: "engine".into(),
                        media: vec![],
                    });
                }
                if interception.is_none()
                    && controlled_turn
                    && legacy_world_mechanics(&tool_call.name)
                    && decode_tool_call(&tool_call.name, &tool_call.args).is_ok()
                {
                    let decision = self
                        .assess_player(
                            &request.player_message,
                            Some((tool_call.name.clone(), tool_call.args.clone())),
                            &resolved_intentions,
                            &messages,
                            &context.public_intention_context,
                        )
                        .await?;
                    if !decision.allowed {
                        interception = Some(super::tool_dispatch::ToolExecution {
                            result: json!({"error":"canonical_player_intention_required"}),
                            is_error: true,
                            handled_by: "engine".into(),
                            media: vec![],
                        });
                    }
                }
                if interception.is_none()
                    && controlled_turn
                    && player_resolution_tool(&tool_call.name)
                    && (player_intention_resolved
                        || tools.iter().any(|tool| tool.name == tool_call.name))
                {
                    if let Ok(decoded) = decode_tool_call(&tool_call.name, &tool_call.args) {
                        let recorded = self
                            .tools
                            .replay(ToolDispatchRequest {
                                request_id: command.request_id.clone(),
                                tool_call_id: tool_call.id.clone(),
                                campaign_id: request.campaign_id,
                                session_id: request.session_id,
                                command: decoded,
                            })
                            .await
                            .map_err(|error| AgentTurnError::ToolDispatch(error.code))?;
                        if let Some(recorded) = recorded {
                            if let Some(rejected) = super::intentions::recognized_resolution(
                                recorded.is_error,
                                &recorded.result,
                            ) {
                                if !resolved_intentions
                                    .iter()
                                    .any(|resolved| resolved.tool_call_id == tool_call.id)
                                {
                                    resolved_intentions.push(
                                        super::intentions::ResolvedPlayerIntention {
                                            tool_call_id: tool_call.id.clone(),
                                            result: recorded.result.clone(),
                                            tool_name: tool_call.name.clone(),
                                            args: tool_call.args.clone(),
                                            rejected,
                                        },
                                    );
                                }
                                let decision = self
                                    .assess_player(
                                        &request.player_message,
                                        None,
                                        &resolved_intentions,
                                        &messages,
                                        &context.public_intention_context,
                                    )
                                    .await?;
                                player_intention_resolved = !decision.further_requested;
                            } else {
                                return Err(AgentTurnError::ToolDispatch(
                                    "player_resolution_uncertain",
                                ));
                            }
                            interception = Some(recorded);
                        } else if player_intention_resolved {
                            interception = Some(super::tool_dispatch::ToolExecution {
                                result: json!({"error":"player_intention_already_resolved"}),
                                is_error: true,
                                handled_by: "engine".into(),
                                media: vec![],
                            });
                        } else {
                            let decision = self
                                .assess_player(
                                    &request.player_message,
                                    Some((tool_call.name.clone(), tool_call.args.clone())),
                                    &resolved_intentions,
                                    &messages,
                                    &context.public_intention_context,
                                )
                                .await?;
                            if decision.allowed {
                                permission = Some(decision);
                            } else {
                                // Denying a proposal is not a recorded outcome of
                                // the human's request. Recheck completion separately
                                // against actual outcomes before publishing prose.
                                interception = Some(super::tool_dispatch::ToolExecution {
                                    result: json!({"error":"player_intention_not_requested"}),
                                    is_error: true,
                                    handled_by: "engine".into(),
                                    media: vec![],
                                });
                            }
                        }
                    }
                }
                let execution = if let Some(execution) = interception {
                    execution
                } else if context.adventure_active
                    && !tools.iter().any(|tool| tool.name == tool_call.name)
                {
                    super::tool_dispatch::ToolExecution {
                        result: json!({"error":"tool_not_available_in_authored_adventure"}),
                        is_error: true,
                        handled_by: "engine".into(),
                        media: vec![],
                    }
                } else {
                    match decode_tool_call(&tool_call.name, &tool_call.args) {
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
                            let execution =
                                result.map_err(|error| AgentTurnError::ToolDispatch(error.code))?;
                            if let Some(decision) = permission {
                                let rejected = super::intentions::recognized_resolution(
                                    execution.is_error,
                                    &execution.result,
                                )
                                .ok_or(
                                    AgentTurnError::ToolDispatch("player_resolution_uncertain"),
                                )?;
                                resolved_intentions.push(
                                    super::intentions::ResolvedPlayerIntention {
                                        tool_call_id: tool_call.id.clone(),
                                        result: execution.result.clone(),
                                        tool_name: tool_call.name.clone(),
                                        args: tool_call.args.clone(),
                                        rejected,
                                    },
                                );
                                player_intention_resolved = !decision.further_requested;
                            }
                            execution
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
                    }
                };

                for media in execution.media {
                    let event = match media {
                        GeneratedAgentMedia::Image {
                            mime_type,
                            data_b64,
                            kind,
                            source,
                        } => AgentEvent::ImageGenerated {
                            tool_call_id: tool_call.id.clone(),
                            round: total_rounds,
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
                if controlled_turn {
                    context = self
                        .context
                        .build(AgentContextRequest {
                            campaign_id: request.campaign_id,
                            player_message: request.player_message.clone(),
                            base_system_prompt: self.config.system_prompt.clone(),
                            embedding_model: self.config.embedding_model.clone(),
                        })
                        .await
                        .map_err(|error| AgentTurnError::Context(error.code))?;
                    system_prompt =
                        append_board(context.system_prompt.clone(), request.board.as_deref());
                    tools = self
                        .config
                        .tools
                        .for_context(context.combat_active, context.adventure_active);
                    restrict_to_narration(
                        &mut tools,
                        &mut system_prompt,
                        player_intention_resolved,
                    );
                    append_rejection_brief(&mut system_prompt, &resolved_intentions);
                }
            }

            if matches!(finish_reason, FinishReason::Stop) {
                final_reply_confirmed = true;
                break;
            }
            {
                context = self
                    .context
                    .build(AgentContextRequest {
                        campaign_id: request.campaign_id,
                        player_message: request.player_message.clone(),
                        base_system_prompt: self.config.system_prompt.clone(),
                        embedding_model: self.config.embedding_model.clone(),
                    })
                    .await
                    .map_err(|error| AgentTurnError::Context(error.code))?;
                system_prompt =
                    append_board(context.system_prompt.clone(), request.board.as_deref());
                if continuing {
                    system_prompt.push_str("\nThis is an interrupted turn continuation. Recorded tool results are already confirmed. Finish the original intention from those results; do not repeat confirmed effects.");
                }
                tools = self
                    .config
                    .tools
                    .for_context(context.combat_active, context.adventure_active);
                restrict_to_narration(&mut tools, &mut system_prompt, player_intention_resolved);
                append_rejection_brief(&mut system_prompt, &resolved_intentions);
            }
        }

        if !final_reply_confirmed {
            // Confirmed tool results remain recoverable; exhaustion is not a DM answer.
            tracing::warn!(error_code = "agent_round_limit", "LLM turn incomplete");
            return Err(AgentTurnError::Provider);
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

    async fn assess_player(
        &self,
        player_message: &str,
        candidate: Option<(String, Value)>,
        resolved: &[super::intentions::ResolvedPlayerIntention],
        conversation: &[ChatMessage],
        public_context: &Value,
    ) -> Result<super::intentions::PlayerIntentionDecision, AgentTurnError> {
        let verifier = self
            .config
            .player_verifier
            .as_ref()
            .ok_or(AgentTurnError::ToolDispatch("player_verifier_missing"))?;
        let check = super::intentions::PlayerIntentionCheck {
            player_message: player_message.into(),
            candidate,
            resolved: resolved.to_vec(),
            conversation: conversation.to_vec(),
            public_context: public_context.clone(),
        };
        let decision = tokio::select! {biased;_ = self.cancellation.cancelled()=>return Err(AgentTurnError::Cancelled),result=verifier.assess(check)=>result.map_err(|_|AgentTurnError::ToolDispatch("player_verification_unavailable"))?};
        tracing::debug!(
            allowed = decision.allowed,
            further_requested = decision.further_requested,
            "player intention checked"
        );
        Ok(decision)
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

    async fn push_delta(
        &self,
        session_id: uuid::Uuid,
        sequence: &mut u64,
        pending: &mut Option<AgentEvent>,
        incoming: AgentEvent,
        tx: &mpsc::Sender<Result<AgentEvent, AgentTurnError>>,
    ) -> Result<(), AgentTurnError> {
        const FLUSH_CHARS: usize = 256;
        let merged = match (pending.as_mut(), incoming) {
            (Some(AgentEvent::TextDelta { text }), AgentEvent::TextDelta { text: next }) => {
                text.push_str(&next);
                true
            }
            (
                Some(AgentEvent::ReasoningText { text }),
                AgentEvent::ReasoningText { text: next },
            ) => {
                text.push_str(&next);
                true
            }
            (_, event) => {
                self.flush_delta(session_id, sequence, pending, tx).await?;
                *pending = Some(event);
                false
            }
        };
        let should_flush = match pending.as_ref() {
            Some(AgentEvent::TextDelta { text } | AgentEvent::ReasoningText { text }) => {
                text.chars().count() >= FLUSH_CHARS
            }
            _ => false,
        };
        if merged && should_flush {
            self.flush_delta(session_id, sequence, pending, tx).await?;
        }
        Ok(())
    }

    async fn flush_delta(
        &self,
        session_id: uuid::Uuid,
        sequence: &mut u64,
        pending: &mut Option<AgentEvent>,
        tx: &mpsc::Sender<Result<AgentEvent, AgentTurnError>>,
    ) -> Result<(), AgentTurnError> {
        if let Some(event) = pending.take() {
            self.emit(session_id, sequence, event, tx).await?;
        }
        Ok(())
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
        AgentTurnError::ToolDispatch(code) => code,
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

/// Diagnostic categories must never contain provider bodies, credentials or thought text.
fn safe_provider_error_code(error: &crate::ports::llm::LlmError) -> &'static str {
    use crate::ports::llm::LlmError;
    match error {
        LlmError::RateLimit => "rate_limit",
        LlmError::AuthFailure => "authentication",
        LlmError::Network(_) => "network",
        LlmError::Provider(_) => "provider",
        LlmError::InvalidResponse(code) => match code.as_str() {
            "provider_output_truncated" => "provider_output_truncated",
            "provider_output_filtered" => "provider_output_filtered",
            "provider_public_response_missing" => "provider_public_response_missing",
            "provider_finish_reason_unknown" => "provider_finish_reason_unknown",
            "provider_tool_calls_missing" => "provider_tool_calls_missing",
            "provider_tool_identity_invalid" => "provider_tool_identity_invalid",
            _ => "invalid_response",
        },
    }
}

fn player_intention_tool(name: &str) -> bool {
    matches!(
        name,
        "adventure_action" | "combat_action" | "rest_action" | "end_combat"
    )
}
fn player_resolution_tool(name: &str) -> bool {
    player_intention_tool(name) || name == "roll_dice"
}
fn resolved_player_intentions(
    messages: &[ChatMessage],
) -> Result<Vec<super::intentions::ResolvedPlayerIntention>, AgentTurnError> {
    let calls = messages
        .iter()
        .filter_map(|message| match message {
            ChatMessage::AssistantWithToolCalls { tool_calls, .. } => Some(tool_calls),
            _ => None,
        })
        .flatten()
        .filter(|call| {
            player_resolution_tool(&call.name) && decode_tool_call(&call.name, &call.args).is_ok()
        })
        .collect::<Vec<_>>();
    let mut resolved = Vec::new();
    let mut seen = HashSet::new();
    for call in calls {
        if !seen.insert(call.id.clone()) {
            continue;
        }
        if let Some(result) = messages.iter().find_map(|message| match message {
            ChatMessage::ToolResult(result) if result.tool_call_id == call.id => Some(result),
            _ => None,
        }) {
            let value: Value = serde_json::from_str(&result.content)
                .map_err(|_| AgentTurnError::ToolDispatch("player_resolution_uncertain"))?;
            if let Some(rejected) =
                super::intentions::recognized_resolution(result.is_error, &value)
            {
                resolved.push(super::intentions::ResolvedPlayerIntention {
                    tool_call_id: call.id.clone(),
                    result: value.clone(),
                    tool_name: call.name.clone(),
                    args: call.args.clone(),
                    rejected,
                });
            } else if !matches!(
                value["error"].as_str(),
                Some(
                    "tool_not_available_in_authored_adventure"
                        | "player_intention_already_resolved"
                        | "player_intention_not_requested"
                        | "canonical_player_intention_required"
                )
            ) {
                return Err(AgentTurnError::ToolDispatch("player_resolution_uncertain"));
            }
        }
    }
    Ok(resolved)
}
fn restrict_to_narration(
    tools: &mut Vec<crate::models::chat::Tool>,
    prompt: &mut String,
    resolved: bool,
) {
    if !resolved {
        return;
    }
    tools.retain(|tool| !player_resolution_tool(&tool.name));
    prompt.push_str("\nThe player's engine intention in THIS request is already resolved, either confirmed or definitively rejected. Narrate its actual result and stop. Do not choose a follow-up action, move, attack, rest, end-turn, or new dice roll for the hero. A rejected action does not authorize a replacement action. Future choices belong to the next human request. Recorded results are historical; do not invent success or overwrite current facts.");
}

fn legacy_hero_mutation(name: &str, args: &Value, context: &Value) -> bool {
    let id = match name {
        "update_token" | "remove_token" => args["id"].as_str(),
        "cast_spell" => args["caster_id"].as_str(),
        _ => None,
    };
    id.is_some_and(|id| {
        context["targets"].as_array().is_some_and(|targets| {
            targets.iter().any(|actor| {
                !actor["character_id"].is_null()
                    && (actor["id"] == id || actor["character_id"] == id)
            })
        })
    })
}

fn legacy_world_mechanics(name: &str) -> bool {
    matches!(
        name,
        "apply_damage" | "apply_healing" | "cast_spell" | "update_token" | "remove_token"
    )
}

fn append_rejection_brief(
    prompt: &mut String,
    resolved: &[super::intentions::ResolvedPlayerIntention],
) {
    let Some(last) = resolved.last().filter(|result| result.rejected) else {
        return;
    };
    if last.result["error"] == "combat_obstructed" {
        prompt.push_str("\nCONFIRMED ENGINE REFUSAL: a physical obstacle blocks the requested attack. The attack did not occur. Do not describe a hit/miss roll or say there are no barriers; do not speculate about another cause. This refusal does not authorize moving, changing targets or ending the hero's turn. / Подтверждённый отказ движка: препятствие перекрывает линию атаки. Атака не выполнена; это не попадание и не промах. Объясни именно препятствие, не утверждай, что физических преград нет, и не придумывай другую причину. Следующее действие выбирает игрок.");
    }
}
