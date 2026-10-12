use crate::models::media_asset::{
    MediaAssetError, MediaDescriptor, MediaReadScope, OwnedMediaAsset,
};
use async_trait::async_trait;
use uuid::Uuid;

/// Format inspection and content hashing belong to the outbound media adapter.
pub trait MediaContentInspector: Send + Sync {
    fn inspect(&self, bytes: &[u8], mime_type: &str) -> Result<MediaDescriptor, MediaAssetError>;
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
