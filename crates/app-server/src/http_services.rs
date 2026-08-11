use std::sync::Arc;

use adapter_http::services::{
    AgentHttpService, AgentTurnHttpCommand, CampaignHttpService, CharacterAssistHttpService,
    ChatHttpCommand, ChatHttpService, CombatActionCommand as HttpCombatActionCommand,
    CombatHttpService, CombatStartCommand, HttpOutcome, HttpServiceError, LocalControlHttpService,
    LocalControlOperation, LocalEventStream, MediaHttpService, ProviderDiscoveryCommand,
    SaveMetadataCommand, SavesHttpService, SettingsHttpService, SrdSection,
};
use app_application::combat::commands::{CombatActionCommand, ResolveCombatCommand};
use app_application::combat::resolve::{ResolveCombatAction, ResolveCombatError};
use app_application::models::agent::AgentEvent;
use app_application::models::combat::{
    CombatProjection, CombatSnapshot, COMBAT_PROJECTION_VERSION,
};
use app_application::models::local_models::{RuntimeStartRequest, RuntimeState, RuntimeStatus};
use app_application::ports::media::{ImageBytes, ImagePrompt, VideoPrompt, VideoStream};
use app_application::ports::repositories::CombatRepository;
use app_application::settings::{SettingsUpdateError, UpdateSettings};
use app_domain::combat::combatant::Combatant;
use app_domain::combat::initiative::{InitiativeEntry, InitiativeOrder};
use app_domain::combat::types::{CombatantId, DamageType, Position};
use app_domain::compendium::compendium;
use app_domain::dice::{DiceExpr, Die};
use app_llm::{
    Capabilities, ChatChunk, ChatMessage, ChatRequest, LlmError, LlmProvider,
    MistralrsLocalProvider, OpenAICompatProvider,
};
use async_trait::async_trait;
use axum::extract::{Path, Query, State};
use axum::Json;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio_stream::wrappers::BroadcastStream;
use uuid::Uuid;

use crate::state::AppState;

pub fn bundle(state: AppState) -> adapter_http::services::HttpServices {
    adapter_http::services::HttpServices {
        media: media_http_service(state.clone()),
        campaign: campaign_http_service(state.clone()),
        combat: combat_http_service(state.clone()),
        saves: saves_http_service(state.clone()),
        settings: settings_http_service(state.clone()),
        agent: agent_http_service(state.clone()),
        chat: chat_http_service(state.clone()),
        character_assist: character_assist_http_service(state.clone()),
        local_control: local_control_http_service(state),
    }
}

pub struct AppStateMediaHttpService {
    state: AppState,
}

impl AppStateMediaHttpService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl MediaHttpService for AppStateMediaHttpService {
    async fn generate_image(&self, prompt: ImagePrompt) -> Result<ImageBytes, HttpServiceError> {
        let provider = self
            .state
            .image_provider()
            .ok_or(HttpServiceError::NotFound)?;
        provider
            .generate(prompt)
            .await
            .map_err(|_| HttpServiceError::Internal {
                code: "image_generation_failed",
            })
    }

    async fn generate_video(&self, prompt: VideoPrompt) -> Result<VideoStream, HttpServiceError> {
        let provider = self
            .state
            .video_provider()
            .ok_or(HttpServiceError::NotFound)?;
        provider
            .generate(prompt)
            .await
            .map_err(|_| HttpServiceError::Internal {
                code: "video_generation_failed",
            })
    }
}

pub fn media_http_service(state: AppState) -> Arc<dyn MediaHttpService> {
    Arc::new(AppStateMediaHttpService::new(state))
}

pub struct AppStateCampaignHttpService {
    state: AppState,
}

impl AppStateCampaignHttpService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

fn serialization_error() -> HttpServiceError {
    HttpServiceError::Internal {
        code: "response_serialization_failed",
    }
}

#[async_trait]
impl CampaignHttpService for AppStateCampaignHttpService {
    async fn journal(&self, campaign_id: Uuid) -> Result<Value, HttpServiceError> {
        let entries = crate::db::journal_list(self.state.db(), campaign_id)
            .await
            .map_err(|_| HttpServiceError::Internal {
                code: "journal_list_failed",
            })?;
        serde_json::to_value(entries).map_err(|_| serialization_error())
    }

    async fn npcs(&self, campaign_id: Uuid) -> Result<Value, HttpServiceError> {
        let npcs = crate::db::npc_get_all(self.state.db(), campaign_id)
            .await
            .map_err(|_| HttpServiceError::Internal {
                code: "npc_list_failed",
            })?;
        serde_json::to_value(npcs).map_err(|_| serialization_error())
    }

