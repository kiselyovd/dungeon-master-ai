//! Portable data is staged only by one atomic new-campaign transaction.
use crate::SqliteStore;
use app_application::{
    models::{
        campaign_bundle::*, identity::*, media_asset::MediaReference, portable_campaign::*,
        snapshot::CampaignSnapshot,
    },
    ports::campaign_bundles::CampaignBundleRepository,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use sqlx::Row;
use std::collections::BTreeMap;
use uuid::Uuid;
fn storage<E>(_: E) -> BundleError {
    BundleError::Storage
}

#[async_trait]
impl CampaignBundleRepository for SqliteStore {
    async fn capture(&self, id: CampaignId) -> Result<CampaignExportRecord, BundleError> {
        let mut tx = self.pool.begin().await.map_err(storage)?;
        let state = crate::campaign_state::load_in_transaction(&mut tx, id)
            .await
            .map_err(storage)?;
        let mut references = BTreeMap::new();
        media_references(
            &serde_json::to_value(&state.projection).map_err(storage)?,
            &mut references,
        )?;
        let mut media = Vec::new();
        for (asset_id, descriptor) in references {
            let row = sqlx::query("SELECT a.reference_json,c.descriptor_json,c.payload FROM media_assets a JOIN media_content c ON c.sha256=a.sha256 WHERE a.asset_id=?1 AND ((a.owner_kind IN ('campaign','room_member') AND a.owner_id=?2) OR EXISTS(SELECT 1 FROM media_bindings b WHERE b.asset_id=a.asset_id AND b.campaign_id=?2))")
                .bind(asset_id.to_string()).bind(id.0.to_string()).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or(BundleError::MissingMedia)?;
            let reference: MediaReference =
                serde_json::from_str(&row.get::<String, _>("reference_json")).map_err(storage)?;
            let MediaReference::Owned {
                descriptor: actual, ..
            } = reference
            else {
                return Err(BundleError::MediaInvalid);
            };
            if actual != descriptor {
                return Err(BundleError::IntegrityMismatch);
            }
            let stored = serde_json::from_str::<
                app_application::models::media_asset::MediaDescriptor,
            >(&row.get::<String, _>("descriptor_json"))
            .map_err(storage)?;
            if stored != descriptor {
                return Err(BundleError::IntegrityMismatch);
            }
            media.push(BundleMediaEntry {
                asset_id,
                name: BundleMediaEntry::opaque_name(asset_id, &descriptor.mime_type)
                    .ok_or(BundleError::MediaInvalid)?,
                descriptor,
                bytes: row.get("payload"),
            });
        }
        validate_media_structure(&media)?;
        tx.commit().await.map_err(storage)?;
        Ok(CampaignExportRecord {
            campaign: state.campaign,
            projection: state.projection,
            media,
        })
    }
    async fn import_new(
        &self,
        verified: VerifiedCampaignImport,
    ) -> Result<CampaignImportResult, BundleError> {
        let source = verified.backup();
        let mut mapping = BTreeMap::new();
        let campaign_id = CampaignId(new_id(&mut mapping, source.campaign.id.0)?);
        let session_id = SessionId(new_id(&mut mapping, source.campaign.last_session_id.0)?);
        let mut projection = source.projection.clone();
        let mut structured = serde_json::to_value(&projection).map_err(storage)?;
        // Prose is never searched/replaced. Only identity-bearing structured
        // fields and the explicitly keyed appearance map are remapped.
        collect_ids(&structured, &mut mapping)?;
        for entry in verified.media() {
            new_id(&mut mapping, entry.asset_id)?;
        }
        rewrite_ids(&mut structured, &mapping);
        projection = serde_json::from_value(structured).map_err(storage)?;
        if !projection.is_complete() {
            return Err(BundleError::PayloadInvalid);
        }
        let now = chrono::Utc::now().timestamp_millis().to_string();
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(storage)?;
        if let Some((digest,result))=sqlx::query_as::<_,(String,String)>("SELECT integrity_sha256,result_json FROM campaign_bundle_imports WHERE operation_id=?1").bind(verified.operation_id().to_string()).fetch_optional(&mut *tx).await.map_err(storage)?{
            if digest!=verified.integrity_sha256(){return Err(BundleError::ImportConflict);}
            let result=serde_json::from_str(&result).map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            return Ok(result);
        }
        sqlx::query("INSERT INTO campaigns(id,schema_version,ruleset_id,title,premise,created_at,updated_at,name,language,last_played) VALUES(?1,1,?2,?3,?4,?5,?5,?3,'en',?5)")
            .bind(campaign_id.0.to_string()).bind(&source.campaign.ruleset_id).bind(&source.campaign.title).bind(&source.campaign.premise).bind(&now).execute(&mut *tx).await.map_err(storage)?;
        sqlx::query("INSERT INTO sessions(id,campaign_id,revision,created_at,updated_at,number,started_at) VALUES(?1,?2,0,?3,?3,1,?3)")
            .bind(session_id.0.to_string()).bind(campaign_id.0.to_string()).bind(&now).execute(&mut *tx).await.map_err(storage)?;
        sqlx::query("INSERT INTO campaign_events(campaign_id,revision,event_id,kind,payload_json,causation_id,created_at) VALUES(?1,1,?2,'campaign_imported',?3,?4,?5)")
            .bind(campaign_id.0.to_string()).bind(Uuid::new_v4().to_string()).bind(json!({"seed":source.campaign.seed}).to_string()).bind(format!("import:{}",Uuid::new_v4())).bind(&now).execute(&mut *tx).await.map_err(storage)?;
        for entry in verified.media() {
            let id = mapping[&entry.asset_id];
            let reference = find_reference(&projection, id).unwrap_or(MediaReference::Owned {
                asset_id: id,
                category: app_application::models::media_asset::MediaCategory::Illustration,
                origin: app_application::models::media_asset::MediaOrigin::Uploaded,
                descriptor: entry.descriptor.clone(),
            });
            sqlx::query("INSERT INTO media_content(sha256,descriptor_json,payload) VALUES(?1,?2,?3) ON CONFLICT(sha256) DO NOTHING")
                .bind(&entry.descriptor.sha256).bind(serde_json::to_string(&entry.descriptor).map_err(storage)?).bind(&entry.bytes).execute(&mut *tx).await.map_err(storage)?;
            let existing: (String, Vec<u8>) =
                sqlx::query_as("SELECT descriptor_json,payload FROM media_content WHERE sha256=?1")
                    .bind(&entry.descriptor.sha256)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(storage)?;
            if serde_json::from_str::<app_application::models::media_asset::MediaDescriptor>(
                &existing.0,
            )
            .map_err(storage)?
                != entry.descriptor
                || existing.1 != entry.bytes
            {
                return Err(BundleError::IntegrityMismatch);
            }
            sqlx::query("INSERT INTO media_assets(asset_id,sha256,reference_json,owner_kind,owner_id,actor_id) VALUES(?1,?2,?3,'campaign',?4,NULL)")
                .bind(id.to_string()).bind(&entry.descriptor.sha256).bind(serde_json::to_string(&reference).map_err(storage)?).bind(campaign_id.0.to_string()).execute(&mut *tx).await.map_err(storage)?;
        }
        crate::campaign_state::persist_snapshot_character(
            &mut tx,
            campaign_id,
            &projection.character,
            true,
        )
        .await
        .map_err(storage)?;
        bind_imported_portrait(&mut tx, campaign_id.0, &projection.character).await?;
        if let Some(party) = &projection.party {
            sqlx::query("INSERT INTO campaign_parties(campaign_id,host_member_id) VALUES(?1,?2)")
                .bind(campaign_id.0.to_string())
                .bind(party.host_member_id.to_string())
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            for member in &party.members {
                bind_imported_portrait(&mut tx, campaign_id.0, &member.character).await?;
                sqlx::query("INSERT INTO party_characters(campaign_id,member_id,character_id,character_json) VALUES(?1,?2,?3,?4)")
                    .bind(campaign_id.0.to_string()).bind(member.member_id.to_string()).bind(member.character_id.to_string()).bind(member.character.to_string()).execute(&mut *tx).await.map_err(storage)?;
            }
        }
        let snapshot = CampaignSnapshot {
            id: Uuid::new_v4(),
            campaign_id,
            session_id,
            revision: Revision(1),
            schema_version: 1,
            ruleset_id: source.campaign.ruleset_id.clone(),
            projection,
        };
        crate::saves::apply_projection(&mut tx, &snapshot)
            .await
            .map_err(storage)?;
        // apply_projection binds restored combat to the next session revision.
        // Publish that same revision before any canonical read or transaction commit.
        sqlx::query("UPDATE sessions SET revision=1 WHERE id=?1 AND revision=0")
            .bind(session_id.0.to_string())
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
        // Restoring combat may advance its timeline revision. Capture the exact
        // resulting canonical state, without issuing another save/revision.
        let imported = crate::campaign_state::load_in_transaction(&mut tx, campaign_id)
            .await
            .map_err(storage)?;
        sqlx::query("INSERT INTO campaign_snapshots(id,campaign_id,session_id,revision,schema_version,ruleset_id,projection_json,created_at) VALUES(?1,?2,?3,1,1,?4,?5,?6)")
            .bind(snapshot.id.to_string()).bind(campaign_id.0.to_string()).bind(session_id.0.to_string()).bind(&snapshot.ruleset_id).bind(serde_json::to_string(&imported.projection).map_err(storage)?).bind(&now).execute(&mut *tx).await.map_err(storage)?;
        let result = CampaignImportResult {
            campaign_id,
            session_id,
            revision: Revision(1),
            remapped_ids: mapping,
        };
        sqlx::query("INSERT INTO campaign_bundle_imports(operation_id,integrity_sha256,campaign_id,result_json) VALUES(?1,?2,?3,?4)").bind(verified.operation_id().to_string()).bind(verified.integrity_sha256()).bind(campaign_id.0.to_string()).bind(serde_json::to_string(&result).map_err(storage)?).execute(&mut *tx).await.map_err(storage)?;
        tx.commit().await.map_err(storage)?;
        tracing::info!(campaign_id=%campaign_id.0, "campaign bundle imported");
        Ok(result)
    }
}
fn new_id(map: &mut BTreeMap<Uuid, Uuid>, old: Uuid) -> Result<Uuid, BundleError> {
    if old.is_nil() {
        return Err(BundleError::IdentityInvalid);
    }
    Ok(*map.entry(old).or_insert_with(Uuid::new_v4))
}
async fn bind_imported_portrait(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    campaign: Uuid,
    sheet: &Value,
) -> Result<(), BundleError> {
    if let Some(MediaReference::Owned { asset_id, .. }) =
        crate::media_assets::portrait_reference(sheet)
    {
        let actor = sheet
            .get("characterId")
            .and_then(Value::as_str)
            .and_then(|id| Uuid::parse_str(id).ok())
            .ok_or(BundleError::IdentityInvalid)?;
        sqlx::query("INSERT INTO media_bindings(asset_id,campaign_id,actor_id) VALUES(?1,?2,?3) ON CONFLICT DO NOTHING").bind(asset_id.to_string()).bind(campaign.to_string()).bind(actor.to_string()).execute(&mut **tx).await.map_err(storage)?;
    }
    Ok(())
}
fn identity_key(key: &str) -> bool {
    [
        "id",
        "campaign_id",
        "campaignId",
        "session_id",
        "sessionId",
        "character_id",
        "characterId",
        "member_id",
        "host_member_id",
        "actor_id",
        "asset_id",
        "encounter_id",
        "current_combatant",
        "combatant",
        "character",
        "defense_encounter_id",
        "combatant_id",
        "attacker",
        "defender",
        "caster",
        "source",
        "target",
    ]
    .contains(&key)
}
fn collect_ids(value: &Value, map: &mut BTreeMap<Uuid, Uuid>) -> Result<(), BundleError> {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if identity_key(key) {
                    if let Some(id) = child.as_str().and_then(|s| Uuid::parse_str(s).ok()) {
                        new_id(map, id)?;
                    }
                }
                if key == "appearances" {
                    if let Some(entries) = child.as_object() {
                        for key in entries.keys() {
                            new_id(
                                map,
                                Uuid::parse_str(key).map_err(|_| BundleError::IdentityInvalid)?,
                            )?;
                        }
                    }
                }
                collect_ids(child, map)?;
            }
        }
        Value::Array(array) => {
            for child in array {
                collect_ids(child, map)?;
            }
        }
        _ => {}
    }
    Ok(())
}
fn rewrite_ids(value: &mut Value, map: &BTreeMap<Uuid, Uuid>) {
    match value {
        Value::Object(object) => {
            for (key, child) in object.iter_mut() {
                if ["deaths", "ability_scores_checked"].contains(&key.as_str()) {
                    rewrite_id_array(child, map);
                }
                if identity_key(key) {
                    if let Some(new) = child
                        .as_str()
                        .and_then(|s| Uuid::parse_str(s).ok())
                        .and_then(|id| map.get(&id))
                    {
                        *child = json!(new);
                    }
                }
                if key == "appearances" {
                    if let Some(entries) = child.as_object_mut() {
                        let old = std::mem::take(entries);
                        for (key, value) in old {
                            let new = Uuid::parse_str(&key)
                                .ok()
                                .and_then(|id| map.get(&id))
                                .map(ToString::to_string)
                                .unwrap_or(key);
                            entries.insert(new, value);
                        }
                    }
                }
                rewrite_ids(child, map);
            }
        }
        Value::Array(array) => {
            for child in array {
                rewrite_ids(child, map);
            }
        }
        _ => {}
    }
}
fn rewrite_id_array(value: &mut Value, map: &BTreeMap<Uuid, Uuid>) {
    match value {
        Value::String(text) => {
            if let Some(new) = Uuid::parse_str(text).ok().and_then(|id| map.get(&id)) {
                *text = new.to_string();
            }
        }
        Value::Array(array) => {
            for child in array {
                rewrite_id_array(child, map);
            }
        }
        _ => {}
    }
}
fn find_reference(
    projection: &app_application::models::snapshot::SnapshotProjection,
    id: Uuid,
) -> Option<MediaReference> {
    fn find(value: &Value, id: Uuid) -> Option<MediaReference> {
        if let Ok(reference @ MediaReference::Owned { asset_id, .. }) =
            serde_json::from_value::<MediaReference>(value.clone())
        {
            if asset_id == id {
                return Some(reference);
            }
        }
        match value {
            Value::Object(object) => object.values().find_map(|v| find(v, id)),
            Value::Array(array) => array.iter().find_map(|v| find(v, id)),
            _ => None,
        }
    }
    find(&serde_json::to_value(projection).ok()?, id)
}
