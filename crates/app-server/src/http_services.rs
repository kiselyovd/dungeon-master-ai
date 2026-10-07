use std::sync::Arc;

use adapter_http::routes::campaigns::{
    CampaignResponse, CampaignStateResponse, CampaignSummaryResponse,
};
use adapter_http::services::{
    AgentHttpService, AgentTurnHttpCommand, CampaignHttpService, CharacterAssistHttpService,
    ChatHttpCommand, ChatHttpService, CombatActionCommand as HttpCombatActionCommand,
    CombatHttpService, CombatStartCommand, HttpOutcome, HttpServiceError, LocalControlHttpService,
    LocalControlOperation, LocalEventStream, MediaHttpService, ProviderDiscoveryCommand,
    RestoreSaveCommand, SaveCreated, SaveMetadataCommand, SavesHttpService, SettingsHttpService,
    SrdSection,
};
use adapter_sqlite::SqliteStore;
use app_application::campaigns::{
    CreateCampaign, CreateCampaignCommand, CreateSnapshot, CreateSnapshotCommand, GetCampaign,
    ListCampaigns, RestoreSnapshot, RestoreSnapshotCommand,
};
use app_application::combat::commands::{CombatActionCommand, ResolveCombatCommand};
use app_application::combat::resolve::{ResolveCombatAction, ResolveCombatError};
use app_application::models::agent::AgentEvent;
use app_application::models::combat::{
    CombatProjection, CombatSnapshot, COMBAT_PROJECTION_VERSION,
};
use app_application::models::identity::{CampaignId, Revision, SessionId};
use app_application::models::local_models::{RuntimeStartRequest, RuntimeState, RuntimeStatus};
use app_application::models::session::{CampaignDetail, CampaignSummary};
use app_application::ports::media::{
    ImageBytes, ImageError, ImagePrompt, VideoPrompt, VideoStream,
};
use app_application::ports::repositories::{CombatRepository, SnapshotRepository};
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
use chrono::{DateTime, SecondsFormat, Utc};
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
        provider.generate(prompt).await.map_err(image_http_error)
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