    async fn messages(&self, session_id: String) -> Result<Value, HttpServiceError> {
        let messages = crate::db::list_messages_by_session(self.state.db(), &session_id)
            .await
            .map_err(|_| HttpServiceError::Internal {
                code: "message_list_failed",
            })?;
        Ok(json!({ "messages": messages }))
    }

    async fn srd(&self, section: SrdSection) -> Result<Value, HttpServiceError> {
        let source = compendium();
        match section {
            SrdSection::Races => serde_json::to_value(&source.races),
            SrdSection::Classes => serde_json::to_value(&source.classes),
            SrdSection::Backgrounds => serde_json::to_value(&source.backgrounds),
            SrdSection::Spells => serde_json::to_value(&source.spells),
            SrdSection::Equipment => Ok(json!({
                "weapons": source.equipment.weapons,
                "armor": source.equipment.armor,
                "adventuring_gear": source.equipment.adventuring_gear,
            })),
            SrdSection::Feats => serde_json::to_value(&source.feats),
            SrdSection::WeaponProperties => serde_json::to_value(&source.weapon_properties),
        }
        .map_err(|_| serialization_error())
    }
}

pub fn campaign_http_service(state: AppState) -> Arc<dyn CampaignHttpService> {
    Arc::new(AppStateCampaignHttpService::new(state))
}

pub struct AppStateCombatHttpService {
    state: AppState,
}

impl AppStateCombatHttpService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[derive(Debug, Deserialize)]
struct AttackActionArgs {
    attacker_id: Uuid,
    target_id: Uuid,
    #[serde(default)]
    attack_modifier: i32,
    damage_dice: String,
    damage_type: DamageType,
}

#[derive(Debug, Deserialize)]
struct ActorActionArgs {
    combatant_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct MoveActionArgs {
    combatant_id: Uuid,
    x: i32,
    y: i32,
}

#[async_trait]
impl CombatHttpService for AppStateCombatHttpService {
    async fn start(&self, command: CombatStartCommand) -> Result<Uuid, HttpServiceError> {
        let encounter_id = Uuid::new_v4();
        let projection = projection_from_start(encounter_id, &command.initiative_entries);
        let repository = adapter_sqlite::SqliteStore::new(self.state.db().clone());
        repository
            .create(command.session_id, projection)
            .await
            .map_err(|_| HttpServiceError::Internal {
                code: "combat_start_failed",
            })?;
        Ok(encounter_id)
    }

    async fn action(
        &self,
        command: HttpCombatActionCommand,
    ) -> Result<Option<CombatProjection>, HttpServiceError> {
        let action = match command.action_type.as_str() {
            "attack" => {
                let args: AttackActionArgs =
                    serde_json::from_value(command.args).map_err(|_| {
                        HttpServiceError::BadRequest {
                            code: "combat_action_invalid",
                        }
                    })?;
                let damage_expr =
                    parse_damage_dice(&args.damage_dice).ok_or(HttpServiceError::BadRequest {
                        code: "damage_dice_invalid",
                    })?;
                CombatActionCommand::Attack {
                    attacker: CombatantId(args.attacker_id),
                    target: CombatantId(args.target_id),
                    attack_modifier: args.attack_modifier,
                    damage_expr,
                    damage_type: args.damage_type,
                }
            }
            "cast" => {
                let args: ActorActionArgs = serde_json::from_value(command.args).map_err(|_| {
                    HttpServiceError::BadRequest {
                        code: "combat_action_invalid",
                    }
                })?;
                CombatActionCommand::Cast {
                    combatant: CombatantId(args.combatant_id),
                }
            }
            "move" => {
                let args: MoveActionArgs = serde_json::from_value(command.args).map_err(|_| {
                    HttpServiceError::BadRequest {
                        code: "combat_action_invalid",
                    }
                })?;
                CombatActionCommand::Move {
                    combatant: CombatantId(args.combatant_id),
                    to: Position {
                        x: args.x,
                        y: args.y,
                    },
                }
            }
            "end_turn" => {
                let args: ActorActionArgs = serde_json::from_value(command.args).map_err(|_| {
                    HttpServiceError::BadRequest {
                        code: "combat_action_invalid",
                    }
                })?;
                CombatActionCommand::EndTurn {
                    combatant: CombatantId(args.combatant_id),
                }
            }
            _ => {
                return Err(HttpServiceError::BadRequest {
                    code: "combat_action_invalid",
                });
            }
        };
        let repository = Arc::new(adapter_sqlite::SqliteStore::new(self.state.db().clone()));
        let result = ResolveCombatAction::new(repository)
            .execute(ResolveCombatCommand {
                request_id: command
                    .request_id
                    .unwrap_or_else(|| Uuid::new_v4().to_string()),
                encounter_id: command.encounter_id,
                expected_revision: command.expected_revision.unwrap_or(0),
                rng_seed: command.rng_seed.unwrap_or(0),
                action,
            })
            .await
            .map_err(combat_error)?;
        Ok(Some(result.projection))
    }

