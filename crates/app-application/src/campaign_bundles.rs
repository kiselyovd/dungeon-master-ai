use crate::models::campaign_bundle::{
    validate_media_structure, BundleError, BundleHeader, BundleMediaEntry, VerifiedBundleMedia,
};
use crate::models::{identity::CampaignId, portable_campaign::*};
use crate::ports::campaign_bundles::CampaignBundleRepository;
use crate::ports::media_assets::MediaContentInspector;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

/// Verify actual media before any repository staging/import. The existing
/// outward inspector owns hashing and bounded format/dimension inspection.
pub struct VerifyBundleMedia<I>(Arc<I>);
impl<I: MediaContentInspector> VerifyBundleMedia<I> {
    pub fn new(inspector: Arc<I>) -> Self {
        Self(inspector)
    }
    pub fn execute(
        &self,
        header: &BundleHeader,
        entries: Vec<BundleMediaEntry>,
    ) -> Result<VerifiedBundleMedia, BundleError> {
        header.validate()?;
        validate_media_structure(&entries)?;
        for entry in &entries {
            let inspected = self
                .0
                .inspect(&entry.bytes, &entry.descriptor.mime_type)
                .map_err(|_| BundleError::MediaInvalid)?;
            if !inspected.valid() || inspected != entry.descriptor {
                return Err(BundleError::IntegrityMismatch);
            }
            self.0
                .verify_decoded(&entry.bytes, &entry.descriptor)
                .map_err(|_| BundleError::MediaInvalid)?;
        }
        Ok(VerifiedBundleMedia { entries })
    }
}

