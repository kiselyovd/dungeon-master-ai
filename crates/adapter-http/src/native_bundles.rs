//! Privileged backup/import stays outside the network room router.
use crate::services::{HttpServiceError, NativeCampaignBundleHttpService};
use app_application::models::{
    campaign_bundle::BundleKind,
    identity::CampaignId,
    portable_campaign::{CampaignImportResult, MAX_BUNDLE_FILE_BYTES},
};
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Path},
    http::header,
    routing::post,
    Extension, Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportRequest {
    kind: BundleKind,
}
pub fn router(
    service: Arc<dyn NativeCampaignBundleHttpService>,
    token: String,
) -> Result<Router, HttpServiceError> {
    crate::room_owner::protect(
        Router::new()
            .route(
                "/native/campaigns/{campaign_id}/bundle/export",
                post(export),
            )
            .route("/native/campaigns/bundle/import", post(import))
            .route("/native/campaigns/bundle/preview", post(preview))
            .layer(DefaultBodyLimit::max(MAX_BUNDLE_FILE_BYTES))
            .layer(Extension(service)),
        token,
    )
}
async fn export(
    Extension(service): Extension<Arc<dyn NativeCampaignBundleHttpService>>,
    Path(id): Path<Uuid>,
    Json(request): Json<ExportRequest>,
) -> Result<([(header::HeaderName, &'static str); 1], Vec<u8>), HttpServiceError> {
    if id.is_nil() {
        return Err(HttpServiceError::BadRequest {
            code: "bundle_identity_invalid",
        });
    }
    service
        .export(CampaignId(id), request.kind)
        .await
        .map(|bytes| ([(header::CONTENT_TYPE, "application/octet-stream")], bytes))
}
async fn import(
    Extension(service): Extension<Arc<dyn NativeCampaignBundleHttpService>>,
    headers: axum::http::HeaderMap,
    bytes: Bytes,
) -> Result<Json<CampaignImportResult>, HttpServiceError> {
    let mut operations = headers.get_all("x-import-operation-id").iter();
    let operation = match operations.next() {
        Some(value) => {
            Uuid::parse_str(value.to_str().map_err(|_| HttpServiceError::BadRequest {
                code: "bundle_identity_invalid",
            })?)
            .map_err(|_| HttpServiceError::BadRequest {
                code: "bundle_identity_invalid",
            })?
        }
        None => Uuid::new_v4(),
    };
    if operation.is_nil() || operations.next().is_some() {
        return Err(HttpServiceError::BadRequest {
            code: "bundle_identity_invalid",
        });
    }
    service.import(&bytes, operation).await.map(Json)
}
async fn preview(
    Extension(service): Extension<Arc<dyn NativeCampaignBundleHttpService>>,
    bytes: Bytes,
) -> Result<Json<app_application::models::portable_campaign::CampaignBundlePreview>, HttpServiceError>
{
    service.preview(&bytes).await.map(Json)
}