    async fn end(&self, encounter_id: Uuid) -> Result<(), HttpServiceError> {
        let repository = adapter_sqlite::SqliteStore::new(self.state.db().clone());
        repository
            .end(encounter_id)
            .await
            .map_err(|_| HttpServiceError::Internal {
                code: "combat_end_failed",
            })?;
        Ok(())
    }
}

fn projection_from_start(
    encounter_id: Uuid,
    entries: &[adapter_http::routes::combat::InitiativeEntryDto],
) -> CombatProjection {
    let initiative = entries
        .iter()
        .map(|entry| InitiativeEntry {
            id: CombatantId(entry.id),
            roll: entry.roll,
            dex_tiebreak: entry.dex_mod,
        })
        .collect::<Vec<_>>();
    let order = InitiativeOrder::build(initiative);
    let combatants = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let mut combatant = Combatant::new(
                CombatantId(entry.id),
                entry.name.clone(),
                entry.max_hp,
                entry.hp,
                entry.ac,
            );
            combatant.initiative_roll = entry.roll;
            combatant.dex_mod = entry.dex_mod;
            combatant.position = Position {
                x: (index % 8) as i32,
                y: (index / 8) as i32,
            };
            combatant
        })
        .collect();
    CombatProjection {
        schema_version: COMBAT_PROJECTION_VERSION,
        encounter_id,
        revision: 0,
        snapshot: CombatSnapshot {
            active: true,
            round: 1,
            current_combatant: (!order.is_empty()).then(|| order.current().id),
            initiative: order.as_slice().to_vec(),
            combatants,
        },
        events: Vec::new(),
    }
}

fn parse_damage_dice(input: &str) -> Option<DiceExpr> {
    let normalized = input.trim().to_ascii_lowercase();
    let (count, remainder) = normalized.split_once('d')?;
    let (sides, modifier) = remainder
        .split_once('+')
        .map(|(sides, modifier)| (sides, modifier.parse::<i32>().ok()))
        .or_else(|| {
            remainder
                .split_once('-')
                .map(|(sides, modifier)| (sides, modifier.parse::<i32>().ok().map(|value| -value)))
        })
        .unwrap_or((remainder, Some(0)));
    let die = match sides.parse::<u16>().ok()? {
        4 => Die::D4,
        6 => Die::D6,
        8 => Die::D8,
        10 => Die::D10,
        12 => Die::D12,
        20 => Die::D20,
        100 => Die::D100,
        _ => return None,
    };
    let count = count.parse::<u8>().ok()?;
    (count > 0).then_some(DiceExpr {
        count,
        die,
        modifier: modifier?,
    })
}

fn combat_error(error: ResolveCombatError) -> HttpServiceError {
    match error {
        ResolveCombatError::NotFound => HttpServiceError::BadRequest {
            code: "combat_not_found",
        },
        ResolveCombatError::StaleRevision { .. } => HttpServiceError::BadRequest {
            code: "combat_revision_stale",
        },
        ResolveCombatError::Rejected(_) => HttpServiceError::BadRequest {
            code: "combat_action_rejected",
        },
        ResolveCombatError::Persistence => HttpServiceError::Internal {
            code: "combat_persistence_failed",
        },
    }
}

pub fn combat_http_service(state: AppState) -> Arc<dyn CombatHttpService> {
    Arc::new(AppStateCombatHttpService::new(state))
}

pub struct AppStateSavesHttpService {
    state: AppState,
}

impl AppStateSavesHttpService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

fn save_envelope(metadata: &SaveMetadataCommand) -> Value {
    json!({
        "schema_version": 1,
        "state": {
            "title": metadata.title,
            "summary": metadata.summary,
            "tag": metadata.tag,
            "kind": metadata.kind,
        }
    })
}

fn save_db_error(code: &'static str) -> impl FnOnce(sqlx::Error) -> HttpServiceError {
    move |_| HttpServiceError::Internal { code }
}

