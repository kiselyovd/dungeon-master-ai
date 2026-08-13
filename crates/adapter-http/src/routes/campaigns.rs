use app_application::campaigns::{CreateCampaignCommand, GAME_SCHEMA_VERSION, RULESET_ID};
use app_application::models::session::CampaignSeed;
use axum::extract::{Extension, Path};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::services::{HttpServiceError, HttpServices};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateCampaignRequest {
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