fn image_http_error(error: ImageError) -> HttpServiceError {
    match error {
        ImageError::Auth => HttpServiceError::Unauthorized {
            code: "image_auth_failed",
        },
        ImageError::Degraded {
            code: "bundled_fallback_failed",
        } => HttpServiceError::Internal {
            code: "bundled_fallback_failed",
        },
        ImageError::Provider(_)
        | ImageError::Network(_)
        | ImageError::Timeout { .. }
        | ImageError::Degraded { .. }
        | ImageError::Cancelled => HttpServiceError::Internal {
            code: "image_generation_failed",
        },
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
    async fn rest_status(
        &self,
        id: Uuid,
    ) -> Result<app_application::models::rest::RestStatus, HttpServiceError> {
        app_application::rest::RestCampaign::new(Arc::new(SqliteStore::new(
            self.state.db().clone(),
        )))
        .status(CampaignId(id))
        .await
        .map_err(rest_error)
    }
    async fn rest(
        &self,
        command: app_application::models::rest::RestCommand,
    ) -> Result<app_application::models::rest::RestResult, HttpServiceError> {
        let _lane = self
            .state
            .campaign_lane(command.campaign_id.0)
            .lock_owned()
            .await;
        app_application::rest::RestCampaign::new(Arc::new(SqliteStore::new(
            self.state.db().clone(),
        )))
        .execute(command)
        .await
        .map_err(rest_error)
    }
    async fn adventure_action(
        &self,
        command: app_application::models::adventure::AdventureCommand,
    ) -> Result<CampaignStateResponse, HttpServiceError> {
        let _lane = self
            .state
            .campaign_lane(command.campaign_id.0)
            .lock_owned()
            .await;
        let repository = Arc::new(SqliteStore::new(self.state.db().clone()));
        let state = app_application::campaigns::ActInAdventure::new(repository)
            .execute(command)
            .await
            .map_err(campaign_use_case_error)?;
        campaign_state_response(state)
    }
    async fn state(&self, id: Uuid) -> Result<CampaignStateResponse, HttpServiceError> {
        let repository = Arc::new(SqliteStore::new(self.state.db().clone()));
        let state = app_application::campaigns::LoadCampaignSession::new(repository)
            .execute(CampaignId(id))
            .await
            .map_err(campaign_use_case_error)?;
        campaign_state_response(state)
    }

    async fn commit_character(
        &self,
        command: app_application::models::campaign_state::CommitCampaignCharacter,
    ) -> Result<CampaignStateResponse, HttpServiceError> {
        let _lane = self
            .state
            .campaign_lane(command.campaign_id.0)
            .lock_owned()
            .await;
        let repository = Arc::new(SqliteStore::new(self.state.db().clone()));
        let state = app_application::campaigns::CommitCharacter::new(repository)
            .execute(command)
            .await
            .map_err(campaign_use_case_error)?;
        campaign_state_response(state)
    }
    async fn create_campaign(
        &self,
        command: CreateCampaignCommand,
    ) -> Result<CampaignResponse, HttpServiceError> {
        let repository = Arc::new(SqliteStore::new(self.state.db().clone()));
        let detail = CreateCampaign::new(repository)
            .execute(command)
            .await
            .map_err(campaign_use_case_error)?;
        campaign_response(detail)
    }

    async fn list_campaigns(&self) -> Result<Vec<CampaignSummaryResponse>, HttpServiceError> {
        let repository = Arc::new(SqliteStore::new(self.state.db().clone()));
        ListCampaigns::new(repository)
            .execute()
            .await
            .map_err(campaign_use_case_error)?
            .into_iter()
            .map(campaign_summary_response)
            .collect()
    }

    async fn get_campaign(&self, campaign_id: Uuid) -> Result<CampaignResponse, HttpServiceError> {
        let repository = Arc::new(SqliteStore::new(self.state.db().clone()));
        let detail = GetCampaign::new(repository)
            .execute(CampaignId(campaign_id))
            .await
            .map_err(campaign_use_case_error)?
            .ok_or(HttpServiceError::NotFound)?;
        campaign_response(detail)
    }

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

fn campaign_use_case_error(error: app_application::campaigns::CampaignError) -> HttpServiceError {
    match error {
        app_application::campaigns::CampaignError::InvalidInput { code } => {
            HttpServiceError::UnprocessableEntity { code }
        }
        app_application::campaigns::CampaignError::Persistence {
            code: "snapshot_revision_conflict" | "campaign_revision_conflict",
        } => HttpServiceError::Conflict {
            code: "snapshot_revision_conflict",
        },
        app_application::campaigns::CampaignError::Persistence {
            code: "snapshot_not_found" | "campaign_not_found",
        } => HttpServiceError::NotFound,
        app_application::campaigns::CampaignError::Persistence {
            code:
                code @ ("character_invalid"
                | "character_identity_mismatch"
                | "character_session_mismatch"
                | "character_causation_mismatch"
                | "character_too_large"
                | "snapshot_causation_mismatch"
                | "adventure_action_unavailable"
                | "adventure_session_mismatch"
                | "adventure_character_mismatch"
                | "adventure_causation_mismatch"
                | "adventure_combat_active"
                | "adventure_not_initialized"),
        } => HttpServiceError::UnprocessableEntity { code },
        app_application::campaigns::CampaignError::Persistence { .. } => {
            HttpServiceError::Internal {
                code: "campaign_operation_failed",
            }
        }
    }
}

fn rest_error(error: app_application::ports::repositories::RepositoryError) -> HttpServiceError {
    use app_application::ports::repositories::RepositoryError;
    match error {
        RepositoryError::NotFound => HttpServiceError::NotFound,
        RepositoryError::Conflict { .. } | RepositoryError::RevisionConflict { .. } => {
            HttpServiceError::Conflict {
                code: "campaign_revision_conflict",
            }
        }
        RepositoryError::Operation {
            code: "rest_causation_mismatch",
            ..
        } => HttpServiceError::Conflict {
            code: "rest_causation_mismatch",
        },
        RepositoryError::Operation {
            code:
                code @ ("rest_intention_invalid"
                | "rest_character_required"
                | "rest_character_invalid"
                | "rest_resources_invalid"
                | "rest_identity_mismatch"
                | "rest_unsafe"
                | "rest_character_downed"
                | "rest_too_soon"),
            ..
        } => HttpServiceError::UnprocessableEntity { code },
        _ => HttpServiceError::Internal {
            code: "rest_operation_failed",
        },
    }
}

fn campaign_state_response(
    state: app_application::models::campaign_state::CampaignSessionState,
) -> Result<CampaignStateResponse, HttpServiceError> {
    Ok(CampaignStateResponse {
        schema_version: 1,
        ruleset_id: "srd-5.2.1".into(),
        campaign: campaign_response(state.campaign)?,
        session_id: state.session_id.0,
        character_id: state.character_id,
        projection: state.projection,
    })
}

fn campaign_response(detail: CampaignDetail) -> Result<CampaignResponse, HttpServiceError> {
    Ok(CampaignResponse {
        id: detail.id.0,
        schema_version: detail.schema_version,
        ruleset_id: detail.ruleset_id,
        title: detail.title,
        premise: detail.premise,
        seed: detail.seed,
        last_session_id: detail.last_session_id.0,
        revision: detail.revision.0,
        created_at: campaign_timestamp(detail.created_at_epoch_ms)?,
        updated_at: campaign_timestamp(detail.updated_at_epoch_ms)?,
    })
}

fn campaign_summary_response(
    summary: CampaignSummary,
) -> Result<CampaignSummaryResponse, HttpServiceError> {
    Ok(CampaignSummaryResponse {
        id: summary.id.0,
        schema_version: summary.schema_version,
        ruleset_id: summary.ruleset_id,
        title: summary.title,
        premise: summary.premise,
        last_session_id: summary.last_session_id.0,
        revision: summary.revision.0,
        updated_at: campaign_timestamp(summary.updated_at_epoch_ms)?,
    })
}

fn campaign_timestamp(epoch_ms: i64) -> Result<String, HttpServiceError> {
    DateTime::<Utc>::from_timestamp_millis(epoch_ms)
        .map(|timestamp| timestamp.to_rfc3339_opts(SecondsFormat::Millis, true))
        .ok_or(HttpServiceError::Internal {
            code: "campaign_timestamp_invalid",
        })
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
#[serde(deny_unknown_fields)]
struct WeaponActionArgs {
    attacker_id: Uuid,
    target_id: Uuid,
    #[serde(default)]
    weapon_id: Option<String>,
    #[serde(default)]
    mode: app_domain::combat::loadout::WeaponMode,
    #[serde(default)]
    two_handed: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SpellActionArgs {
    caster_id: Uuid,
    spell_id: String,
    target_ids: Vec<Uuid>,
    slot_level: u8,
}

#[derive(Debug, Deserialize)]
struct MoveActionArgs {
    combatant_id: Uuid,
    x: i32,
    y: i32,
}

#[async_trait]
impl CombatHttpService for AppStateCombatHttpService {
    async fn start(
        &self,
        command: CombatStartCommand,
    ) -> Result<CombatProjection, HttpServiceError> {
        SqliteStore::new(self.state.db().clone())
            .require_session_campaign(command.session_id, command.campaign_id)
            .await
            .map_err(|_| HttpServiceError::BadRequest {
                code: "combat_campaign_mismatch",
            })?;
        let _lane = self
            .state
            .campaign_lane(command.campaign_id)
            .lock_owned()
            .await;
        let encounter_id = Uuid::new_v4();
        let projection = projection_from_start(encounter_id, &command.initiative_entries);
        let repository = adapter_sqlite::SqliteStore::new(self.state.db().clone());
        repository
            .create(command.session_id, projection)
            .await
            .map_err(|_| HttpServiceError::Internal {
                code: "combat_start_failed",
            })
    }

    async fn action(
        &self,
        command: HttpCombatActionCommand,
    ) -> Result<Option<CombatProjection>, HttpServiceError> {
        let action = match command.action_type.as_str() {
            "spell" => {
                let args: SpellActionArgs = serde_json::from_value(command.args).map_err(|_| {
                    HttpServiceError::BadRequest {
                        code: "combat_spell_intention_invalid",
                    }
                })?;
                CombatActionCommand::Spell {
                    caster: CombatantId(args.caster_id),
                    spell_id: args.spell_id,
                    targets: args.target_ids.into_iter().map(CombatantId).collect(),
                    slot_level: args.slot_level,
                }
            }
            "weapon_attack" => {
                let args: WeaponActionArgs =
                    serde_json::from_value(command.args).map_err(|_| {
                        HttpServiceError::BadRequest {
                            code: "combat_weapon_intention_invalid",
                        }
                    })?;
                CombatActionCommand::WeaponAttack {
                    attacker: CombatantId(args.attacker_id),
                    target: CombatantId(args.target_id),
                    weapon_id: args.weapon_id,
                    mode: args.mode,
                    two_handed: args.two_handed,
                }
            }
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
                // Production rolls are generated by the host. The legacy
                // transport seed is not an authority for game outcomes.
                rng_seed: Uuid::new_v4().as_u128() as u64,
                action,
            })
            .await
            .map_err(combat_error)?;
        Ok(Some(result.projection))
    }

    async fn end(&self, encounter_id: Uuid) -> Result<Option<CombatProjection>, HttpServiceError> {
        let repository = adapter_sqlite::SqliteStore::new(self.state.db().clone());
        repository
            .end(encounter_id)
            .await
            .map_err(|_| HttpServiceError::Internal {
                code: "combat_end_failed",
            })
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
        campaign_scope: None,
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
        ResolveCombatError::RequestMismatch => HttpServiceError::Conflict {
            code: "combat_request_mismatch",
        },
        ResolveCombatError::InvalidState => HttpServiceError::BadRequest {
            code: "combat_state_invalid",
        },
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

#[async_trait]
impl SavesHttpService for AppStateSavesHttpService {
    async fn list(&self, session_id: Uuid) -> Result<Value, HttpServiceError> {
        let store = SqliteStore::new(self.state.db().clone());
        let saves = store
            .list_snapshots(SessionId(session_id))
            .await
            .map_err(snapshot_repository_error)?;
        serde_json::to_value(saves).map_err(|_| serialization_error())
    }

    async fn create(
        &self,
        session_id: Uuid,
        metadata: SaveMetadataCommand,
    ) -> Result<SaveCreated, HttpServiceError> {
        let _lane = self
            .state
            .campaign_lane(metadata.campaign_id)
            .lock_owned()
            .await;
        let save_id = Uuid::new_v4();
        let repository = Arc::new(SqliteStore::new(self.state.db().clone()));
        let snapshot = CreateSnapshot::new(repository)
            .execute(CreateSnapshotCommand {
                id: save_id,
                campaign_id: CampaignId(metadata.campaign_id),
                session_id: SessionId(session_id),
                expected_revision: Revision(metadata.expected_revision),
                title: metadata.title,
                kind: metadata.kind,
                summary: metadata.summary,
                tag: metadata.tag,
                causation_id: metadata.causation_id,
                character: metadata.character,
            })
            .await
            .map_err(campaign_use_case_error)?;
        Ok(SaveCreated {
            id: snapshot.save_id,
            revision: snapshot.snapshot.revision.0,
        })
    }

    async fn quick(&self, session_id: Uuid) -> Result<SaveCreated, HttpServiceError> {
        let row = sqlx::query("SELECT campaign_id, revision FROM sessions WHERE id = ?1")
            .bind(session_id.to_string())
            .fetch_optional(self.state.db())
            .await
            .map_err(|_| HttpServiceError::Internal {
                code: "save_session_failed",
            })?
            .ok_or(HttpServiceError::NotFound)?;
        use sqlx::Row as _;
        let campaign_id = row
            .try_get::<String, _>("campaign_id")
            .ok()
            .and_then(|value| Uuid::parse_str(&value).ok())
            .ok_or(HttpServiceError::Internal {
                code: "save_session_invalid",
            })?;
        let expected_revision = row.try_get::<i64, _>("revision").unwrap_or_default() as u64;
        self.create(
            session_id,
            SaveMetadataCommand {
                campaign_id,
                expected_revision,
                causation_id: format!("quick-save-{}", Uuid::new_v4()),
                character: json!({}),
                kind: "auto".into(),
                title: "Quick save".into(),
                summary: "".into(),
                tag: "exploration".into(),
            },
        )
        .await
    }

    async fn get(&self, save_id: Uuid) -> Result<Value, HttpServiceError> {
        let save = SqliteStore::new(self.state.db().clone())
            .get_snapshot(save_id)
            .await
            .map_err(snapshot_repository_error)?
            .ok_or(HttpServiceError::NotFound)?;
        serde_json::to_value(save).map_err(|_| serialization_error())
    }

    async fn delete(&self, save_id: Uuid) -> Result<bool, HttpServiceError> {
        SqliteStore::new(self.state.db().clone())
            .delete_snapshot(save_id)
            .await
            .map_err(snapshot_repository_error)
    }

    async fn restore(&self, command: RestoreSaveCommand) -> Result<Value, HttpServiceError> {
        let _lane = self
            .state
            .campaign_lane(command.campaign_id)
            .lock_owned()
            .await;
        let repository = Arc::new(SqliteStore::new(self.state.db().clone()));
        let snapshot = RestoreSnapshot::new(repository.clone())
            .execute(RestoreSnapshotCommand {
                save_id: command.save_id,
                campaign_id: CampaignId(command.campaign_id),
                session_id: SessionId(command.session_id),
                expected_revision: Revision(command.expected_revision),
                causation_id: command.causation_id,
            })
            .await
            .map_err(campaign_use_case_error)?;
        let current = app_application::campaigns::LoadCampaignSession::new(repository.clone())
            .execute(CampaignId(command.campaign_id))
            .await;
        let mut response = serde_json::to_value(snapshot).map_err(|_| serialization_error())?;
        match current {
            Ok(current) => {
                response["current_state"] = serde_json::to_value(campaign_state_response(current)?)
                    .map_err(|_| serialization_error())?;
            }
            Err(app_application::campaigns::CampaignError::Persistence {
                code: "campaign_not_found",
            }) => {
                // Legacy saves predate the campaign-created seed event. Do not
                // invent a seed or report failure after a committed restoration.
                let session = app_application::ports::repositories::SessionRepository::get(
                    repository.as_ref(),
                    SessionId(command.session_id),
                )
                .await
                .map_err(snapshot_repository_error)?
                .ok_or(HttpServiceError::NotFound)?;
                response["current_revision"] = json!(session.revision.0);
            }
            Err(error) => return Err(campaign_use_case_error(error)),
        }
        Ok(response)
    }

    async fn update(
        &self,
        save_id: Uuid,
        metadata: SaveMetadataCommand,
    ) -> Result<bool, HttpServiceError> {
        SqliteStore::new(self.state.db().clone())
            .update_snapshot_metadata(
                save_id,
                metadata.kind,
                metadata.title,
                metadata.summary,
                metadata.tag,
            )
            .await
            .map_err(snapshot_repository_error)
    }
}

fn snapshot_repository_error(
    error: app_application::ports::repositories::RepositoryError,
) -> HttpServiceError {
    match error {
        app_application::ports::repositories::RepositoryError::NotFound => {
            HttpServiceError::NotFound
        }
        app_application::ports::repositories::RepositoryError::RevisionConflict { .. }
        | app_application::ports::repositories::RepositoryError::Conflict { .. } => {
            HttpServiceError::Conflict {
                code: "snapshot_revision_conflict",
            }
        }
        app_application::ports::repositories::RepositoryError::Operation { .. } => {
            HttpServiceError::Internal {
                code: "snapshot_repository_failed",
            }
        }
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
        if let Some(model) = &command.model {
            config.model = model.clone();
        }
        let retriever = self.state.srd_retriever();
        let image_provider = self.state.image_provider();
        let video_provider = self.state.video_provider();
        // Process ownership lives in Tauri. The backend cannot directly stop or
        // restart the model child, so no legacy in-process GPU swap is built.
        let gpu_swap = None;
        let pool = self.state.db().clone();
        let orchestrator = crate::agent::orchestrator::AgentOrchestrator::new(
            provider,
            pool.clone(),
            config,
            retriever,
            image_provider,
        )
        .with_gpu_swap(gpu_swap)
        .with_video_provider(video_provider);
        let driver = Arc::new(DurableOrchestratorDriver {
            orchestrator,
            lane: self.state.campaign_lane(command.request.campaign_id),
        });
        let service = app_application::agent::operations::DurableAgentTurns::new(
            Arc::new(SqliteStore::new(pool)),
            driver,
            self.state.process_id(),
        );
        service
            .execute_mode(
                app_application::agent::operations::AgentOperation {
                    request_id: command.request_id.unwrap_or_else(Uuid::new_v4),
                    request: command.request,
                    model: command.model,
                },
                command.resume,
            )
            .await
            .map_err(|error| match error {
                app_application::ports::repositories::RepositoryError::Operation {
                    code:
                        code @ ("agent_request_mismatch"
                        | "agent_turn_running"
                        | "agent_turn_interrupted"
                        | "agent_turn_superseded"
                        | "agent_turn_recovery_unavailable"),
                    ..
                } => HttpServiceError::Conflict { code },
                _ => HttpServiceError::Internal {
                    code: "agent_operation_failed",
                },
            })
    }
}

struct DurableOrchestratorDriver {
    orchestrator: crate::agent::orchestrator::AgentOrchestrator,
    lane: Arc<tokio::sync::Mutex<()>>,
}
#[async_trait]
impl app_application::agent::operations::AgentOperationDriver for DurableOrchestratorDriver {
    async fn acquire(&self) -> Box<dyn Send> {
        Box::new(self.lane.clone().lock_owned().await)
    }
    async fn resume(
        &self,
        input: app_application::agent::operations::AgentOperation,
        messages: Vec<app_application::models::chat::ChatMessage>,
        events: mpsc::Sender<AgentEvent>,
        cancellation: Arc<app_application::agent::operations::TurnCancellation>,
    ) -> Result<(), &'static str> {
        self.orchestrator
            .run_recovered(
                input.request,
                events,
                input.request_id.to_string(),
                cancellation,
                Some(messages),
            )
            .await
            .map_err(|_| "agent_turn_failed")
    }
    async fn run(
        &self,
        input: app_application::agent::operations::AgentOperation,
        events: mpsc::Sender<AgentEvent>,
        cancellation: Arc<app_application::agent::operations::TurnCancellation>,
    ) -> Result<(), &'static str> {
        self.orchestrator
            .run_with_id(
                input.request,
                events,
                input.request_id.to_string(),
                cancellation,
            )
            .await
            .map_err(|_| "agent_turn_failed")
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