#[async_trait]
impl SavesHttpService for AppStateSavesHttpService {
    async fn list(&self, session_id: Uuid) -> Result<Value, HttpServiceError> {
        let saves = crate::db::save_list_by_session(self.state.db(), session_id)
            .await
            .map_err(save_db_error("save_list_failed"))?;
        serde_json::to_value(saves).map_err(|_| serialization_error())
    }

    async fn create(
        &self,
        session_id: Uuid,
        metadata: SaveMetadataCommand,
    ) -> Result<Uuid, HttpServiceError> {
        let envelope = save_envelope(&metadata);
        crate::db::save_insert(
            self.state.db(),
            session_id,
            &metadata.kind,
            &metadata.title,
            &metadata.summary,
            &metadata.tag,
            &envelope,
        )
        .await
        .map_err(save_db_error("save_create_failed"))
    }

    async fn quick(&self, session_id: Uuid) -> Result<Uuid, HttpServiceError> {
        self.create(
            session_id,
            SaveMetadataCommand {
                kind: "auto".into(),
                title: "Quick save".into(),
                summary: "(no scene)".into(),
                tag: "exploration".into(),
            },
        )
        .await
    }

    async fn get(&self, save_id: Uuid) -> Result<Value, HttpServiceError> {
        let save = crate::db::save_load(self.state.db(), save_id)
            .await
            .map_err(save_db_error("save_load_failed"))?
            .ok_or(HttpServiceError::NotFound)?;
        serde_json::to_value(save).map_err(|_| serialization_error())
    }

    async fn delete(&self, save_id: Uuid) -> Result<bool, HttpServiceError> {
        crate::db::save_delete(self.state.db(), save_id)
            .await
            .map_err(save_db_error("save_delete_failed"))
    }

    async fn restore(&self, session_id: Uuid, save_id: Uuid) -> Result<Value, HttpServiceError> {
        let game_state = crate::db::restore_snapshot(self.state.db(), session_id, save_id)
            .await
            .map_err(save_db_error("save_restore_failed"))?
            .ok_or(HttpServiceError::NotFound)?;
        // Fetch the render history before returning from the mutating operation.
        // The frontend can therefore validate one complete restore projection and
        // commit it atomically without a fallible GET after backend state changed.
        let messages =
            crate::db::list_messages_by_session(self.state.db(), &session_id.to_string())
                .await
                .map_err(save_db_error("save_messages_failed"))?;
        Ok(json!({ "game_state": game_state, "messages": messages }))
    }

    async fn update(
        &self,
        save_id: Uuid,
        metadata: SaveMetadataCommand,
    ) -> Result<bool, HttpServiceError> {
        let envelope = save_envelope(&metadata);
        crate::db::save_update(
            self.state.db(),
            save_id,
            &metadata.kind,
            &metadata.title,
            &metadata.summary,
            &metadata.tag,
            &envelope,
        )
        .await
        .map_err(save_db_error("save_update_failed"))
    }
}

pub fn saves_http_service(state: AppState) -> Arc<dyn SavesHttpService> {
    Arc::new(AppStateSavesHttpService::new(state))
}

pub struct AppStateSettingsHttpService {
    state: AppState,
}

impl AppStateSettingsHttpService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl SettingsHttpService for AppStateSettingsHttpService {
    async fn provider_info(&self) -> Result<Value, HttpServiceError> {
        Ok(json!({
            "available": ["openai-compat", "local-mistralrs"],
            "active": {
                "kind": self.state.provider().name(),
                "default_model": self.state.default_model(),
            }
        }))
    }

    async fn update(
        &self,
        config: app_application::models::settings::SettingsConfigV2,
    ) -> Result<Value, HttpServiceError> {
        if !(0.0..=2.0).contains(&config.behavior.temperature) {
            return Err(HttpServiceError::BadRequest {
                code: "temperature must be between 0.0 and 2.0",
            });
        }
        crate::control_services::settings::validate_settings_v2(&config).map_err(|_| {
            HttpServiceError::BadRequest {
                code: "settings_invalid",
            }
        })?;
        let update = UpdateSettings::new(
            Arc::new(
                crate::providers::settings_factory::ServerSettingsFactory::new(self.state.clone()),
            ),
            self.state.secrets_repo(),
            Arc::new(
                crate::providers::settings_factory::AppStateSettingsCommit::new(self.state.clone()),
            ),
        );
        let result = update.execute(config).await.map_err(|error| match error {
            SettingsUpdateError::Prepare { .. } => HttpServiceError::BadRequest {
                code: "settings_prepare_failed",
            },
            SettingsUpdateError::Secret { .. } | SettingsUpdateError::Commit { .. } => {
                HttpServiceError::Internal {
                    code: "settings_commit_failed",
                }
            }
        })?;
        Ok(json!({
            "status": "ok",
            "license_restricted_no_compat": result.license_restricted_no_compat,
        }))
    }

