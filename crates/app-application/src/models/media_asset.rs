//! Immutable media references contain metadata, never bytes or source URLs.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MAX_MEDIA_ASSET_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_MEDIA_PIXELS: u64 = 32 * 1024 * 1024;
pub const MAX_MEDIA_DIMENSION: u32 = 8192;
pub const MAX_LEGACY_MEDIA_URL_BYTES: usize = 8 * 1024 * 1024;
pub struct InlineMediaData {
    pub mime_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaCategory {
    Portrait,
    Map,
    Illustration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaOrigin {
    Generated,
    Uploaded,
    Migrated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaDescriptor {
    pub sha256: String,
    pub mime_type: String,
    pub byte_len: u64,
    pub width: u32,
    pub height: u32,
}
impl MediaDescriptor {
    pub fn valid(&self) -> bool {
        self.sha256.len() == 64
            && self
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            && matches!(
                self.mime_type.as_str(),
                "image/png" | "image/jpeg" | "image/webp"
            )
            && (1..=MAX_MEDIA_ASSET_BYTES as u64).contains(&self.byte_len)
            && (1..=MAX_MEDIA_DIMENSION).contains(&self.width)
            && (1..=MAX_MEDIA_DIMENSION).contains(&self.height)
            && u64::from(self.width) * u64::from(self.height) <= MAX_MEDIA_PIXELS
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum MediaReference {
    Bundled {
        asset_id: String,
        version: u16,
    },
    Owned {
        asset_id: Uuid,
        category: MediaCategory,
        origin: MediaOrigin,
        descriptor: MediaDescriptor,
    },
}
impl MediaReference {
    pub fn valid(&self) -> bool {
        match self {
            Self::Bundled { asset_id, version } => {
                *version > 0
                    && !asset_id.is_empty()
                    && asset_id.len() <= 128
                    && asset_id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
            }
            Self::Owned {
                asset_id,
                descriptor,
                ..
            } => !asset_id.is_nil() && descriptor.valid(),
        }
    }
}

/// Ownership comes from trusted composition/authentication, never a public DTO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaOwner {
    Draft(Uuid),
    Campaign {
        campaign_id: Uuid,
        actor_id: Option<Uuid>,
    },
    RoomMember {
        campaign_id: Uuid,
        member_id: Uuid,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedMediaAsset {
    pub reference: MediaReference,
    pub owner: MediaOwner,
    pub bytes: Vec<u8>,
}
/// Portability carries bytes once in a separate package, never in sheet/preferences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortableMediaAsset {
    pub reference: MediaReference,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum MediaReadScope {
    Draft(Uuid),
    Campaign(Uuid),
    Room(super::room::RoomLease),
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum MediaAssetError {
    #[error("media_asset_invalid")]
    Invalid,
    #[error("media_asset_too_large")]
    TooLarge,
    #[error("media_asset_missing")]
    Missing,
    #[error("media_asset_denied")]
    Denied,
    #[error("media_asset_corrupt")]
    Corrupt,
    #[error("media_asset_storage_unavailable")]
    Storage,
}
