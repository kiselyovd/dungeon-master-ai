use crate::services::{HttpServiceError, HttpServices};
use app_application::models::media_asset::*;
use axum::{
    extract::{Extension, Path, Query},
    http::{header, HeaderValue},
    response::{IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MAX_UPLOAD_BODY_BYTES: usize = MAX_MEDIA_ASSET_BYTES.div_ceil(3) * 4 + 16 * 1024;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LocalMediaScope {
    Draft {
        draft_id: Uuid,
    },
    Campaign {
        campaign_id: Uuid,
        actor_id: Option<Uuid>,
    },
}
impl LocalMediaScope {
    pub fn owner(&self) -> Result<MediaOwner, HttpServiceError> {
        let invalid = || HttpServiceError::BadRequest {
            code: "media_asset_invalid",
        };
        match self {
            Self::Draft { draft_id } if !draft_id.is_nil() => Ok(MediaOwner::Draft(*draft_id)),
            Self::Campaign {
                campaign_id,
                actor_id,
            } if !campaign_id.is_nil() && !actor_id.is_some_and(|id| id.is_nil()) => {
                Ok(MediaOwner::Campaign {
                    campaign_id: *campaign_id,
                    actor_id: *actor_id,
                })
            }
            _ => Err(invalid()),
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadMediaRequest {
    pub scope: LocalMediaScope,
    pub category: MediaCategory,
    pub mime_type: String,
    pub data_b64: String,
    #[serde(default)]
    pub migrated: bool,
}
pub async fn upload_asset(
    Extension(services): Extension<HttpServices>,
    Json(body): Json<UploadMediaRequest>,
) -> Result<Json<MediaReference>, HttpServiceError> {
    let owner = body.scope.owner()?;
    if body.data_b64.len() > MAX_MEDIA_ASSET_BYTES.div_ceil(3) * 4 {
        return Err(HttpServiceError::PayloadTooLarge {
            code: "media_asset_too_large",
        });
    };
    let bytes = STANDARD
        .decode(body.data_b64)
        .map_err(|_| HttpServiceError::BadRequest {
            code: "media_asset_invalid",
        })?;
    if bytes.len() > MAX_MEDIA_ASSET_BYTES {
        return Err(HttpServiceError::PayloadTooLarge {
            code: "media_asset_too_large",
        });
    };
    if body.category == MediaCategory::Portrait
        && matches!(owner, MediaOwner::Campaign { actor_id: None, .. })
    {
        return Err(HttpServiceError::BadRequest {
            code: "media_asset_invalid",
        });
    };
    services
        .media
        .register_asset(
            owner,
            body.category,
            if body.migrated {
                MediaOrigin::Migrated
            } else {
                MediaOrigin::Uploaded
            },
            &body.mime_type,
            bytes,
        )
        .await
        .map(Json)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetQuery {
    pub draft_id: Option<Uuid>,
    pub campaign_id: Option<Uuid>,
}
pub async fn read_asset(
    Extension(services): Extension<HttpServices>,
    Path(id): Path<Uuid>,
    Query(query): Query<AssetQuery>,
) -> Result<Response, HttpServiceError> {
    let scope = match (query.draft_id, query.campaign_id) {
        (Some(id), None) if !id.is_nil() => MediaReadScope::Draft(id),
        (None, Some(id)) if !id.is_nil() => MediaReadScope::Campaign(id),
        _ => {
            return Err(HttpServiceError::BadRequest {
                code: "media_asset_invalid",
            })
        }
    };
    let asset = services.media.read_asset(id, scope).await?;
    asset_response(asset)
}
pub fn asset_response(asset: OwnedMediaAsset) -> Result<Response, HttpServiceError> {
    let MediaReference::Owned { descriptor, .. } = asset.reference else {
        return Err(HttpServiceError::Internal {
            code: "media_asset_corrupt",
        });
    };
    let mime =
        HeaderValue::from_str(&descriptor.mime_type).map_err(|_| HttpServiceError::Internal {
            code: "media_asset_corrupt",
        })?;
    Ok((
        [
            (header::CONTENT_TYPE, mime),
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static("private, no-store"),
            ),
            (
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            ),
        ],
        asset.bytes,
    )
        .into_response())
}
pub fn media_asset_error(error: MediaAssetError) -> HttpServiceError {
    match error {
        MediaAssetError::Invalid => HttpServiceError::BadRequest {
            code: "media_asset_invalid",
        },
        MediaAssetError::TooLarge => HttpServiceError::PayloadTooLarge {
            code: "media_asset_too_large",
        },
        MediaAssetError::Missing => HttpServiceError::NotFound,
        MediaAssetError::Denied => HttpServiceError::Unauthorized {
            code: "media_asset_denied",
        },
        MediaAssetError::Corrupt => HttpServiceError::Internal {
            code: "media_asset_corrupt",
        },
        MediaAssetError::Storage => HttpServiceError::Internal {
            code: "media_asset_storage_unavailable",
        },
    }
}