    async fn catalog(&self) -> Result<Value, HttpServiceError> {
        serde_json::to_value(json!({
            "chat": crate::providers::catalog::CHAT_CATALOG,
            "image": crate::providers::catalog::IMAGE_CATALOG,
            "video": crate::providers::catalog::VIDEO_CATALOG,
        }))
        .map_err(|_| serialization_error())
    }

    async fn capabilities(
        &self,
        provider_id: String,
        model_id: String,
    ) -> Result<Option<Value>, HttpServiceError> {
        let Some(entry) = crate::providers::catalog::find_entry_any_modality(&provider_id) else {
            return Ok(None);
        };
        let capabilities = if let Some(curated) = entry
            .curated_models
            .iter()
            .find(|model| model.model_id == model_id)
        {
            curated.capabilities
        } else {
            match provider_id.as_str() {
                "openai-compat" => OpenAICompatProvider::new(String::new(), String::new())
                    .capabilities_for_model(&model_id),
                "local-mistralrs" => MistralrsLocalProvider::new(0, model_id.clone())
                    .capabilities_for_model(&model_id),
                _ => Capabilities::default(),
            }
        };
        serde_json::to_value(capabilities)
            .map(Some)
            .map_err(|_| serialization_error())
    }

    async fn discover(&self, command: ProviderDiscoveryCommand) -> Result<Value, HttpServiceError> {
        use crate::providers::discovery::{
            merge_recommended, recommended_for, DiscoverParams, DiscoveryError, DiscoverySource,
            HfHubSearch, OpenAIV1Models, ReplicateSearch,
        };

        let provider_id = command.provider_id.clone();
        let params = DiscoverParams {
            provider_id: command.provider_id,
            base_url: command.base_url,
            api_key: command.api_key,
            search_query: command.search_query,
            cursor: command.cursor,
        };
        let result = match provider_id.as_str() {
            "openai" | "openai-compat" => OpenAIV1Models::default().discover(params).await,
            "local-mistralrs" => HfHubSearch::default().discover(params).await,
            "replicate" => ReplicateSearch::default().discover(params).await,
            unknown => Err(DiscoveryError::UnsupportedProvider(unknown.to_string())),
        };
        let mut result = result.map_err(|error| match error {
            DiscoveryError::UnsupportedProvider(_) => HttpServiceError::NotFound,
            DiscoveryError::Unauthorized => HttpServiceError::Unauthorized {
                code: "discovery_unauthorized",
            },
            DiscoveryError::RateLimit => HttpServiceError::RateLimit {
                code: "discovery_rate_limit",
            },
            _ => HttpServiceError::BadGateway {
                code: "discovery_failed",
            },
        })?;
        let recommended = recommended_for(&provider_id);
        if !recommended.is_empty() {
            let discovered = std::mem::take(&mut result.models);
            result.models = merge_recommended(recommended, discovered);
        }
        serde_json::to_value(result).map_err(|_| serialization_error())
    }
}

pub fn settings_http_service(state: AppState) -> Arc<dyn SettingsHttpService> {
    Arc::new(AppStateSettingsHttpService::new(state))
}

pub struct AppStateAgentHttpService {
    state: AppState,
}

impl AppStateAgentHttpService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl AgentHttpService for AppStateAgentHttpService {
    async fn turn(
        &self,
        command: AgentTurnHttpCommand,
    ) -> Result<mpsc::Receiver<AgentEvent>, HttpServiceError> {
        let provider = self.state.provider();
        let mut config = self.state.agent_config();
        if let Some(model) = command.model {
            config.model = model;
        }
        let retriever = self.state.srd_retriever();
        let image_provider = self.state.image_provider();
        let video_provider = self.state.video_provider();
        // Process ownership lives in Tauri. The backend cannot directly stop or
        // restart the model child, so no legacy in-process GPU swap is built.
        let gpu_swap = None;
        let pool = self.state.db().clone();
        let (sender, receiver) = mpsc::channel::<AgentEvent>(64);
        tokio::spawn(async move {
            let orchestrator = crate::agent::orchestrator::AgentOrchestrator::new(
                provider,
                pool,
                config,
                retriever,
                image_provider,
            )
            .with_gpu_swap(gpu_swap)
            .with_video_provider(video_provider);
            if let Err(error) = orchestrator.run(command.request, sender).await {
                tracing::warn!(error = %error, "agent loop error");
            }
        });
        Ok(receiver)
    }
}

