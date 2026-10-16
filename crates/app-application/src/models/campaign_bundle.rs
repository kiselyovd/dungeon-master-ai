//! Portable wire-contract boundaries. Structural checks do not substitute for
//! media hashing/decoding or the repository's atomic import transaction.
use super::media_asset::{MediaDescriptor, MAX_MEDIA_ASSET_BYTES};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const BUNDLE_SCHEMA_VERSION: u16 = 1;
pub const MAX_BUNDLE_MEDIA_ENTRIES: usize = 64;
pub const MAX_BUNDLE_MEDIA_BYTES: u64 = 32 * 1024 * 1024;
pub const MAX_BUNDLE_DECODED_PIXELS: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BundleKind {
    FullCampaignBackup,
    PlayerDossier,
}

/// Constructed from an authenticated transport/storage context, never from
/// an export request body or a claimed character display name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportPrincipal {
    SoloStorageOwner { own_actor: Option<Uuid> },
    HostStorageOwner { own_actor: Option<Uuid> },
    Player { actor_id: Uuid },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorizedExport {
    FullCampaignBackup,
    PlayerDossier { actor_id: Uuid },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BundleError {
    #[error("bundle_export_forbidden")]
    ExportForbidden,
    #[error("bundle_not_restorable")]
    NotRestorable,
    #[error("bundle_schema_unsupported")]
    SchemaUnsupported,
    #[error("bundle_ruleset_mismatch")]
    RulesetMismatch,
    #[error("bundle_identity_invalid")]
    IdentityInvalid,
    #[error("bundle_media_invalid")]
    MediaInvalid,
    #[error("bundle_media_integrity_mismatch")]
    IntegrityMismatch,
    #[error("bundle_limit_exceeded")]
    LimitExceeded,
    #[error("bundle_payload_invalid")]
    PayloadInvalid,
    #[error("bundle_required_media_missing")]
    MissingMedia,
    #[error("bundle_storage_failed")]
    Storage,
    #[error("bundle_import_operation_conflict")]
    ImportConflict,
}
pub fn authorize_export(
    principal: ExportPrincipal,
    kind: BundleKind,
    actor: Option<Uuid>,
) -> Result<AuthorizedExport, BundleError> {
    match kind {
        BundleKind::FullCampaignBackup => match principal {
            ExportPrincipal::SoloStorageOwner { .. } | ExportPrincipal::HostStorageOwner { .. }
                if actor.is_none() =>
            {
                Ok(AuthorizedExport::FullCampaignBackup)
            }
            _ => Err(BundleError::ExportForbidden),
        },
        BundleKind::PlayerDossier => {
            let own = match principal {
                ExportPrincipal::SoloStorageOwner { own_actor }
                | ExportPrincipal::HostStorageOwner { own_actor } => own_actor,
                ExportPrincipal::Player { actor_id } => Some(actor_id),
            };
            match (own, actor) {
                (Some(own), Some(requested)) if !own.is_nil() && own == requested => {
                    Ok(AuthorizedExport::PlayerDossier { actor_id: own })
                }
                _ => Err(BundleError::ExportForbidden),
            }
        }
    }
}
pub fn require_restorable(kind: BundleKind) -> Result<(), BundleError> {
    if kind == BundleKind::FullCampaignBackup {
        Ok(())
    } else {
        Err(BundleError::NotRestorable)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleHeader {
    pub schema_version: u16,
    pub ruleset_id: String,
    pub source_campaign_id: Uuid,
    pub exported_at_epoch_ms: i64,
}
impl BundleHeader {
    pub fn validate(&self) -> Result<(), BundleError> {
        if self.schema_version != BUNDLE_SCHEMA_VERSION {
            return Err(BundleError::SchemaUnsupported);
        }
        if self.ruleset_id != app_domain::ruleset::RULESET_ID {
            return Err(BundleError::RulesetMismatch);
        }
        if self.source_campaign_id.is_nil() || self.exported_at_epoch_ms < 0 {
            return Err(BundleError::IdentityInvalid);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleMediaEntry {
    pub asset_id: Uuid,
    pub name: String,
    pub descriptor: MediaDescriptor,
    #[serde(with = "encoded_media")]
    pub bytes: Vec<u8>,
}

mod encoded_media {
    use super::MAX_MEDIA_ASSET_BYTES;
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let encoded = String::deserialize(deserializer)?;
        if encoded.len() > MAX_MEDIA_ASSET_BYTES.div_ceil(3) * 4 {
            return Err(serde::de::Error::custom("bundle_limit_exceeded"));
        }
        let bytes = STANDARD
            .decode(encoded)
            .map_err(|_| serde::de::Error::custom("bundle_media_invalid"))?;
        if bytes.len() > MAX_MEDIA_ASSET_BYTES {
            return Err(serde::de::Error::custom("bundle_limit_exceeded"));
        }
        Ok(bytes)
    }
}
impl BundleMediaEntry {
    pub fn opaque_name(id: Uuid, mime: &str) -> Option<String> {
        let extension = match mime {
            "image/png" => "png",
            "image/jpeg" => "jpg",
            "image/webp" => "webp",
            _ => return None,
        };
        if id.is_nil() {
            return None;
        }
        Some(format!("media/{}.{extension}", id.simple()))
    }
    pub fn validate_structure(&self) -> Result<(), BundleError> {
        let expected = Self::opaque_name(self.asset_id, &self.descriptor.mime_type)
            .ok_or(BundleError::MediaInvalid)?;
        if self.name != expected
            || !self.descriptor.valid()
            || self.bytes.is_empty()
            || self.bytes.len() > MAX_MEDIA_ASSET_BYTES
            || self.bytes.len() as u64 != self.descriptor.byte_len
        {
            return Err(BundleError::MediaInvalid);
        }
        Ok(())
    }
}
pub fn validate_media_structure(entries: &[BundleMediaEntry]) -> Result<(), BundleError> {
    if entries.len() > MAX_BUNDLE_MEDIA_ENTRIES {
        return Err(BundleError::LimitExceeded);
    }
    let mut ids = std::collections::HashSet::new();
    let mut bytes = 0u64;
    let mut pixels = 0u64;
    for entry in entries {
        entry.validate_structure()?;
        if !ids.insert(entry.asset_id) {
            return Err(BundleError::MediaInvalid);
        }
        bytes = bytes
            .checked_add(entry.descriptor.byte_len)
            .ok_or(BundleError::LimitExceeded)?;
        pixels = pixels
            .checked_add(u64::from(entry.descriptor.width) * u64::from(entry.descriptor.height))
            .ok_or(BundleError::LimitExceeded)?;
        if bytes > MAX_BUNDLE_MEDIA_BYTES || pixels > MAX_BUNDLE_DECODED_PIXELS {
            return Err(BundleError::LimitExceeded);
        }
    }
    Ok(())
}

/// Not deserializable: only the application verification use case constructs it.
pub struct VerifiedBundleMedia {
    pub(crate) entries: Vec<BundleMediaEntry>,
}
impl VerifiedBundleMedia {
    pub fn entries(&self) -> &[BundleMediaEntry] {
        &self.entries
    }
}
