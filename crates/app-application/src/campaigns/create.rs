use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use app_domain::ruleset::{GAME_SCHEMA_VERSION, RULESET_ID};
use serde_json::json;
use uuid::Uuid;

use crate::models::identity::{CampaignId, Revision, SessionId};
use crate::models::session::{
    CampaignDetail, CampaignEvent, CampaignEventKind, CampaignSeed, CampaignSession,
    InitialCampaign,
};
use crate::ports::repositories::CampaignRepository;

use super::CampaignError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateCampaignCommand {
    pub title: String,
    pub premise: String,
    pub seed: CampaignSeed,
}

pub struct CreateCampaign {
    repository: Arc<dyn CampaignRepository>,
}

impl CreateCampaign {
    pub fn new(repository: Arc<dyn CampaignRepository>) -> Self {
        Self { repository }
    }

    #[tracing::instrument(skip_all)]
    pub async fn execute(
        &self,
        command: CreateCampaignCommand,
    ) -> Result<CampaignDetail, CampaignError> {
        let title = command.title.trim().to_owned();
        if title.is_empty() {
            return Err(CampaignError::InvalidInput {
                code: "title_required",
            });
        }
        let premise = command.premise.trim().to_owned();
        let campaign_id = CampaignId(Uuid::new_v4());
        let session_id = SessionId(Uuid::new_v4());
        let revision = Revision(1);
        let now = epoch_ms();
        let detail = CampaignDetail {
            id: campaign_id,
            schema_version: GAME_SCHEMA_VERSION,
            ruleset_id: RULESET_ID.to_owned(),
            title,
            premise,
            seed: command.seed,
            last_session_id: session_id,
            revision,
            created_at_epoch_ms: now,
            updated_at_epoch_ms: now,
        };
        let initial = InitialCampaign {
            session: CampaignSession {
                id: session_id,
                campaign_id,
                revision,
                created_at_epoch_ms: now,
                updated_at_epoch_ms: now,
            },
            event: CampaignEvent {
                event_id: Uuid::new_v4(),
                campaign_id,
                revision,
                kind: CampaignEventKind::new("campaign_created"),
                payload: json!({
                    "title": detail.title,
                    "premise": detail.premise,
                    "seed": detail.seed,
                }),
                causation_id: None,
                created_at_epoch_ms: now,
            },
            detail,
        };
        let created =
            self.repository
                .create(initial)
                .await
                .map_err(|_| CampaignError::Persistence {
                    code: "create_failed",
                })?;
        tracing::info!(
            campaign_id = %created.id.0,
            session_id = %created.last_session_id.0,
            revision = created.revision.0,
            "campaign aggregate committed"
        );
        Ok(created)
    }
}

fn epoch_ms() -> i64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    i64::try_from(millis).unwrap_or(i64::MAX)
}
