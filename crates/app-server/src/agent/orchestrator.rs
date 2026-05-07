//! Agent loop orchestrator.
//!
//! Runs up to `max_rounds` of (LLM stream -> tool-calls -> engine -> inject results).
//! Emits `AgentEvent` values to the caller's channel; the SSE handler converts these
//! to SSE events. Using a channel instead of returning a stream keeps the orchestrator
//! logic synchronous and easier to test.

use std::collections::HashMap;
use std::sync::Arc;

use app_domain::srd::retriever::SrdRetriever;
use app_llm::{ChatChunk, ChatMessage, ChatRequest, FinishReason, LlmProvider, ToolCall};
use futures::StreamExt;
use serde_json::Value;
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::agent::context_builder::build_context;
use crate::agent::tool_executor::execute_tool;
use crate::agent::tools::all_tools;

/// Configuration for the agent that does not change between turns.
#[derive(Clone)]
pub struct AgentConfig {
    pub model: String,
    pub system_prompt: String,
    pub temperature: f32,
    pub max_rounds: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model: "claude-haiku-4-5-20251001".into(),
            system_prompt: String::new(),
            temperature: 0.7,
            max_rounds: 8,
        }
    }
}

/// Request for a single player turn through the agent.
pub struct AgentTurnRequest {
    pub campaign_id: Uuid,
    pub session_id: Uuid,
    pub player_message: String,
    pub history: Vec<ChatMessage>,
}

/// Events emitted by the orchestrator, consumed by the SSE handler.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// A streaming text token from the LLM.
    TextDelta { text: String },
    /// The LLM started a tool-call block.
    ToolCallStart {
        id: String,
        tool_name: String,
        round: usize,
    },
    /// The engine executed the tool-call and this is the result.
    ToolCallResult {
        id: String,
        tool_name: String,
        args: Value,
        result: Value,
        is_error: bool,
        round: usize,
    },
    /// The agent loop completed.
    AgentDone { total_rounds: usize },
    /// A non-fatal error in one tool-call (loop continues).
    ToolCallError {
        id: String,
        tool_name: String,
        message: String,
    },
}

pub struct AgentOrchestrator {
    provider: Arc<dyn LlmProvider>,
    pool: SqlitePool,
    config: AgentConfig,
    retriever: Option<Arc<SrdRetriever>>,
}

impl AgentOrchestrator {
    pub fn new(
        provider: Arc<dyn LlmProvider>,
        pool: SqlitePool,
        config: AgentConfig,
        retriever: Option<Arc<SrdRetriever>>,
    ) -> Self {
        Self {
            provider,
            pool,
            config,
            retriever,
        }
    }

    pub async fn run(
        &self,
        req: AgentTurnRequest,
        tx: mpsc::Sender<AgentEvent>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Build initial system context.
        let system_context = build_context(
            &self.pool,
            req.campaign_id,
            &req.player_message,
            &self.config.system_prompt,
            self.retriever.as_deref(),
        )
        .await
        .unwrap_or_else(|e| {
            warn!("context build error: {e}");
            self.config.system_prompt.clone()
        });

        let tools = all_tools();
        let mut messages: Vec<ChatMessage> = req.history;
        messages.push(ChatMessage::User {
            content: req.player_message.clone(),
        });

        let mut total_rounds = 0usize;

        for round in 0..self.config.max_rounds {
            total_rounds = round + 1;

            let chat_req = ChatRequest {
                messages: messages.clone(),
                model: self.config.model.clone(),
                max_tokens: Some(2048),
                temperature: Some(self.config.temperature),
                tools: tools.clone(),
                system_prompt: Some(system_context.clone()),
            };

            let mut stream = match self.provider.stream_chat(chat_req).await {
                Ok(s) => s,
                Err(e) => {
                    warn!("LLM stream error round {round}: {e}");
                    break;
                }
            };

            // Accumulate text + tool-call arg buffers for this round.
            let mut round_text = String::new();
            // Map of tool-call id -> (tool name, accumulated JSON arg fragment).
            let mut tool_args_buf: HashMap<String, (String, String)> = HashMap::new();
            let mut tool_calls_this_round: Vec<ToolCall> = Vec::new();
            let mut finish_reason = FinishReason::Stop;

            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(ChatChunk::TextDelta { text }) => {
                        round_text.push_str(&text);
                        if tx.send(AgentEvent::TextDelta { text }).await.is_err() {
                            return Ok(());
                        }
                    }
                    Ok(ChatChunk::ToolCallStart { id, name }) => {
                        tool_args_buf.insert(id.clone(), (name.clone(), String::new()));
                        if tx
                            .send(AgentEvent::ToolCallStart {
                                id: id.clone(),
                                tool_name: name,
                                round: total_rounds,
                            })
                            .await
                            .is_err()
                        {
                            return Ok(());
                        }
                    }
                    Ok(ChatChunk::ToolCallArgsDelta { id, args_fragment }) => {
                        if let Some((_, args)) = tool_args_buf.get_mut(&id) {
                            args.push_str(&args_fragment);
                        }
                    }
                    Ok(ChatChunk::ToolCallDone { id }) => {
                        if let Some((name, args_str)) = tool_args_buf.remove(&id) {
                            let args: Value = serde_json::from_str(&args_str)
                                .unwrap_or(Value::Object(Default::default()));
                            tool_calls_this_round.push(ToolCall { id, name, args });
                        }
                    }
                    Ok(ChatChunk::Done { reason }) => {
                        finish_reason = reason;
                        break;
                    }
                    Err(e) => {
                        warn!("stream chunk error: {e}");
                        break;
                    }
                }
            }

            // Inject the assistant turn (text + any tool-calls) into history.
            if tool_calls_this_round.is_empty() {
                if !round_text.is_empty() {
                    messages.push(ChatMessage::Assistant {
                        content: round_text,
                    });
                }
            } else {
                messages.push(ChatMessage::AssistantWithToolCalls {
                    content: if round_text.is_empty() {
                        None
                    } else {
                        Some(round_text)
                    },
                    tool_calls: tool_calls_this_round.clone(),
                });
            }

            // Execute all tool-calls from this round.
            for tc in &tool_calls_this_round {
                let (result_val, is_error) = execute_tool(tc, &self.pool, req.campaign_id).await;
                info!("tool {} -> {:?}", tc.name, result_val);

                let result_str = serde_json::to_string(&result_val).unwrap_or_default();
                let _ = tx
                    .send(AgentEvent::ToolCallResult {
                        id: tc.id.clone(),
                        tool_name: tc.name.clone(),
                        args: tc.args.clone(),
                        result: result_val.clone(),
                        is_error,
                        round: total_rounds,
                    })
                    .await;

                messages.push(ChatMessage::ToolResult(app_llm::ToolResult {
                    tool_call_id: tc.id.clone(),
                    content: result_str,
                    is_error,
                }));
            }

            // If finish reason is Stop/Length/Error (no tool-calls expected), we are done.
            match finish_reason {
                FinishReason::Stop | FinishReason::Length | FinishReason::Error => break,
                FinishReason::ToolUse => {
                    // Continue to next round with updated history.
                }
            }

            if tool_calls_this_round.is_empty() {
                // LLM declared ToolUse but emitted no calls - safety exit.
                break;
            }
        }

        let _ = tx.send(AgentEvent::AgentDone { total_rounds }).await;
        Ok(())
    }
}
