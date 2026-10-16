use std::sync::Arc;

use crate::models::identity::CampaignId;
use crate::models::session::CampaignDetail;
use crate::ports::repositories::CampaignRepository;

use super::CampaignError;

pub struct GetCampaign {
    repository: Arc<dyn CampaignRepository>,
}

impl GetCampaign {
    pub fn new(repository: Arc<dyn CampaignRepository>) -> Self {
        Self { repository }
    }

    #[tracing::instrument(skip_all, fields(campaign_id = %campaign_id.0))]
    pub async fn execute(
        &self,
        campaign_id: CampaignId,
    ) -> Result<Option<CampaignDetail>, CampaignError> {
        self.repository
            .get(campaign_id)
            .await
            .map_err(|_| CampaignError::Persistence { code: "get_failed" })
    }
}
