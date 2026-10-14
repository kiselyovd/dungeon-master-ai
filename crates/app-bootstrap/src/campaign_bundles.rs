//! Composition/HTTP translation only; export/import policy remains inward.
use adapter_http::services::{HttpServiceError, NativeCampaignBundleHttpService};
use app_application::{
    campaign_bundles::CampaignBundles,
    models::{campaign_bundle::*, identity::CampaignId, portable_campaign::CampaignImportResult},
    ports::repositories::CampaignStateRepository,
};
use std::sync::Arc;
pub struct NativeBundles {
    store: Arc<adapter_sqlite::SqliteStore>,
    use_case: CampaignBundles<
        adapter_sqlite::SqliteStore,
        adapter_media::bundle_codec::DecodedRasterInspector,
    >,
}
impl NativeBundles {
    pub fn new(store: Arc<adapter_sqlite::SqliteStore>) -> Self {
        Self {
            use_case: CampaignBundles::new(
                store.clone(),
                Arc::new(adapter_media::bundle_codec::DecodedRasterInspector),
            ),
            store,
        }
    }
}
#[async_trait::async_trait]
impl NativeCampaignBundleHttpService for NativeBundles {
    async fn preview(
        &self,
        bytes: &[u8],
    ) -> Result<app_application::models::portable_campaign::CampaignBundlePreview, HttpServiceError>
    {
        self.use_case.preview(bytes).map_err(map_error)
    }
    async fn export(
        &self,
        campaign: CampaignId,
        kind: BundleKind,
    ) -> Result<Vec<u8>, HttpServiceError> {
        // This service is only composed behind the native owner's private token.
        let current = self
            .store
            .load_current(campaign)
            .await
            .map_err(|_| HttpServiceError::NotFound)?;
        let actor = if kind == BundleKind::PlayerDossier {
            current.character_id
        } else {
            None
        };
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|time| i64::try_from(time.as_millis()).ok())
            .unwrap_or(0);
        self.use_case
            .export(
                campaign,
                ExportPrincipal::SoloStorageOwner {
                    own_actor: current.character_id,
                },
                kind,
                actor,
                timestamp,
            )
            .await
            .map_err(map_error)?
            .encode()
            .map_err(map_error)
    }
    async fn import(
        &self,
        bytes: &[u8],
        operation_id: uuid::Uuid,
    ) -> Result<CampaignImportResult, HttpServiceError> {
        self.use_case
            .import_with_operation(bytes, operation_id)
            .await
            .map_err(map_error)
    }
}
pub fn map_error(error: BundleError) -> HttpServiceError {
    let code = match error {
        BundleError::ExportForbidden => {
            return HttpServiceError::Unauthorized {
                code: "bundle_export_forbidden",
            }
        }
        BundleError::LimitExceeded => {
            return HttpServiceError::PayloadTooLarge {
                code: "bundle_limit_exceeded",
            }
        }
        BundleError::Storage => {
            return HttpServiceError::Internal {
                code: "bundle_storage_failed",
            }
        }
        BundleError::ImportConflict => {
            return HttpServiceError::Conflict {
                code: "bundle_import_operation_conflict",
            }
        }
        BundleError::NotRestorable => "bundle_not_restorable",
        BundleError::SchemaUnsupported => "bundle_schema_unsupported",
        BundleError::RulesetMismatch => "bundle_ruleset_mismatch",
        BundleError::IdentityInvalid => "bundle_identity_invalid",
        BundleError::MediaInvalid => "bundle_media_invalid",
        BundleError::IntegrityMismatch => "bundle_media_integrity_mismatch",
        BundleError::PayloadInvalid => "bundle_payload_invalid",
        BundleError::MissingMedia => "bundle_required_media_missing",
    };
    HttpServiceError::UnprocessableEntity { code }
}
