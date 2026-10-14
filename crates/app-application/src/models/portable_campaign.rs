//! A portable payload is a typed snapshot, never a database/settings dump.
use super::{
    campaign_bundle::*, campaign_state::valid_character_document, identity::*,
    media_asset::MediaReference, session::CampaignDetail, snapshot::SnapshotProjection,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub const MAX_BUNDLE_FILE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_BUNDLE_METADATA_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_BUNDLE_MESSAGES: usize = 20_000;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrivateCampaignBackup {
    pub campaign: CampaignDetail,
    /// Messages use the portable typed history below; these two fields must be
    /// empty in this projection until media is verified and history reconstructed.
    pub projection: SnapshotProjection,
    pub history: Vec<PortableMessage>,
    pub shared_message_indices: Vec<usize>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PortableMessagePart {
    Text {
        text: String,
    },
    Image {
        asset_id: Uuid,
        mime_type: String,
        name: Option<String>,
    },
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "snake_case", deny_unknown_fields)]
pub enum PortableMessage {
    User { parts: Vec<PortableMessagePart> },
    Assistant { content: String },
}
/// A dossier contains an owned sheet and published prose/discoveries. It has no
/// campaign seed, private party documents, hidden adventure/combat or room keys.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerDossier {
    pub actor_id: Uuid,
    pub character: Value,
    pub messages: Vec<PortableMessage>,
    pub discovered_facts: Vec<super::adventure::Fact>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "payload",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum PortablePayload {
    FullCampaignBackup(Box<PrivateCampaignBackup>),
    PlayerDossier(PlayerDossier),
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortableCampaignBundle {
    pub integrity_sha256: String,
    pub header: BundleHeader,
    pub payload: PortablePayload,
    pub media: Vec<BundleMediaEntry>,
}
/// Captured consistently inside the repository's read transaction.
pub struct CampaignExportRecord {
    pub campaign: CampaignDetail,
    pub projection: SnapshotProjection,
    pub media: Vec<BundleMediaEntry>,
}
/// Produced only after payload, reference, hash and full-decode validation.
pub struct VerifiedCampaignImport {
    pub(crate) operation_id: Uuid,
    pub(crate) integrity_sha256: String,
    pub(crate) backup: PrivateCampaignBackup,
    pub(crate) media: VerifiedBundleMedia,
}
impl VerifiedCampaignImport {
    pub fn operation_id(&self) -> Uuid {
        self.operation_id
    }
    pub fn integrity_sha256(&self) -> &str {
        &self.integrity_sha256
    }
    pub fn backup(&self) -> &PrivateCampaignBackup {
        &self.backup
    }
    pub fn media(&self) -> &[BundleMediaEntry] {
        self.media.entries()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignImportResult {
    pub campaign_id: CampaignId,
    pub session_id: SessionId,
    pub revision: Revision,
    pub remapped_ids: BTreeMap<Uuid, Uuid>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignBundlePreview {
    pub kind: BundleKind,
    pub title: String,
    pub character_count: usize,
    pub message_count: usize,
    pub media_count: usize,
    pub private_backup: bool,
}
impl PortableCampaignBundle {
    fn digest(&self) -> Result<String, BundleError> {
        let media: Vec<_> = self
            .media
            .iter()
            .map(|entry| (&entry.asset_id, &entry.name, &entry.descriptor))
            .collect();
        let bytes = serde_json::to_vec(&(&self.header, &self.payload, media))
            .map_err(|_| BundleError::PayloadInvalid)?;
        Ok(Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect())
    }
    pub fn seal(&mut self) -> Result<(), BundleError> {
        self.integrity_sha256 = self.digest()?;
        Ok(())
    }
    pub fn encode(&self) -> Result<Vec<u8>, BundleError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|_| BundleError::PayloadInvalid)?;
        if bytes.len() > MAX_BUNDLE_FILE_BYTES {
            return Err(BundleError::LimitExceeded);
        }
        Ok(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, BundleError> {
        if bytes.is_empty() || bytes.len() > MAX_BUNDLE_FILE_BYTES {
            return Err(BundleError::LimitExceeded);
        }
        let bundle: Self =
            serde_json::from_slice(bytes).map_err(|_| BundleError::PayloadInvalid)?;
        bundle.validate()?;
        Ok(bundle)
    }
    pub fn validate(&self) -> Result<(), BundleError> {
        self.header.validate()?;
        validate_media_structure(&self.media)?;
        if serde_json::to_vec(&self.payload)
            .map_err(|_| BundleError::PayloadInvalid)?
            .len()
            > MAX_BUNDLE_METADATA_BYTES
        {
            return Err(BundleError::LimitExceeded);
        }
        match &self.payload {
            PortablePayload::FullCampaignBackup(backup) => {
                if backup.campaign.id.0 != self.header.source_campaign_id
                    || backup.campaign.id.0.is_nil()
                    || backup.campaign.last_session_id.0.is_nil()
                    || backup.campaign.schema_version != 1
                    || backup.campaign.ruleset_id != self.header.ruleset_id
                    || !backup.projection.is_complete()
                    || (!valid_character_document(&backup.projection.character)
                        && backup.projection.character != serde_json::json!({}))
                    || !backup.projection.messages.is_empty()
                    || !backup.projection.shared_message_indices.is_empty()
                    || backup.history.len() > MAX_BUNDLE_MESSAGES
                    || backup
                        .shared_message_indices
                        .iter()
                        .any(|index| *index >= backup.history.len())
                    || backup
                        .shared_message_indices
                        .iter()
                        .collect::<BTreeSet<_>>()
                        .len()
                        != backup.shared_message_indices.len()
                {
                    return Err(BundleError::PayloadInvalid);
                }
                if let Some(value) = &backup.projection.combat {
                    let combat: super::combat::CombatProjection =
                        serde_json::from_value(value.clone())
                            .map_err(|_| BundleError::PayloadInvalid)?;
                    if combat.campaign_scope.as_ref().is_some_and(|scope| {
                        scope.campaign_id != backup.campaign.id.0
                            || scope.session_id != backup.campaign.last_session_id.0
                    }) {
                        return Err(BundleError::IdentityInvalid);
                    }
                }
                for rows in [&backup.projection.journal, &backup.projection.npcs] {
                    let mut ids = BTreeSet::new();
                    for row in rows {
                        let id = row
                            .get("id")
                            .and_then(Value::as_str)
                            .and_then(|id| Uuid::parse_str(id).ok())
                            .filter(|id| !id.is_nil())
                            .ok_or(BundleError::IdentityInvalid)?;
                        if !ids.insert(id) {
                            return Err(BundleError::IdentityInvalid);
                        }
                    }
                }
            }
            PortablePayload::PlayerDossier(dossier) => {
                if dossier.actor_id.is_nil()
                    || !valid_character_document(&dossier.character)
                    || dossier.character.get("characterId").and_then(Value::as_str)
                        != Some(&dossier.actor_id.to_string())
                    || dossier.messages.len() > MAX_BUNDLE_MESSAGES
                {
                    return Err(BundleError::PayloadInvalid);
                }
            }
        }
        let value = serde_json::to_value(&self.payload).map_err(|_| BundleError::PayloadInvalid)?;
        reject_infrastructure(&value)?;
        let mut references = BTreeMap::new();
        media_references(&value, &mut references)?;
        let history = match &self.payload {
            PortablePayload::FullCampaignBackup(backup) => &backup.history,
            PortablePayload::PlayerDossier(dossier) => &dossier.messages,
        };
        for message in history {
            if let PortableMessage::User { parts } = message {
                for part in parts {
                    if let PortableMessagePart::Image {
                        asset_id,
                        mime_type,
                        ..
                    } = part
                    {
                        let entry = self
                            .media
                            .iter()
                            .find(|entry| entry.asset_id == *asset_id)
                            .ok_or(BundleError::MissingMedia)?;
                        if entry.descriptor.mime_type != *mime_type {
                            return Err(BundleError::IntegrityMismatch);
                        }
                        if references
                            .insert(*asset_id, entry.descriptor.clone())
                            .is_some_and(|previous| previous != entry.descriptor)
                        {
                            return Err(BundleError::IntegrityMismatch);
                        }
                    }
                }
            }
        }
        let supplied: BTreeSet<_> = self.media.iter().map(|entry| entry.asset_id).collect();
        let needed: BTreeSet<_> = references.keys().copied().collect();
        if supplied != needed {
            return Err(BundleError::MissingMedia);
        }
        for entry in &self.media {
            if references[&entry.asset_id] != entry.descriptor {
                return Err(BundleError::IntegrityMismatch);
            }
        }
        if self.integrity_sha256.len() != 64 || self.integrity_sha256 != self.digest()? {
            return Err(BundleError::IntegrityMismatch);
        }
        Ok(())
    }
}
/// Import/export never treats arbitrary URLs or infrastructure keys as media.
pub fn reject_infrastructure(value: &Value) -> Result<(), BundleError> {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if [
                    "apiKey",
                    "api_key",
                    "provider_token",
                    "access_token",
                    "capability",
                    "resume_proof",
                    "vault",
                    "outbox",
                    "settings",
                    "reasoning",
                    "discarded_draft",
                ]
                .contains(&key.as_str())
                {
                    return Err(BundleError::PayloadInvalid);
                }
                if ["portraitUrl", "imageUrl", "videoUrl"].contains(&key.as_str())
                    && !child.is_null()
                    && child.as_str() != Some("")
                {
                    return Err(BundleError::MissingMedia);
                }
                reject_infrastructure(child)?;
            }
        }
        Value::Array(array) => {
            for child in array {
                reject_infrastructure(child)?;
            }
        }
        _ => {}
    }
    Ok(())
}
pub fn media_references(
    value: &Value,
    found: &mut BTreeMap<Uuid, super::media_asset::MediaDescriptor>,
) -> Result<(), BundleError> {
    match value {
        Value::Object(object) => {
            if object.get("source").and_then(Value::as_str) == Some("owned") {
                let reference: MediaReference =
                    serde_json::from_value(value.clone()).map_err(|_| BundleError::MediaInvalid)?;
                if !reference.valid() {
                    return Err(BundleError::MediaInvalid);
                }
                if let MediaReference::Owned {
                    asset_id,
                    descriptor,
                    ..
                } = reference
                {
                    if found
                        .insert(asset_id, descriptor.clone())
                        .is_some_and(|previous| previous != descriptor)
                    {
                        return Err(BundleError::IntegrityMismatch);
                    }
                }
            } else {
                for child in object.values() {
                    media_references(child, found)?;
                }
            }
        }
        Value::Array(array) => {
            for child in array {
                media_references(child, found)?;
            }
        }
        _ => {}
    }
    Ok(())
}
