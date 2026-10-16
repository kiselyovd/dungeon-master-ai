use crate::models::{campaign_bundle::BundleError, identity::CampaignId, portable_campaign::*};
use async_trait::async_trait;
#[async_trait]
pub trait CampaignBundleRepository: Send + Sync {
    /// An export read transaction must not advance revisions or write a save.
    async fn capture(&self, id: CampaignId) -> Result<CampaignExportRecord, BundleError>;
    /// Create new identifiers and all related rows within one write transaction.
    async fn import_new(
        &self,
        verified: VerifiedCampaignImport,
    ) -> Result<CampaignImportResult, BundleError>;
}
#[async_trait]
pub trait PlayerDossierExporter: Send + Sync {
    /// Actor comes from the current authenticated room member, never request JSON.
    async fn export_own(
        &self,
        campaign: CampaignId,
        actor: uuid::Uuid,
        timestamp: i64,
    ) -> Result<Vec<u8>, BundleError>;
}
