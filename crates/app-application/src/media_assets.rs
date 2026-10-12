//! Register and resolve immutable bytes using inward-owned storage/inspection ports.
pub mod migrate_portrait;
use crate::{
    models::media_asset::*,
    ports::media_assets::{MediaAssetRepository, MediaContentInspector},
};
use std::sync::Arc;
use uuid::Uuid;

pub struct MediaAssets {
    repository: Arc<dyn MediaAssetRepository>,
    inspector: Arc<dyn MediaContentInspector>,
}
impl MediaAssets {
    pub fn new(
        repository: Arc<dyn MediaAssetRepository>,
        inspector: Arc<dyn MediaContentInspector>,
    ) -> Self {
        Self {
            repository,
            inspector,
        }
    }
    pub async fn register(
        &self,
        owner: MediaOwner,
        category: MediaCategory,
        origin: MediaOrigin,
        mime_type: &str,
        bytes: Vec<u8>,
    ) -> Result<MediaReference, MediaAssetError> {
        self.register_as(Uuid::new_v4(), owner, category, origin, mime_type, bytes)
            .await
    }
    pub async fn register_as(
        &self,
        asset_id: Uuid,
        owner: MediaOwner,
        category: MediaCategory,
        origin: MediaOrigin,
        mime_type: &str,
        bytes: Vec<u8>,
    ) -> Result<MediaReference, MediaAssetError> {
        if asset_id.is_nil() {
            return Err(MediaAssetError::Invalid);
        };
        if bytes.len() > MAX_MEDIA_ASSET_BYTES {
            return Err(MediaAssetError::TooLarge);
        }
        let descriptor = self.inspector.inspect(&bytes, mime_type)?;
        if !descriptor.valid() {
            return Err(MediaAssetError::Invalid);
        }
        let reference = MediaReference::Owned {
            asset_id,
            category,
            origin,
            descriptor,
        };
        self.repository
            .insert(OwnedMediaAsset {
                reference: reference.clone(),
                owner,
                bytes,
            })
            .await?;
        if let MediaReference::Owned {
            asset_id,
            descriptor,
            ..
        } = &reference
        {
            tracing::info!(%asset_id, byte_len=descriptor.byte_len, "durable media registered");
        }
        Ok(reference)
    }
    pub async fn read(
        &self,
        id: Uuid,
        scope: &MediaReadScope,
    ) -> Result<OwnedMediaAsset, MediaAssetError> {
        let asset = self.repository.read(id, scope).await?;
        let MediaReference::Owned {
            asset_id,
            descriptor,
            ..
        } = &asset.reference
        else {
            return Err(MediaAssetError::Corrupt);
        };
        if *asset_id != id
            || !asset.reference.valid()
            || self
                .inspector
                .inspect(&asset.bytes, &descriptor.mime_type)
                .map_err(|_| MediaAssetError::Corrupt)?
                != *descriptor
        {
            return Err(MediaAssetError::Corrupt);
        }
        Ok(asset)
    }
    pub async fn export_asset(
        &self,
        reference: &MediaReference,
        scope: &MediaReadScope,
    ) -> Result<PortableMediaAsset, MediaAssetError> {
        match reference {
            MediaReference::Bundled { .. } if reference.valid() => Ok(PortableMediaAsset {
                reference: reference.clone(),
                bytes: vec![],
            }),
            MediaReference::Owned { asset_id, .. } => {
                let asset = self.read(*asset_id, scope).await?;
                if asset.reference != *reference {
                    return Err(MediaAssetError::Corrupt);
                };
                Ok(PortableMediaAsset {
                    reference: asset.reference,
                    bytes: asset.bytes,
                })
            }
            _ => Err(MediaAssetError::Invalid),
        }
    }
    pub async fn migrate_inline_as(
        &self,
        id: Uuid,
        owner: MediaOwner,
        category: MediaCategory,
        url: &str,
    ) -> Result<MediaReference, MediaAssetError> {
        let decoded = self.inspector.decode_inline(url)?;
        self.register_as(
            id,
            owner,
            category,
            MediaOrigin::Migrated,
            &decoded.mime_type,
            decoded.bytes,
        )
        .await
    }
    pub async fn import_asset(
        &self,
        owner: MediaOwner,
        package: PortableMediaAsset,
    ) -> Result<MediaReference, MediaAssetError> {
        if !package.reference.valid() {
            return Err(MediaAssetError::Invalid);
        };
        match package.reference {
            reference @ MediaReference::Bundled { .. } if package.bytes.is_empty() => Ok(reference),
            MediaReference::Owned {
                category,
                origin,
                descriptor,
                ..
            } => {
                let actual = self
                    .inspector
                    .inspect(&package.bytes, &descriptor.mime_type)?;
                if actual != descriptor {
                    return Err(MediaAssetError::Corrupt);
                };
                self.register(
                    owner,
                    category,
                    origin,
                    &descriptor.mime_type,
                    package.bytes,
                )
                .await
            }
            _ => Err(MediaAssetError::Invalid),
        }
    }
}
