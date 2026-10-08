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
use app_application::ports::media::ImageSource;
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
    player_verifier: Option<Arc<dyn app_application::agent::intentions::PlayerIntentionVerifier>>,
}

impl AgentOrchestrator {
    pub fn with_player_verifier(
        mut self,
        verifier: Arc<dyn app_application::agent::intentions::PlayerIntentionVerifier>,
    ) -> Self {
        self.player_verifier = Some(verifier);
        self
    }
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
            player_verifier: None,
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
        self.run_with_id(
            request,
            tx,
            Uuid::new_v4().to_string(),
            Arc::new(NeverCancelled),
        )
        .await
    }

    pub async fn run_with_id(
        &self,
        request: AgentTurnRequest,
        tx: mpsc::Sender<AgentEvent>,
        request_id: String,
        cancellation: Arc<dyn app_application::agent::turn::CancellationSignal>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.run_recovered(request, tx, request_id, cancellation, None)
            .await
    }
    pub async fn run_recovered(
        &self,
        request: AgentTurnRequest,
        tx: mpsc::Sender<AgentEvent>,
        request_id: String,
        cancellation: Arc<dyn app_application::agent::turn::CancellationSignal>,
        checkpoint: Option<Vec<app_application::models::chat::ChatMessage>>,
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
            Arc::new(super::checkpoint_tools::CheckpointTools {
                inner: legacy_tools.clone(),
                store: adapter_sqlite::SqliteStore::new(self.pool.clone()),
            }),
            Arc::new(super::checkpoint_tools::CheckpointTools {
                inner: legacy_tools.clone(),
                store: adapter_sqlite::SqliteStore::new(self.pool.clone()),
            }),
            Arc::new(super::checkpoint_tools::CheckpointTools {
                inner: legacy_tools.clone(),
                store: adapter_sqlite::SqliteStore::new(self.pool.clone()),
            }),
            Arc::new(super::checkpoint_tools::CheckpointTools {
                inner: legacy_tools.clone(),
                store: adapter_sqlite::SqliteStore::new(self.pool.clone()),
            }),
            Arc::new(super::checkpoint_tools::CheckpointTools {
                inner: legacy_tools,
                store: adapter_sqlite::SqliteStore::new(self.pool.clone()),
            }),
        ));
        let service = Arc::new(AgentTurnService::new(
            self.provider.clone(),
            Arc::new(LegacyContextAdapter {
                store: adapter_sqlite::SqliteStore::new(self.pool.clone()),
                retriever: self.retriever.clone(),
            }),
            Arc::new(adapter_sqlite::messages::OperationMessages {
                store: adapter_sqlite::SqliteStore::new(self.pool.clone()),
                request_id: Uuid::parse_str(&request_id).unwrap_or_else(|_| Uuid::nil()),
            }),
            tool_dispatcher,
            Arc::new(LegacyRuntimeAdapter {
                gpu_swap: self.gpu_swap.clone(),
            }),
            Arc::new(NoopApplicationEventSink),
            cancellation,
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
                player_verifier: self.player_verifier.clone(),
            },
        ));
        let command = AgentTurnCommand {
            request_id,
            request,
        };
        let mut events = service.execute_recovered(command, checkpoint);
        let mut failure = None;
        while let Some(event) = events.next().await {
            match event {
                Ok(event) => {
                    if tx.send(event).await.is_err() {
                        failure = Some("agent event receiver dropped".to_string());
                    }
                }
                Err(error) => failure = Some(error.to_string()),
            }
        }
        match failure {
            Some(error) => Err(error.into()),
            None => Ok(()),
        }
    }
}

struct LegacyContextAdapter {
    store: adapter_sqlite::SqliteStore,
    retriever: Option<Arc<SrdRetriever>>,
}

