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
    pub request_id: Option<Uuid>,
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
        if matches!(&command.seed, CampaignSeed::Bundled { adventure_id } if adventure_id != app_domain::adventure::BELL_ADVENTURE_ID)
        {
            return Err(CampaignError::InvalidInput {
                code: "adventure_package_unknown",
            });
        }
        if title.is_empty() {
            return Err(CampaignError::InvalidInput {
                code: "title_required",
            });
        }
        let premise = command.premise.trim().to_owned();
        if command.request_id.is_some_and(|id| id.is_nil()) {
            return Err(CampaignError::InvalidInput {
                code: "create_request_invalid",
            });
        }
        let campaign_id = CampaignId(command.request_id.unwrap_or_else(Uuid::new_v4));
        if command.request_id.is_some() {
            if let Some(existing) =
                self.repository
                    .get(campaign_id)
                    .await
                    .map_err(|_| CampaignError::Persistence {
                        code: "create_lookup_failed",
                    })?
            {
                if existing.title == title
                    && existing.premise == premise
                    && existing.seed == command.seed
                {
                    return Ok(existing);
                }
                return Err(CampaignError::InvalidInput {
                    code: "create_request_changed",
                });
            }
        }
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
            opening_scene: if matches!(detail.seed, CampaignSeed::Collaborative { .. }) {
                Some(crate::models::campaign::SceneRecord {
                    id: Uuid::new_v4(),
                    campaign_id: campaign_id.0,
                    name: detail.title.clone(),
                    description: detail.premise.clone(),
                })
            } else {
                None
            },
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
        let expected = initial.detail.clone();
        let created = match self.repository.create(initial).await {
            Ok(created) => created,
            Err(_) => {
                // A concurrent retry or lost commit ACK can finish after our
                // initial lookup. Read the durable aggregate before failing.
                let existing = if command.request_id.is_some() {
                    self.repository.get(campaign_id).await.ok().flatten()
                } else {
                    None
                };
                match existing {
                    Some(existing)
                        if existing.title == expected.title
                            && existing.premise == expected.premise
                            && existing.seed == expected.seed =>
                    {
                        existing
                    }
                    Some(_) => {
                        return Err(CampaignError::InvalidInput {
                            code: "create_request_changed",
                        })
                    }
                    None => {
                        return Err(CampaignError::Persistence {
                            code: "create_failed",
                        })
                    }
                }
            }
        };
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
