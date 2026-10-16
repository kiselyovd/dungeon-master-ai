use app_application::campaigns::{CreateCampaignCommand, GAME_SCHEMA_VERSION, RULESET_ID};
use app_application::models::campaign_state::CommitCampaignCharacter;
use app_application::models::identity::{CampaignId, Revision, SessionId};
use app_application::models::session::CampaignSeed;
use app_application::models::snapshot::SnapshotProjection;
use axum::extract::{Extension, Path};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::services::{HttpServiceError, HttpServices};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortraitMigrationRequest {
    pub character_id: Uuid,
    pub expected_revision: Revision,
    pub request_id: Uuid,
}
#[derive(Serialize)]
pub struct PortraitMigrationResponse {
    pub state: CampaignStateResponse,
    pub changed: bool,
    pub retained: bool,
}
pub async fn migrate_portrait(
    Extension(services): Extension<HttpServices>,
    Path(campaign): Path<Uuid>,
    Json(body): Json<PortraitMigrationRequest>,
) -> Result<Json<PortraitMigrationResponse>, HttpServiceError> {
    services
        .campaign
        .migrate_portrait(
            app_application::media_assets::migrate_portrait::MigratePortrait {
                campaign_id: CampaignId(campaign),
                character_id: body.character_id,
                expected_revision: body.expected_revision,
                request_id: body.request_id,
            },
        )
        .await
        .map(Json)
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateCampaignRequest {
    #[serde(default)]
    pub request_id: Option<Uuid>,
    pub schema_version: u16,
    pub ruleset_id: String,
    pub title: String,
    pub premise: String,
    pub seed: CampaignSeed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignResponse {
    pub id: Uuid,
    pub schema_version: u16,
    pub ruleset_id: String,
    pub title: String,
    pub premise: String,
    pub seed: CampaignSeed,
    pub last_session_id: Uuid,
    pub revision: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignSummaryResponse {
    pub id: Uuid,
    pub schema_version: u16,
    pub ruleset_id: String,
    pub title: String,
    pub premise: String,
    pub last_session_id: Uuid,
    pub revision: u64,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_location: Option<app_application::models::session::CampaignLocation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CampaignStateResponse {
    pub schema_version: u16,
    pub ruleset_id: String,
    pub campaign: CampaignResponse,
    pub session_id: Uuid,
    pub character_id: Option<Uuid>,
    pub projection: SnapshotProjection,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommitCharacterRequest {
    pub schema_version: u16,
    pub ruleset_id: String,
    pub session_id: Uuid,
    pub character_id: Uuid,
    pub expected_revision: u64,
    pub causation_id: String,
    pub character: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdventureActionRequest {
    pub schema_version: u16,
    pub ruleset_id: String,
    pub session_id: Uuid,
    pub character_id: Uuid,
    pub expected_revision: u64,
    pub causation_id: String,
    pub action: app_application::models::adventure::Action,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestRequest {
    pub schema_version: u16,
    pub ruleset_id: String,
    pub session_id: Uuid,
    pub character_id: Uuid,
    pub expected_revision: u64,
    pub causation_id: String,
    pub intent: app_application::models::rest::RestIntent,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoloRecoveryRequest {
    pub schema_version: u16,
    pub ruleset_id: String,
    pub session_id: Uuid,
    pub character_id: Uuid,
    pub expected_revision: u64,
    pub causation_id: String,
    pub intent: app_application::models::room_recovery::RecoveryIntent,
}
pub async fn get_recovery_status(
    Extension(services): Extension<HttpServices>,
    Path(id): Path<Uuid>,
) -> Result<Json<app_application::models::solo_recovery::SoloRecoveryStatus>, HttpServiceError> {
    services.campaign.recovery_status(id).await.map(Json)
}
pub async fn post_recovery(
    Extension(services): Extension<HttpServices>,
    Path(id): Path<Uuid>,
    Json(body): Json<SoloRecoveryRequest>,
) -> Result<Json<app_application::models::solo_recovery::SoloRecoveryResult>, HttpServiceError> {
    if body.schema_version != GAME_SCHEMA_VERSION || body.ruleset_id != RULESET_ID {
        return Err(HttpServiceError::UnprocessableEntity {
            code: "campaign_contract_unsupported",
        });
    }
    services
        .campaign
        .recover_solo(
            app_application::models::solo_recovery::SoloRecoveryCommand {
                campaign_id: CampaignId(id),
                session_id: SessionId(body.session_id),
                character_id: body.character_id,
                expected_revision: Revision(body.expected_revision),
                causation_id: body.causation_id,
                intent: body.intent,
            },
        )
        .await
        .map(Json)
}

pub async fn get_rest_status(
    Extension(services): Extension<HttpServices>,
    Path(id): Path<Uuid>,
) -> Result<Json<app_application::models::rest::RestStatus>, HttpServiceError> {
    services.campaign.rest_status(id).await.map(Json)
}
pub async fn post_rest(
    Extension(services): Extension<HttpServices>,
    Path(id): Path<Uuid>,
    Json(body): Json<RestRequest>,
) -> Result<Json<app_application::models::rest::RestResult>, HttpServiceError> {
    if body.schema_version != GAME_SCHEMA_VERSION || body.ruleset_id != RULESET_ID {
        return Err(HttpServiceError::UnprocessableEntity {
            code: "campaign_contract_unsupported",
        });
    }
    services
        .campaign
        .rest(app_application::models::rest::RestCommand {
            campaign_id: CampaignId(id),
            session_id: SessionId(body.session_id),
            character_id: body.character_id,
            expected_revision: Revision(body.expected_revision),
            causation_id: body.causation_id,
            intent: body.intent,
        })
        .await
        .map(Json)
}

pub async fn post_adventure_action(
    Extension(services): Extension<HttpServices>,
    Path(id): Path<Uuid>,
    Json(body): Json<AdventureActionRequest>,
) -> Result<Json<CampaignStateResponse>, HttpServiceError> {
    if body.schema_version != GAME_SCHEMA_VERSION || body.ruleset_id != RULESET_ID {
        return Err(HttpServiceError::UnprocessableEntity {
            code: "campaign_contract_unsupported",
        });
    }
    services
        .campaign
        .adventure_action(app_application::models::adventure::AdventureCommand {
            campaign_id: CampaignId(id),
            session_id: SessionId(body.session_id),
            character_id: body.character_id,
            expected_revision: Revision(body.expected_revision),
            causation_id: body.causation_id,
            action: body.action,
        })
        .await
        .map(Json)
}

pub async fn get_campaign_state(
    Extension(services): Extension<HttpServices>,
    Path(id): Path<Uuid>,
) -> Result<Json<CampaignStateResponse>, HttpServiceError> {
    services.campaign.state(id).await.map(Json)
}

pub async fn put_campaign_character(
    Extension(services): Extension<HttpServices>,
    Path(id): Path<Uuid>,
    Json(body): Json<CommitCharacterRequest>,
) -> Result<Json<CampaignStateResponse>, HttpServiceError> {
    if body.schema_version != GAME_SCHEMA_VERSION || body.ruleset_id != RULESET_ID {
        return Err(HttpServiceError::UnprocessableEntity {
            code: "campaign_contract_unsupported",
        });
    }
    services
        .campaign
        .commit_character(CommitCampaignCharacter {
            campaign_id: CampaignId(id),
            session_id: SessionId(body.session_id),
            character_id: body.character_id,
            expected_revision: Revision(body.expected_revision),
            causation_id: body.causation_id,
            character: body.character,
        })
        .await
        .map(Json)
}

pub fn validate_create_campaign(
    request: CreateCampaignRequest,
) -> Result<CreateCampaignCommand, HttpServiceError> {
    if request.schema_version != GAME_SCHEMA_VERSION || request.ruleset_id != RULESET_ID {
        return Err(HttpServiceError::UnprocessableEntity {
            code: "campaign_contract_unsupported",
        });
    }
    if request.title.trim().is_empty() || !valid_seed(&request.seed) {
        return Err(HttpServiceError::UnprocessableEntity {
            code: "campaign_seed_invalid",
        });
    }
    Ok(CreateCampaignCommand {
        request_id: request.request_id,
        title: request.title,
        premise: request.premise,
        seed: request.seed,
    })
}

fn valid_seed(seed: &CampaignSeed) -> bool {
    match seed {
        CampaignSeed::Collaborative { answers } => {
            !answers.is_empty()
                && answers.iter().all(|(key, values)| {
                    !key.trim().is_empty()
                        && !values.is_empty()
                        && values.iter().all(|value| !value.trim().is_empty())
                })
        }
        CampaignSeed::Document { .. } => true,
        CampaignSeed::Bundled { adventure_id } => !adventure_id.trim().is_empty(),
    }
}

pub async fn post_campaign(
    Extension(services): Extension<HttpServices>,
    Json(request): Json<CreateCampaignRequest>,
) -> Result<Json<CampaignResponse>, HttpServiceError> {
    let command = validate_create_campaign(request)?;
    services.campaign.create_campaign(command).await.map(Json)
}

pub async fn get_campaigns(
    Extension(services): Extension<HttpServices>,
) -> Result<Json<Vec<CampaignSummaryResponse>>, HttpServiceError> {
    services.campaign.list_campaigns().await.map(Json)
}

pub async fn get_campaign(
    Extension(services): Extension<HttpServices>,
    Path(campaign_id): Path<Uuid>,
) -> Result<Json<CampaignResponse>, HttpServiceError> {
    services.campaign.get_campaign(campaign_id).await.map(Json)
}
