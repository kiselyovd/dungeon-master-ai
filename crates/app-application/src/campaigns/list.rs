use std::sync::Arc;

use crate::models::session::CampaignSummary;
use crate::ports::repositories::CampaignRepository;

use super::CampaignError;

pub struct ListCampaigns {
    repository: Arc<dyn CampaignRepository>,
}

impl ListCampaigns {
    pub fn new(repository: Arc<dyn CampaignRepository>) -> Self {
        Self { repository }
    }

    #[tracing::instrument(skip_all)]
    pub async fn execute(&self) -> Result<Vec<CampaignSummary>, CampaignError> {
        self.repository
            .list()
            .await
            .map_err(|_| CampaignError::Persistence {
                code: "list_failed",
            })
    }
}