pub struct CampaignBundles<R, I> {
    repository: Arc<R>,
    inspector: Arc<I>,
}
#[async_trait::async_trait]
impl<R: CampaignBundleRepository, I: MediaContentInspector>
    crate::ports::campaign_bundles::PlayerDossierExporter for CampaignBundles<R, I>
{
    async fn export_own(
        &self,
        campaign: CampaignId,
        actor: Uuid,
        timestamp: i64,
    ) -> Result<Vec<u8>, BundleError> {
        self.export(
            campaign,
            crate::models::campaign_bundle::ExportPrincipal::Player { actor_id: actor },
            crate::models::campaign_bundle::BundleKind::PlayerDossier,
            Some(actor),
            timestamp,
        )
        .await?
        .encode()
    }
}
impl<R: CampaignBundleRepository, I: MediaContentInspector> CampaignBundles<R, I> {
    pub fn new(repository: Arc<R>, inspector: Arc<I>) -> Self {
        Self {
            repository,
            inspector,
        }
    }
    pub fn preview(&self, bytes: &[u8]) -> Result<CampaignBundlePreview, BundleError> {
        let bundle = PortableCampaignBundle::decode(bytes)?;
        verify_bundled(
            &serde_json::to_value(&bundle.payload).map_err(|_| BundleError::PayloadInvalid)?,
            self.inspector.as_ref(),
        )?;
        VerifyBundleMedia::new(self.inspector.clone())
            .execute(&bundle.header, bundle.media.clone())?;
        let (kind, title, character_count, message_count, private_backup) = match &bundle.payload {
            PortablePayload::FullCampaignBackup(backup) => (
                crate::models::campaign_bundle::BundleKind::FullCampaignBackup,
                backup.campaign.title.chars().take(200).collect(),
                usize::from(backup.projection.character.get("characterId").is_some())
                    + backup
                        .projection
                        .party
                        .as_ref()
                        .map_or(0, |party| party.members.len()),
                backup.history.len(),
                true,
            ),
            PortablePayload::PlayerDossier(dossier) => (
                crate::models::campaign_bundle::BundleKind::PlayerDossier,
                dossier
                    .character
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .chars()
                    .take(200)
                    .collect(),
                1,
                dossier.messages.len(),
                false,
            ),
        };
        Ok(CampaignBundlePreview {
            kind,
            title,
            character_count,
            message_count,
            media_count: bundle.media.len(),
            private_backup,
        })
    }
    pub async fn export(
        &self,
        campaign_id: CampaignId,
        principal: crate::models::campaign_bundle::ExportPrincipal,
        kind: crate::models::campaign_bundle::BundleKind,
        actor: Option<Uuid>,
        timestamp: i64,
    ) -> Result<PortableCampaignBundle, BundleError> {
        use crate::models::campaign_bundle::*;
        let permission = authorize_export(principal, kind, actor)?;
        let mut record = self.repository.capture(campaign_id).await?;
        if record.campaign.id != campaign_id {
            return Err(BundleError::IdentityInvalid);
        }
        if kind == BundleKind::PlayerDossier {
            if let ExportPrincipal::SoloStorageOwner { own_actor }
            | ExportPrincipal::HostStorageOwner { own_actor } = principal
            {
                let current = record
                    .projection
                    .character
                    .get("characterId")
                    .and_then(Value::as_str)
                    .and_then(|id| Uuid::parse_str(id).ok());
                if current != own_actor {
                    return Err(BundleError::ExportForbidden);
                }
            }
        }
        let mut media = record.media;
        let payload = match permission {
            AuthorizedExport::FullCampaignBackup => {
                normalize_character(
                    &mut record.projection.character,
                    &mut media,
                    self.inspector.as_ref(),
                )?;
                if let Some(party) = &mut record.projection.party {
                    for member in &mut party.members {
                        normalize_character(
                            &mut member.character,
                            &mut media,
                            self.inspector.as_ref(),
                        )?;
                    }
                }
                let (history, shared) = portable_history(
                    &record.projection.messages,
                    &record.projection.shared_message_indices,
                    &mut media,
                    self.inspector.as_ref(),
                    false,
                )?;
                record.projection.messages.clear();
                record.projection.shared_message_indices.clear();
                PortablePayload::FullCampaignBackup(Box::new(PrivateCampaignBackup {
                    campaign: record.campaign,
                    projection: record.projection,
                    history,
                    shared_message_indices: shared,
                }))
            }
            AuthorizedExport::PlayerDossier { actor_id } => {
                let own = record
                    .projection
                    .character
                    .get("characterId")
                    .and_then(Value::as_str)
                    == Some(&actor_id.to_string());
                let mut character = if own {
                    record.projection.character.clone()
                } else {
                    record
                        .projection
                        .party
                        .as_ref()
                        .and_then(|party| {
                            party
                                .members
                                .iter()
                                .find(|member| member.character_id == actor_id)
                        })
                        .map(|member| member.character.clone())
                        .ok_or(BundleError::ExportForbidden)?
                };
                normalize_character(&mut character, &mut media, self.inspector.as_ref())?;
                let (messages, _) = portable_history(
                    &record.projection.messages,
                    &record.projection.shared_message_indices,
                    &mut media,
                    self.inspector.as_ref(),
                    true,
                )?;
                let discovered_facts = record
                    .projection
                    .adventure
                    .as_ref()
                    .map(|projection| projection.state.facts.iter().copied().collect())
                    .unwrap_or_default();
                PortablePayload::PlayerDossier(PlayerDossier {
                    actor_id,
                    character,
                    messages,
                    discovered_facts,
                })
            }
        };
        let mut wanted = std::collections::BTreeMap::new();
        let value = serde_json::to_value(&payload).map_err(|_| BundleError::PayloadInvalid)?;
        media_references(&value, &mut wanted)?;
        let history = match &payload {
            PortablePayload::FullCampaignBackup(backup) => &backup.history,
            PortablePayload::PlayerDossier(dossier) => &dossier.messages,
        };
        for message in history {
            if let PortableMessage::User { parts } = message {
                for part in parts {
                    if let PortableMessagePart::Image { asset_id, .. } = part {
                        let entry = media
                            .iter()
                            .find(|entry| entry.asset_id == *asset_id)
                            .ok_or(BundleError::MissingMedia)?;
                        wanted.insert(*asset_id, entry.descriptor.clone());
                    }
                }
            }
        }
        media.retain(|entry| wanted.contains_key(&entry.asset_id));
        let mut bundle = PortableCampaignBundle {
            integrity_sha256: String::new(),
            header: BundleHeader {
                schema_version: BUNDLE_SCHEMA_VERSION,
                ruleset_id: record_ruleset(),
                source_campaign_id: campaign_id.0,
                exported_at_epoch_ms: timestamp,
            },
            payload,
            media,
        };
        bundle.seal()?;
        bundle.validate()?;
        verify_bundled(
            &serde_json::to_value(&bundle.payload).map_err(|_| BundleError::PayloadInvalid)?,
            self.inspector.as_ref(),
        )?;
        VerifyBundleMedia::new(self.inspector.clone())
            .execute(&bundle.header, bundle.media.clone())?;
        Ok(bundle)
    }
    pub async fn import_new(&self, bytes: &[u8]) -> Result<CampaignImportResult, BundleError> {
        self.import_with_operation(bytes, Uuid::new_v4()).await
    }
    pub async fn import_with_operation(
        &self,
        bytes: &[u8],
        operation_id: Uuid,
    ) -> Result<CampaignImportResult, BundleError> {
        if operation_id.is_nil() {
            return Err(BundleError::IdentityInvalid);
        }
        let bundle = PortableCampaignBundle::decode(bytes)?;
        verify_bundled(
            &serde_json::to_value(&bundle.payload).map_err(|_| BundleError::PayloadInvalid)?,
            self.inspector.as_ref(),
        )?;
        let PortablePayload::FullCampaignBackup(mut backup) = bundle.payload else {
            return Err(BundleError::NotRestorable);
        };
        let media =
            VerifyBundleMedia::new(self.inspector.clone()).execute(&bundle.header, bundle.media)?;
        backup.projection.messages = restore_history(&backup.history, media.entries())?;
        backup.projection.shared_message_indices = backup.shared_message_indices.clone();
        if !backup.projection.is_complete() {
            return Err(BundleError::PayloadInvalid);
        }
        self.repository
            .import_new(VerifiedCampaignImport {
                operation_id,
                integrity_sha256: bundle.integrity_sha256,
                backup: *backup,
                media,
            })
            .await
    }
}
fn record_ruleset() -> String {
    app_domain::ruleset::RULESET_ID.to_owned()
}
fn verify_bundled<I: MediaContentInspector>(
    value: &Value,
    inspector: &I,
) -> Result<(), BundleError> {
    match value {
        Value::Object(object) => {
            if object.get("source").and_then(Value::as_str) == Some("bundled") {
                let reference: crate::models::media_asset::MediaReference =
                    serde_json::from_value(value.clone()).map_err(|_| BundleError::MediaInvalid)?;
                if let crate::models::media_asset::MediaReference::Bundled { asset_id, version } =
                    reference
                {
                    inspector
                        .verify_bundled_asset(&asset_id, Some(version))
                        .map_err(|_| BundleError::MissingMedia)?;
                }
            }
            for (key, child) in object {
                if ["portraitAssetId", "scene_asset_id", "map_skin_asset_id"]
                    .contains(&key.as_str())
                {
                    if let Some(id) = child.as_str().filter(|id| !id.is_empty()) {
                        inspector
                            .verify_bundled_asset(id, None)
                            .map_err(|_| BundleError::MissingMedia)?;
                    }
                }
                verify_bundled(child, inspector)?;
            }
        }
        Value::Array(array) => {
            for child in array {
                verify_bundled(child, inspector)?;
            }
        }
        _ => {}
    }
    Ok(())
}
fn strip_infrastructure(value: &mut Value) {
    match value {
        Value::Object(object) => {
            object.retain(|key, _| {
                ![
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
            });
            for child in object.values_mut() {
                strip_infrastructure(child);
            }
        }
        Value::Array(array) => {
            for child in array {
                strip_infrastructure(child);
            }
        }
        _ => {}
    }
}
fn add_inline<I: MediaContentInspector>(
    mime: &str,
    encoded: &str,
    media: &mut Vec<BundleMediaEntry>,
    inspector: &I,
) -> Result<Uuid, BundleError> {
    use crate::models::media_asset::MAX_MEDIA_ASSET_BYTES;
    if encoded.len() > MAX_MEDIA_ASSET_BYTES.div_ceil(3) * 4 {
        return Err(BundleError::LimitExceeded);
    }
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|_| BundleError::MediaInvalid)?;
    let descriptor = inspector
        .inspect(&bytes, mime)
        .map_err(|_| BundleError::MediaInvalid)?;
    inspector
        .verify_decoded(&bytes, &descriptor)
        .map_err(|_| BundleError::MediaInvalid)?;
    if let Some(existing) = media.iter().find(|entry| entry.descriptor == descriptor) {
        return Ok(existing.asset_id);
    }
    let asset_id = Uuid::new_v4();
    media.push(BundleMediaEntry {
        asset_id,
        name: BundleMediaEntry::opaque_name(asset_id, mime).ok_or(BundleError::MediaInvalid)?,
        descriptor,
        bytes,
    });
    validate_media_structure(media)?;
    Ok(asset_id)
}
fn normalize_character<I: MediaContentInspector>(
    character: &mut Value,
    media: &mut Vec<BundleMediaEntry>,
    inspector: &I,
) -> Result<(), BundleError> {
    strip_infrastructure(character);
    let url = character
        .get("portraitUrl")
        .and_then(Value::as_str)
        .filter(|url| !url.is_empty())
        .map(str::to_owned);
    if let Some(url) = url {
        if character.get("portraitMediaRef").is_none_or(Value::is_null) {
            let (mime, encoded) = url
                .strip_prefix("data:")
                .and_then(|value| value.split_once(";base64,"))
                .ok_or(BundleError::MissingMedia)?;
            let id = add_inline(mime, encoded, media, inspector)?;
            let entry = media
                .iter()
                .find(|entry| entry.asset_id == id)
                .ok_or(BundleError::MissingMedia)?;
            character["portraitMediaRef"] =
                serde_json::to_value(crate::models::media_asset::MediaReference::Owned {
                    asset_id: id,
                    category: crate::models::media_asset::MediaCategory::Portrait,
                    origin: crate::models::media_asset::MediaOrigin::Migrated,
                    descriptor: entry.descriptor.clone(),
                })
                .map_err(|_| BundleError::PayloadInvalid)?;
        }
        character
            .as_object_mut()
            .ok_or(BundleError::PayloadInvalid)?
            .remove("portraitUrl");
    }
    Ok(())
}
fn portable_history<I: MediaContentInspector>(
    messages: &[crate::models::chat::ChatMessage],
    shared: &[usize],
    media: &mut Vec<BundleMediaEntry>,
    inspector: &I,
    shared_only: bool,
) -> Result<(Vec<PortableMessage>, Vec<usize>), BundleError> {
    use crate::models::chat::{ChatMessage, MessagePart};
    let mut history = Vec::new();
    let mut remapped = Vec::new();
    for (index, message) in messages.iter().enumerate() {
        if shared_only && !shared.contains(&index) {
            continue;
        }
        let portable = match message {
            ChatMessage::User { parts } => {
                let mut output = Vec::new();
                for part in parts {
                    output.push(match part {
                        MessagePart::Text { text } => {
                            PortableMessagePart::Text { text: text.clone() }
                        }
                        MessagePart::Image {
                            mime,
                            data_b64,
                            name,
                        } => PortableMessagePart::Image {
                            asset_id: add_inline(mime, data_b64, media, inspector)?,
                            mime_type: mime.clone(),
                            name: name.clone(),
                        },
                    });
                }
                PortableMessage::User { parts: output }
            }
            ChatMessage::Assistant { content } => PortableMessage::Assistant {
                content: content.clone(),
            },
            _ => continue,
        };
        if shared.contains(&index) {
            remapped.push(history.len());
        }
        history.push(portable);
    }
    Ok((history, remapped))
}
fn restore_history(
    history: &[PortableMessage],
    media: &[BundleMediaEntry],
) -> Result<Vec<crate::models::chat::ChatMessage>, BundleError> {
    use crate::models::chat::{ChatMessage, MessagePart};
    history
        .iter()
        .map(|message| match message {
            PortableMessage::Assistant { content } => Ok(ChatMessage::Assistant {
                content: content.clone(),
            }),
            PortableMessage::User { parts } => Ok(ChatMessage::User {
                parts: parts
                    .iter()
                    .map(|part| match part {
                        PortableMessagePart::Text { text } => {
                            Ok(MessagePart::Text { text: text.clone() })
                        }
                        PortableMessagePart::Image {
                            asset_id,
                            mime_type,
                            name,
                        } => {
                            let entry = media
                                .iter()
                                .find(|entry| entry.asset_id == *asset_id)
                                .ok_or(BundleError::MissingMedia)?;
                            Ok(MessagePart::Image {
                                mime: mime_type.clone(),
                                data_b64: STANDARD.encode(&entry.bytes),
                                name: name.clone(),
                            })
                        }
                    })
                    .collect::<Result<_, BundleError>>()?,
            }),
        })
        .collect()
}
