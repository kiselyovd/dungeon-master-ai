use crate::models::media_asset::{
    MediaAssetError, MediaDescriptor, MediaReadScope, OwnedMediaAsset,
};
use async_trait::async_trait;
use uuid::Uuid;

/// Format inspection and content hashing belong to the outbound media adapter.
pub trait MediaContentInspector: Send + Sync {
    fn verify_bundled_asset(
        &self,
        _asset_id: &str,
        _version: Option<u16>,
    ) -> Result<(), MediaAssetError> {
        Err(MediaAssetError::Missing)
    }
    fn inspect(&self, bytes: &[u8], mime_type: &str) -> Result<MediaDescriptor, MediaAssetError>;
    /// Full pixel decode for untrusted portable imports. Header/container-only
    /// inspectors fail closed instead of claiming that their metadata is enough.
    fn verify_decoded(
        &self,
        _bytes: &[u8],
        _descriptor: &MediaDescriptor,
    ) -> Result<(), MediaAssetError> {
        Err(MediaAssetError::Invalid)
    }
    fn decode_inline(
        &self,
        _url: &str,
    ) -> Result<crate::models::media_asset::InlineMediaData, MediaAssetError> {
        Err(MediaAssetError::Invalid)
    }
}

#[async_trait]
pub trait MediaAssetRepository: Send + Sync {
    async fn insert(&self, asset: OwnedMediaAsset) -> Result<(), MediaAssetError>;
    async fn read(
        &self,
        asset_id: Uuid,
        scope: &MediaReadScope,
    ) -> Result<OwnedMediaAsset, MediaAssetError>;
}