pub fn agent_http_service(state: AppState) -> Arc<dyn AgentHttpService> {
    Arc::new(AppStateAgentHttpService::new(state))
}

pub struct AppStateChatHttpService {
    state: AppState,
}

impl AppStateChatHttpService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl ChatHttpService for AppStateChatHttpService {
    async fn stream(
        &self,
        command: ChatHttpCommand,
    ) -> Result<app_application::ports::llm::ChunkStream, HttpServiceError> {
        if let (Some(session_id), Some(last)) =
            (command.session_id.as_deref(), command.messages.last())
        {
            if matches!(last, ChatMessage::User { .. }) {
                if let Err(error) =
                    crate::db::insert_message(self.state.db(), session_id, last).await
                {
                    tracing::warn!(error = %error, "failed to persist user message");
                }
            }
        }
        let chunks = self
            .state
            .provider()
            .stream_chat(ChatRequest {
                messages: command.messages,
                model: command.model.unwrap_or_else(|| self.state.default_model()),
                max_tokens: command.max_tokens,
                temperature: command.temperature,
                tools: Vec::new(),
                system_prompt: None,
                reasoning: None,
            })
            .await
            .map_err(llm_http_error)?;
        let session_id = command.session_id;
        let pool = self.state.db().clone();
        let buffer = Arc::new(std::sync::Mutex::new(String::new()));
        Ok(Box::pin(chunks.map(move |chunk| {
            match &chunk {
                Ok(ChatChunk::TextDelta { text }) => {
                    if let Ok(mut output) = buffer.lock() {
                        output.push_str(text);
                    }
                }
                Ok(ChatChunk::Done { .. }) => {
                    if let Some(session_id) = session_id.clone() {
                        let text = buffer.lock().map(|output| output.clone()).unwrap_or_default();
                        if !text.is_empty() {
                            let pool = pool.clone();
                            tokio::spawn(async move {
                                let message = ChatMessage::Assistant { content: text };
                                if let Err(error) = crate::db::insert_message(&pool, &session_id, &message).await {
                                    tracing::warn!(error = %error, "failed to persist assistant message");
                                }
                            });
                        }
                    }
                }
                _ => {}
            }
            chunk
        })))
    }
}

fn llm_http_error(error: LlmError) -> HttpServiceError {
    match error {
        LlmError::AuthFailure => HttpServiceError::Unauthorized {
            code: "auth_failed",
        },
        LlmError::RateLimit => HttpServiceError::RateLimit { code: "rate_limit" },
        _ => HttpServiceError::BadGateway {
            code: "provider_error",
        },
    }
}

pub fn chat_http_service(state: AppState) -> Arc<dyn ChatHttpService> {
    Arc::new(AppStateChatHttpService::new(state))
}

pub struct AppStateCharacterAssistHttpService {
    state: AppState,
}

#[async_trait]
impl CharacterAssistHttpService for AppStateCharacterAssistHttpService {
    async fn stream(
        &self,
        mut request: ChatRequest,
    ) -> Result<app_application::ports::llm::ChunkStream, HttpServiceError> {
        request.model = self.state.default_model();
        self.state
            .provider()
            .stream_chat(request)
            .await
            .map_err(llm_http_error)
    }
}

pub fn character_assist_http_service(state: AppState) -> Arc<dyn CharacterAssistHttpService> {
    Arc::new(AppStateCharacterAssistHttpService { state })
}

pub struct AppStateLocalControlHttpService {
    state: AppState,
}

impl AppStateLocalControlHttpService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

fn ok_status(status: axum::http::StatusCode) -> HttpOutcome {
    HttpOutcome {
        status: status.as_u16(),
        body: None,
    }
}

fn ok_json<T: serde::Serialize>(value: T) -> Result<HttpOutcome, HttpServiceError> {
    Ok(HttpOutcome {
        status: 200,
        body: Some(serde_json::to_value(value).map_err(|_| serialization_error())?),
    })
}

fn app_http_error(error: crate::error::AppError) -> HttpServiceError {
    match error {
        crate::error::AppError::NotFound => HttpServiceError::NotFound,
        crate::error::AppError::BadRequest(_) => HttpServiceError::BadRequest {
            code: "request_invalid",
        },
        crate::error::AppError::PayloadTooLarge(_) => HttpServiceError::PayloadTooLarge {
            code: "payload_too_large",
        },
        crate::error::AppError::Llm(error) => llm_http_error(error),
        crate::error::AppError::Db(_) | crate::error::AppError::Internal(_) => {
            HttpServiceError::Internal {
                code: "operation_failed",
            }
        }
    }
}