#[async_trait]
impl AgentContextPort for LegacyContextAdapter {
    async fn build(&self, request: AgentContextRequest) -> Result<AgentContext, AgentContextError> {
        use app_application::ports::repositories::CampaignStateRepository;
        let current = self
            .store
            .load_current(app_application::models::identity::CampaignId(
                request.campaign_id,
            ))
            .await;
        let combat_active = match &current {
            Ok(state) => state
                .projection
                .combat
                .as_ref()
                .is_some_and(|combat| combat["snapshot"]["active"].as_bool() == Some(true)),
            Err(app_application::ports::repositories::RepositoryError::NotFound) => {
                is_combat_active(&self.store).await
            }
            Err(_) => {
                return Err(AgentContextError {
                    code: "campaign_context_failed",
                })
            }
        };
        let inject_rules = needs_rules_context(&request.player_message, combat_active);
        let mut system_prompt = build_context(
            &self.store,
            request.campaign_id,
            &request.player_message,
            &request.base_system_prompt,
            &request.embedding_model,
            self.retriever.as_deref(),
            inject_rules,
        )
        .await
        .map_err(|_| AgentContextError {
            code: "campaign_context_failed",
        })?;
        let public_intention_context = current
            .as_ref()
            .ok()
            .map(|state| {
                app_application::agent::context::public_intention_context(
                    state.projection.adventure.as_ref(),
                    state.projection.combat.as_ref(),
                    state.projection.character["name"].as_str(),
                )
            })
            .unwrap_or_default();
        let adventure_active = match current {
            Ok(state) => {
                if state.character_id.is_some() {
                    let mut status =
                        app_application::rest::status_for_state(&state).map_err(|_| {
                            AgentContextError {
                                code: "hero_context_failed",
                            }
                        })?;
                    status.combat_active =
                        state.projection.combat.as_ref().is_some_and(|combat| {
                            combat["snapshot"]["active"].as_bool() == Some(true)
                        });
                    system_prompt.push_str(&format!("\nCANONICAL HERO MECHANICS: {}\nUse combat_action for the bound hero's intentions and rest_action for rest/time. Choose only supported spell choices and available weapons/targets shown here. After a tool result, that result supersedes the previous state. Raw damage/healing/token mutations cannot replace canonical intentions. All rolls and resource changes belong to the engine. For player skill checks call roll_dice with dice 1d20 and check {{character_id, skill, dc}}; omit modifier. Armor/poisoned disadvantage is enforced automatically. Narrate the confirmed total/success only.",serde_json::json!({"skill_check_actor":state.character_id.map(|id|app_application::hero_checks::context_identity(id,&state.projection.character)),"combat":state.projection.combat,"rest_status":status})));
                }
                state.projection.adventure.is_some()
            }
            Err(app_application::ports::repositories::RepositoryError::NotFound) => false,
            Err(_) => {
                return Err(AgentContextError {
                    code: "campaign_context_failed",
                })
            }
        };
        Ok(AgentContext {
            reasoning_is_private: adventure_active,
            adventure_active,
            system_prompt,
            combat_active,
            public_intention_context,
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
    async fn replay_capability(
        &self,
        request: ToolDispatchRequest,
    ) -> Result<Option<ToolExecution>, ToolDispatchError> {
        self.store
            .recover_player_tool(&request)
            .await
            .map_err(|_| ToolDispatchError {
                code: "tool_replay_failed",
            })
    }
    async fn execute_capability(
        &self,
        request: ToolDispatchRequest,
    ) -> Result<ToolExecution, ToolDispatchError> {
        let tool_name = request.command.tool_name().to_string();
        {
            use app_application::agent::commands::AgentToolCommand;
            use app_application::agent::hero_intentions::{HeroIntention, HeroIntentionRepository};
            let campaign_id = app_application::models::identity::CampaignId(request.campaign_id);
            let session_id = app_application::models::identity::SessionId(request.session_id);
            let causation_id = format!("agent:{}:{}", request.request_id, request.tool_call_id);
            let seed = Uuid::new_v4().as_u128() as u64;
            let outcome = match &request.command {
                AgentToolCommand::CombatAction(action) => Some(
                    self.store
                        .combat_as_dm(
                            HeroIntention {
                                campaign_id,
                                session_id,
                                causation_id,
                                action: action.clone(),
                            },
                            seed,
                        )
                        .await,
                ),
                AgentToolCommand::RestAction(action) => Some(
                    self.store
                        .rest_as_dm(
                            HeroIntention {
                                campaign_id,
                                session_id,
                                causation_id,
                                action: *action,
                            },
                            seed,
                        )
                        .await,
                ),
                _ => None,
            };
            if let Some(outcome) = outcome {
                if outcome
                    .as_ref()
                    .err()
                    .is_some_and(|error| definitive_player_rejection(error).is_none())
                {
                    return Err(ToolDispatchError {
                        code: "hero_intention_uncertain",
                    });
                }
                return Ok(match outcome {
                    Ok(state) => {
                        let mut status =
                            app_application::rest::status_for_state(&state).map_err(|_| {
                                ToolDispatchError {
                                    code: "hero_status_failed",
                                }
                            })?;
                        status.combat_active =
                            state.projection.combat.as_ref().is_some_and(|combat| {
                                combat["snapshot"]["active"].as_bool() == Some(true)
                            });
                        ToolExecution {
                            result: serde_json::json!({"schema_version":1,"ruleset_id":"srd-5.2.1","campaign_id":state.campaign.id.0,"session_id":state.session_id.0,"character_id":state.character_id,"revision":state.campaign.revision.0,"adventure":state.projection.adventure,"inventory":state.projection.character["inventory"],"projection":state.projection.combat,"rest_status":status}),
                            is_error: false,
                            handled_by: "engine".into(),
                            media: vec![],
                        }
                    }
                    Err(error) => ToolExecution {
                        result: serde_json::json!({"error":definitive_player_rejection(&error).expect("classified rejection"),"resolution":"rejected"}),
                        is_error: true,
                        handled_by: "engine".into(),
                        media: vec![],
                    },
                });
            }
        }
        if let app_application::agent::commands::AgentToolCommand::AdventureAction(command) =
            &request.command
        {
            use app_application::ports::repositories::AdventureRepository;
            let outcome = self
                .store
                .act_as_dm(app_application::models::adventure::AdventureIntent {
                    campaign_id: app_application::models::identity::CampaignId(request.campaign_id),
                    session_id: app_application::models::identity::SessionId(request.session_id),
                    causation_id: format!("agent:{}:{}", request.request_id, request.tool_call_id),
                    action: command.action,
                })
                .await;
            if outcome
                .as_ref()
                .err()
                .is_some_and(|error| definitive_player_rejection(error).is_none())
            {
                return Err(ToolDispatchError {
                    code: "adventure_intention_uncertain",
                });
            }
            return Ok(match outcome {
                Ok(state) => ToolExecution {
                    result: serde_json::json!({"schema_version":1,"ruleset_id":"srd-5.2.1","campaign_id":state.campaign.id.0,"session_id":state.session_id.0,"character_id":state.character_id,"revision":state.campaign.revision.0,"adventure":state.projection.adventure,"inventory":state.projection.character["inventory"],"projection":state.projection.combat}),
                    is_error: false,
                    handled_by: "engine".into(),
                    media: vec![],
                },
                Err(error) => {
                    let code = match error {
                        app_application::ports::repositories::RepositoryError::Operation {
                            code,
                            ..
                        } => code,
                        app_application::ports::repositories::RepositoryError::NotFound => {
                            "campaign_not_found"
                        }
                        _ => "campaign_revision_conflict",
                    };
                    ToolExecution {
                        result: serde_json::json!({"error":code,"resolution":"rejected"}),
                        is_error: true,
                        handled_by: "engine".into(),
                        media: vec![],
                    }
                }
            });
        }
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
                    let source = match map
                        .remove("source")
                        .and_then(|value| value.as_str().map(str::to_owned))
                    {
                        Some(source) if source == "bundled" => ImageSource::Bundled {
                            asset_id: map
                                .remove("asset_id")
                                .and_then(|value| value.as_str().map(str::to_owned))
                                .unwrap_or_else(|| "unknown".to_string()),
                        },
                        _ => {
                            map.remove("asset_id");
                            ImageSource::Generated
                        }
                    };
                    media.push(GeneratedAgentMedia::Image {
                        mime_type: map
                            .get("mime_type")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("image/png")
                            .to_string(),
                        data_b64,
                        kind: image_kind(&tool_name).unwrap_or("chat").to_string(),
                        source,
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

fn definitive_player_rejection(
    error: &app_application::ports::repositories::RepositoryError,
) -> Option<&'static str> {
    use app_application::ports::repositories::RepositoryError;
    match error {
        RepositoryError::Conflict { .. } | RepositoryError::RevisionConflict { .. } => {
            Some("player_intention_conflict")
        }
        RepositoryError::Operation { code, .. }
            if app_application::agent::intentions::definitive_rejection(code) =>
        {
            Some(code)
        }
        _ => None,
    }
}
