//! Compatibility facade for the inward-owned agent turn application service.

use std::sync::Arc;

use app_application::agent::context::{
    AgentContext, AgentContextError, AgentContextPort, AgentContextRequest,
};
use app_application::agent::tool_dispatch::{
    CapabilityToolDispatcher, GeneratedAgentMedia, RuntimeCoordinationError, RuntimeCoordinator,
    ToolCapabilityHandler, ToolDispatchError, ToolDispatchRequest, ToolExecution,
};
use app_application::agent::tools::{
    classify_handler, image_kind, video_kind, AgentToolCatalog, ToolAvailability,
};
use app_application::agent::turn::{
    AgentTurnCommand, AgentTurnConfig, AgentTurnService, NeverCancelled,
};
use app_application::ports::events::NoopApplicationEventSink;
use app_domain::srd::retriever::SrdRetriever;
use app_llm::{LlmProvider, ReasoningSpec, ToolCall};
use async_trait::async_trait;
use futures::StreamExt;
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::agent::context_builder::{build_context, compose_system_prompt, needs_rules_context};
use crate::agent::tool_executor::{execute_tool, is_combat_active};
use crate::agent::tools::tools_for_phase;
use crate::image::provider::ImageProvider;
use crate::video::provider::VideoProvider;

pub use app_application::models::agent::{AgentEvent, AgentTurnRequest};

#[derive(Clone)]
pub struct AgentConfig {
    pub model: String,
    pub system_prompt: String,
    pub temperature: f32,
    pub max_rounds: usize,
    pub embedding_model: String,
    pub tool_availability: ToolAvailability,
    pub reasoning_enabled: bool,
    pub reasoning_budget: ReasoningSpec,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model: "claude-haiku-4-5-20251001".into(),
            system_prompt: String::new(),
            temperature: 0.7,
            max_rounds: 8,
            embedding_model: adapter_llm::embeddings::DEFAULT_EMBEDDING_MODEL.into(),
            tool_availability: ToolAvailability::all(),
            reasoning_enabled: false,
            reasoning_budget: ReasoningSpec::Medium,
        }
    }
}

pub struct AgentOrchestrator {
    provider: Arc<dyn LlmProvider>,
    pool: SqlitePool,
    config: AgentConfig,
    retriever: Option<Arc<SrdRetriever>>,
    image_provider: Option<Arc<dyn ImageProvider>>,
    video_provider: Option<Arc<dyn VideoProvider>>,
    gpu_swap: Option<Arc<crate::local_runtime::registry::ImageGpuSwap>>,
}

impl AgentOrchestrator {
    pub fn new(
        provider: Arc<dyn LlmProvider>,
        pool: SqlitePool,
        config: AgentConfig,
        retriever: Option<Arc<SrdRetriever>>,
        image_provider: Option<Arc<dyn ImageProvider>>,
    ) -> Self {
        Self {
            provider,
            pool,
            config,
            retriever,
            image_provider,
            video_provider: None,
            gpu_swap: None,
        }
    }

    pub fn with_video_provider(mut self, video_provider: Option<Arc<dyn VideoProvider>>) -> Self {
        self.video_provider = video_provider;
        self
    }

    pub fn with_gpu_swap(
        mut self,
        gpu_swap: Option<Arc<crate::local_runtime::registry::ImageGpuSwap>>,
    ) -> Self {
        self.gpu_swap = gpu_swap;
        self
    }

    pub async fn run(
        &self,
        request: AgentTurnRequest,
        tx: mpsc::Sender<AgentEvent>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let base_system_prompt =
            compose_system_prompt(&self.config.system_prompt, self.config.tool_availability);
        let tool_catalog = AgentToolCatalog {
            exploration: tools_for_phase(self.config.tool_availability, false),
            combat: tools_for_phase(self.config.tool_availability, true),
        };
        let legacy_tools = Arc::new(LegacyToolAdapter {
            store: adapter_sqlite::SqliteStore::new(self.pool.clone()),
            image_provider: self.image_provider.clone(),
            video_provider: self.video_provider.clone(),
            retriever: self.retriever.clone(),
            embedding_model: self.config.embedding_model.clone(),
        });
        let tool_dispatcher = Arc::new(CapabilityToolDispatcher::new(
            legacy_tools.clone(),
            legacy_tools.clone(),
            legacy_tools.clone(),
            legacy_tools.clone(),
            legacy_tools,
        ));
        let service = Arc::new(AgentTurnService::new(
            self.provider.clone(),
            Arc::new(LegacyContextAdapter {
                store: adapter_sqlite::SqliteStore::new(self.pool.clone()),
                retriever: self.retriever.clone(),
            }),
            Arc::new(adapter_sqlite::SqliteStore::new(self.pool.clone())),
            tool_dispatcher,
            Arc::new(LegacyRuntimeAdapter {
                gpu_swap: self.gpu_swap.clone(),
            }),
            Arc::new(NoopApplicationEventSink),
            Arc::new(NeverCancelled),
            AgentTurnConfig {
                model: self.config.model.clone(),
                system_prompt: base_system_prompt,
                temperature: self.config.temperature,
                max_rounds: self.config.max_rounds,
                embedding_model: self.config.embedding_model.clone(),
                reasoning_enabled: self.config.reasoning_enabled,
                reasoning_budget: self.config.reasoning_budget,
                tool_availability: self.config.tool_availability,
                tools: tool_catalog,
            },
        ));
        let command = AgentTurnCommand {
            request_id: Uuid::new_v4().to_string(),
            request,
        };
        let mut events = service.execute(command);
        while let Some(event) = events.next().await {
            tx.send(event?)
                .await
                .map_err(|_| "agent event receiver dropped")?;
        }
        Ok(())
    }
}