#[async_trait]
impl LocalControlHttpService for AppStateLocalControlHttpService {
    async fn execute(
        &self,
        operation: LocalControlOperation,
    ) -> Result<HttpOutcome, HttpServiceError> {
        use crate::control_services::{hf, local_llm, local_mode};
        match operation {
            LocalControlOperation::HfSetToken { token } => {
                let Json(status) =
                    hf::post_token(State(self.state.clone()), Json(hf::TokenBody { token }))
                        .await
                        .map_err(app_http_error)?;
                ok_json(status)
            }
            LocalControlOperation::HfDeleteToken => hf::delete_token(State(self.state.clone()))
                .await
                .map(ok_status)
                .map_err(app_http_error),
            LocalControlOperation::HfTokenStatus => {
                let Json(status) = hf::get_token_status(State(self.state.clone())).await;
                ok_json(status)
            }
            LocalControlOperation::HfSearch { query } => {
                let params: hf::SearchParams =
                    serde_json::from_value(query).map_err(|_| HttpServiceError::BadRequest {
                        code: "search_query_invalid",
                    })?;
                let Json(models) = hf::search(State(self.state.clone()), Query(params))
                    .await
                    .map_err(app_http_error)?;
                ok_json(models)
            }
            LocalControlOperation::HfLicense { repo_id } => {
                let value = hf::license_check_value(&self.state, &repo_id)
                    .await
                    .map_err(app_http_error)?;
                ok_json(value)
            }
            LocalControlOperation::HfManifestAdd { body } => {
                let body: hf::AddManifestBody =
                    serde_json::from_value(body).map_err(|_| HttpServiceError::BadRequest {
                        code: "manifest_invalid",
                    })?;
                hf::add_manifest(State(self.state.clone()), Json(body))
                    .await
                    .map(ok_status)
                    .map_err(app_http_error)
            }
            LocalControlOperation::HfManifestDelete { id } => {
                hf::delete_manifest(State(self.state.clone()), Path(id))
                    .await
                    .map(ok_status)
                    .map_err(app_http_error)
            }
            LocalControlOperation::LocalLlmManifest => {
                let Json(manifest) = local_llm::get_manifest(State(self.state.clone())).await;
                ok_json(manifest)
            }
            LocalControlOperation::LocalLlmSetActive { id } => local_llm::set_active_model(
                State(self.state.clone()),
                Json(local_llm::SetActiveModelRequest { id }),
            )
            .await
            .map(ok_status)
            .map_err(app_http_error),
            LocalControlOperation::LocalLlmStartDownload { id } => {
                local_llm::start_download(State(self.state.clone()), Path(id))
                    .await
                    .map(ok_status)
                    .map_err(app_http_error)
            }
            LocalControlOperation::LocalLlmCancelOrDelete { id } => {
                local_llm::cancel_or_delete(State(self.state.clone()), Path(id))
                    .await
                    .map(ok_status)
                    .map_err(app_http_error)
            }
            LocalControlOperation::LocalModeGetConfig => {
                let Json(config) = local_mode::get_config(State(self.state.clone())).await;
                ok_json(config)
            }
            LocalControlOperation::LocalModeSetConfig { config } => {
                let config: local_mode::LocalModeConfig =
                    serde_json::from_value(config).map_err(|_| HttpServiceError::BadRequest {
                        code: "local_config_invalid",
                    })?;
                let Json(config) = local_mode::post_config(State(self.state.clone()), Json(config))
                    .await
                    .map_err(app_http_error)?;
                ok_json(config)
            }
            LocalControlOperation::LocalModeStartDownload { id } => {
                let id = serde_json::from_value(serde_json::Value::String(id)).map_err(|_| {
                    HttpServiceError::BadRequest {
                        code: "model_id_invalid",
                    }
                })?;
                local_mode::post_download(State(self.state.clone()), Path(id))
                    .await
                    .map(ok_status)
                    .map_err(app_http_error)
            }
            LocalControlOperation::LocalModeDeleteDownload { id } => {
                let id = serde_json::from_value(serde_json::Value::String(id)).map_err(|_| {
                    HttpServiceError::BadRequest {
                        code: "model_id_invalid",
                    }
                })?;
                Ok(ok_status(
                    local_mode::delete_download(State(self.state.clone()), Path(id)).await,
                ))
            }
            LocalControlOperation::LocalRuntimeStart => {
                let config = self.state.local_mode_config();
                let llm_args =
                    local_mode::build_llm_spawn_args(&self.state, 0).map_err(app_http_error)?;
                let model_id = serde_json::to_value(&config.selected_llm)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .ok_or(HttpServiceError::BadRequest {
                        code: "model_id_invalid",
                    })?;
                let media_enabled =
                    config.vram_strategy != local_mode::VramStrategy::DisableImageGen;
                let status = self
                    .state
                    .runtime_control()
                    .start(RuntimeStartRequest {
                        model_id,
                        enable_image: media_enabled,
                        enable_video: media_enabled,
                        llm_args,
                        weights_dir: Some(self.state.models_dir().to_string_lossy().into_owned()),
                    })
                    .await
                    .map_err(runtime_http_error)?;
                self.apply_runtime_status(&status);
                ok_json(runtime_wire_status(&status))
            }
            LocalControlOperation::LocalRuntimeStop => {
                let status = self
                    .state
                    .runtime_control()
                    .stop()
                    .await
                    .map_err(runtime_http_error)?;
                self.apply_runtime_status(&status);
                ok_json(runtime_wire_status(&status))
            }
            LocalControlOperation::LocalRuntimeStatus => {
                let status = self
                    .state
                    .runtime_control()
                    .status()
                    .await
                    .map_err(runtime_http_error)?;
                self.apply_runtime_status(&status);
                ok_json(runtime_wire_status(&status))
            }
        }
    }

