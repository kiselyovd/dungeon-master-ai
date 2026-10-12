use super::MediaAssets;
use crate::{
    models::{
        campaign_state::{CampaignSessionState, CommitCampaignCharacter},
        identity::{CampaignId, Revision},
        media_asset::*,
    },
    ports::repositories::{CampaignStateRepository, RepositoryError},
};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigratePortrait {
    pub campaign_id: CampaignId,
    pub character_id: Uuid,
    pub expected_revision: Revision,
    pub request_id: Uuid,
}
pub struct PortraitMigrationResult {
    pub state: CampaignSessionState,
    pub changed: bool,
    pub retained: bool,
}
pub struct MigrateCampaignPortrait {
    assets: Arc<MediaAssets>,
    campaigns: Arc<dyn CampaignStateRepository>,
}
fn error(code: &'static str) -> RepositoryError {
    RepositoryError::Operation {
        operation: "media_migration",
        code,
    }
}
impl MigrateCampaignPortrait {
    pub fn new(assets: Arc<MediaAssets>, campaigns: Arc<dyn CampaignStateRepository>) -> Self {
        Self { assets, campaigns }
    }
    pub async fn execute(
        &self,
        command: MigratePortrait,
    ) -> Result<PortraitMigrationResult, RepositoryError> {
        if command.character_id.is_nil() || command.request_id.is_nil() {
            return Err(error("media_asset_invalid"));
        };
        let state = self.campaigns.load_current(command.campaign_id).await?;
        if state.character_id != Some(command.character_id) {
            return Err(error("media_asset_denied"));
        };
        let document = &state.projection.character;
        if document
            .get("portraitMediaRef")
            .is_some_and(|v| !v.is_null())
            || document
                .get("portraitAssetId")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|id| !id.is_empty())
        {
            return Ok(PortraitMigrationResult {
                state,
                changed: false,
                retained: false,
            });
        }
        let Some(url) = document
            .get("portraitUrl")
            .and_then(serde_json::Value::as_str)
            .filter(|url| !url.is_empty())
        else {
            return Ok(PortraitMigrationResult {
                state,
                changed: false,
                retained: false,
            });
        };
        if state.campaign.revision != command.expected_revision {
            return Err(RepositoryError::Conflict {
                expected: command.expected_revision,
                actual: state.campaign.revision,
            });
        };
        let reference = match self
            .assets
            .migrate_inline_as(
                command.request_id,
                MediaOwner::Campaign {
                    campaign_id: command.campaign_id.0,
                    actor_id: Some(command.character_id),
                },
                MediaCategory::Portrait,
                url,
            )
            .await
        {
            Ok(reference) => reference,
            Err(MediaAssetError::Invalid | MediaAssetError::TooLarge) => {
                tracing::warn!(campaign_id=%command.campaign_id.0,"unresolved legacy portrait retained");
                return Ok(PortraitMigrationResult {
                    state,
                    changed: false,
                    retained: true,
                });
            }
            Err(_) => return Err(error("media_asset_storage_unavailable")),
        };
        let mut character = state.projection.character.clone();
        character["portraitMediaRef"] = serde_json::json!(reference);
        character["portraitUrl"] = serde_json::Value::Null;
        let state = self
            .campaigns
            .commit_character(CommitCampaignCharacter {
                campaign_id: command.campaign_id,
                session_id: state.session_id,
                character_id: command.character_id,
                expected_revision: command.expected_revision,
                causation_id: format!("media-migration:{}", command.request_id),
                character,
            })
            .await?;
        Ok(PortraitMigrationResult {
            state,
            changed: true,
            retained: false,
        })
    }
}