struct LegacyContextAdapter {
    store: adapter_sqlite::SqliteStore,
    retriever: Option<Arc<SrdRetriever>>,
}

#[async_trait]
impl AgentContextPort for LegacyContextAdapter {
    async fn build(&self, request: AgentContextRequest) -> Result<AgentContext, AgentContextError> {
        let combat_active = is_combat_active(&self.store).await;
        let inject_rules = needs_rules_context(&request.player_message, combat_active);
        let system_prompt = build_context(
            &self.store,
            request.campaign_id,
            &request.player_message,
            &request.base_system_prompt,
            &request.embedding_model,
            self.retriever.as_deref(),
            inject_rules,
        )
        .await
        .unwrap_or(request.base_system_prompt);
        Ok(AgentContext {
            system_prompt,
            combat_active,
        })
    }
}

struct LegacyToolAdapter {
    store: adapter_sqlite::SqliteStore,
    image_provider: Option<Arc<dyn ImageProvider>>,
    video_provider: Option<Arc<dyn VideoProvider>>,
    retriever: Option<Arc<SrdRetriever>>,
    embedding_model: String,
}

#[async_trait]
impl ToolCapabilityHandler for LegacyToolAdapter {
    async fn execute_capability(
        &self,
        request: ToolDispatchRequest,
    ) -> Result<ToolExecution, ToolDispatchError> {
        let tool_name = request.command.tool_name().to_string();
        let call = ToolCall {
            id: request.tool_call_id,
            name: tool_name.clone(),
            args: request.command.to_args(),
        };
        let (mut result, is_error) = execute_tool(
            &call,
            &self.store,
            self.image_provider.clone(),
            self.video_provider.clone(),
            self.retriever.as_deref(),
            &self.embedding_model,
            request.campaign_id,
            request.session_id,
        )
        .await;
        let mut media = Vec::new();
        if !is_error {
            if let Some(map) = result.as_object_mut() {
                if let Some(serde_json::Value::String(data_b64)) = map.remove("image_b64") {
                    media.push(GeneratedAgentMedia::Image {
                        mime_type: map
                            .get("mime_type")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("image/png")
                            .to_string(),
                        data_b64,
                        kind: image_kind(&tool_name).unwrap_or("chat").to_string(),
                    });
                }
                if let Some(serde_json::Value::String(data_b64)) = map.remove("video_b64") {
                    media.push(GeneratedAgentMedia::Video {
                        mime_type: map
                            .get("mime_type")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("video/mp4")
                            .to_string(),
                        data_b64,
                        kind: video_kind(&tool_name).unwrap_or("chat").to_string(),
                    });
                }
            }
        }
        Ok(ToolExecution {
            result,
            is_error,
            handled_by: classify_handler(&tool_name).to_string(),
            media,
        })
    }
}

struct LegacyRuntimeAdapter {
    gpu_swap: Option<Arc<crate::local_runtime::registry::ImageGpuSwap>>,
}

#[async_trait]
impl RuntimeCoordinator for LegacyRuntimeAdapter {
    async fn acquire_for(&self, _tool_name: &str) -> Result<(), RuntimeCoordinationError> {
        if let Some(gpu_swap) = &self.gpu_swap {
            gpu_swap.acquire().await;
        }
        Ok(())
    }

    async fn release_for(&self, _tool_name: &str) -> Result<(), RuntimeCoordinationError> {
        if let Some(gpu_swap) = &self.gpu_swap {
            gpu_swap.release().await;
        }
        Ok(())
    }
}