    async fn events(
        &self,
        stream: LocalEventStream,
    ) -> Result<mpsc::Receiver<Value>, HttpServiceError> {
        let mut source = BroadcastStream::new(self.state.download_manager().events.subscribe());
        let (sender, receiver) = mpsc::channel(32);
        tokio::spawn(async move {
            while let Some(event) = source.next().await {
                let Ok(event) = event else { continue };
                let value = match &stream {
                    LocalEventStream::LocalLlmDownloads => {
                        crate::control_services::local_llm::to_wire(event)
                            .and_then(|event| serde_json::to_value(event).ok())
                    }
                    LocalEventStream::LocalModeDownload { id } => {
                        let event_id = match &event {
                            crate::models::DownloadEvent::Progress { id, .. }
                            | crate::models::DownloadEvent::Completed { id, .. }
                            | crate::models::DownloadEvent::Failed { id, .. } => id,
                        };
                        let expected: Result<crate::models::manifest::ModelId, _> =
                            serde_json::from_value(Value::String(id.clone()));
                        (expected.as_ref().ok() == Some(event_id))
                            .then(|| serde_json::to_value(event).ok())
                            .flatten()
                    }
                };
                if let Some(value) = value {
                    if sender.send(value).await.is_err() {
                        break;
                    }
                }
            }
        });
        Ok(receiver)
    }
}

impl AppStateLocalControlHttpService {
    fn apply_runtime_status(&self, status: &RuntimeStatus) {
        self.state.set_media_sidecar_url(
            status
                .media_port
                .map(|port| format!("http://127.0.0.1:{port}")),
        );
    }
}

fn runtime_http_error(_error: app_application::ports::runtime::RuntimeError) -> HttpServiceError {
    HttpServiceError::BadGateway {
        code: "runtime_control_failed",
    }
}

fn runtime_wire_status(status: &RuntimeStatus) -> Value {
    let failed_reason = status
        .failure_code
        .clone()
        .unwrap_or_else(|| "runtime_failed".to_owned());
    let llm = match status.state {
        RuntimeState::Stopped => json!({ "state": "off" }),
        RuntimeState::Starting => json!({ "state": "starting" }),
        RuntimeState::Running | RuntimeState::Degraded => status
            .llm_port
            .map(|port| json!({ "state": "ready", "port": port }))
            .unwrap_or_else(|| json!({ "state": "failed", "reason": failed_reason })),
        RuntimeState::Failed => json!({ "state": "failed", "reason": failed_reason }),
    };
    let image = if !status.image_enabled {
        json!({ "state": "off" })
    } else {
        match status.state {
            RuntimeState::Stopped => json!({ "state": "off" }),
            RuntimeState::Starting => json!({ "state": "starting" }),
            RuntimeState::Running => status
                .media_port
                .map(|port| json!({ "state": "ready", "port": port }))
                .unwrap_or_else(|| json!({ "state": "failed", "reason": failed_reason })),
            RuntimeState::Degraded | RuntimeState::Failed => {
                json!({ "state": "failed", "reason": failed_reason })
            }
        }
    };
    json!({ "llm": llm, "image": image })
}

pub fn local_control_http_service(state: AppState) -> Arc<dyn LocalControlHttpService> {
    Arc::new(AppStateLocalControlHttpService::new(state))
}
