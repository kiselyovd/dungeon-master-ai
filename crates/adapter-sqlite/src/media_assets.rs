//! Immutable, application-owned media bytes and explicit campaign bindings.
use app_application::{models::media_asset::*, ports::media_assets::MediaAssetRepository};
use async_trait::async_trait;
use serde_json::Value;
use sqlx::{Row, Sqlite, Transaction};
use uuid::Uuid;

fn storage<T>(_: T) -> MediaAssetError {
    MediaAssetError::Storage
}
fn owner_columns(owner: &MediaOwner) -> (&'static str, String, Option<String>) {
    match owner {
        MediaOwner::Draft(id) => ("draft", id.to_string(), None),
        MediaOwner::Campaign {
            campaign_id,
            actor_id,
        } => (
            "campaign",
            campaign_id.to_string(),
            actor_id.map(|id| id.to_string()),
        ),
        MediaOwner::RoomMember {
            campaign_id,
            member_id,
        } => (
            "room_member",
            campaign_id.to_string(),
            Some(member_id.to_string()),
        ),
    }
}
fn decode(row: &sqlx::sqlite::SqliteRow) -> Result<OwnedMediaAsset, MediaAssetError> {
    let reference: MediaReference = serde_json::from_str(
        &row.try_get::<String, _>("reference_json")
            .map_err(storage)?,
    )
    .map_err(|_| MediaAssetError::Corrupt)?;
    let id = Uuid::parse_str(&row.try_get::<String, _>("owner_id").map_err(storage)?)
        .map_err(|_| MediaAssetError::Corrupt)?;
    let actor = row
        .try_get::<Option<String>, _>("actor_id")
        .map_err(storage)?
        .map(|v| Uuid::parse_str(&v))
        .transpose()
        .map_err(|_| MediaAssetError::Corrupt)?;
    let owner = match row
        .try_get::<String, _>("owner_kind")
        .map_err(storage)?
        .as_str()
    {
        "draft" => MediaOwner::Draft(id),
        "campaign" => MediaOwner::Campaign {
            campaign_id: id,
            actor_id: actor,
        },
        "room_member" => MediaOwner::RoomMember {
            campaign_id: id,
            member_id: actor.ok_or(MediaAssetError::Corrupt)?,
        },
        _ => return Err(MediaAssetError::Corrupt),
    };
    let bytes = row.try_get("payload").map_err(storage)?;
    let MediaReference::Owned { descriptor, .. } = &reference else {
        return Err(MediaAssetError::Corrupt);
    };
    let stored: MediaDescriptor = serde_json::from_str(
        &row.try_get::<String, _>("descriptor_json")
            .map_err(storage)?,
    )
    .map_err(|_| MediaAssetError::Corrupt)?;
    if *descriptor != stored {
        return Err(MediaAssetError::Corrupt);
    };
    Ok(OwnedMediaAsset {
        reference,
        owner,
        bytes,
    })
}
pub(crate) fn portrait_reference(sheet: &Value) -> Option<MediaReference> {
    serde_json::from_value(sheet.get("portraitMediaRef")?.clone())
        .ok()
        .filter(MediaReference::valid)
}
fn has_portrait(sheet: &Value, id: Uuid) -> bool {
    matches!(portrait_reference(sheet),Some(MediaReference::Owned{asset_id,category:MediaCategory::Portrait,..}) if asset_id==id)
}
fn reference_error(code: &'static str) -> app_application::ports::repositories::RepositoryError {
    app_application::ports::repositories::RepositoryError::Operation {
        operation: "media_reference",
        code,
    }
}
pub(crate) async fn current_map(
    tx: &mut Transaction<'_, Sqlite>,
    campaign: Uuid,
    session: Uuid,
) -> Result<Option<MediaReference>, app_application::ports::repositories::RepositoryError> {
    let raw: Option<String> = sqlx::query_scalar(
        "SELECT reference_json FROM current_map_media WHERE campaign_id=?1 AND session_id=?2",
    )
    .bind(campaign.to_string())
    .bind(session.to_string())
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| reference_error("media_asset_storage_unavailable"))?;
    raw.map(|raw|serde_json::from_str::<MediaReference>(&raw).map_err(|_|reference_error("media_asset_corrupt"))).transpose().and_then(|reference|{
        if reference.as_ref().is_some_and(|reference|!reference.valid() || matches!(reference,MediaReference::Owned{category,..} if *category!=MediaCategory::Map)){Err(reference_error("media_asset_corrupt"))} else {Ok(reference)}
    })
}
pub(crate) async fn select_map(
    tx: &mut Transaction<'_, Sqlite>,
    campaign: Uuid,
    session: Uuid,
    reference: Option<&MediaReference>,
    request: &str,
    restoring: bool,
) -> Result<(), app_application::ports::repositories::RepositoryError> {
    let Some(reference) = reference else {
        sqlx::query("DELETE FROM current_map_media WHERE campaign_id=?1")
            .bind(campaign.to_string())
            .execute(&mut **tx)
            .await
            .map_err(|_| reference_error("media_asset_storage_unavailable"))?;
        return Ok(());
    };
    if !reference.valid()
        || matches!(reference,MediaReference::Owned{category,..} if *category!=MediaCategory::Map)
    {
        return Err(reference_error("media_asset_invalid"));
    };
    if let MediaReference::Owned { asset_id, .. } = reference {
        let row = sqlx::query(
            "SELECT owner_kind,owner_id,reference_json FROM media_assets WHERE asset_id=?1",
        )
        .bind(asset_id.to_string())
        .fetch_optional(&mut **tx)
        .await
        .map_err(|_| reference_error("media_asset_storage_unavailable"))?;
        if let Some(row) = row {
            if row.get::<String, _>("owner_kind") != "campaign"
                || row.get::<String, _>("owner_id") != campaign.to_string()
                || serde_json::from_str::<MediaReference>(&row.get::<String, _>("reference_json"))
                    .map_err(|_| reference_error("media_asset_corrupt"))?
                    != *reference
            {
                return Err(reference_error("media_asset_denied"));
            };
            sqlx::query("INSERT INTO media_bindings(asset_id,campaign_id,actor_id) VALUES(?1,?2,'') ON CONFLICT DO NOTHING").bind(asset_id.to_string()).bind(campaign.to_string()).execute(&mut **tx).await.map_err(|_|reference_error("media_asset_storage_unavailable"))?;
        } else if !restoring {
            return Err(reference_error("media_asset_missing"));
        };
    }
    sqlx::query("INSERT INTO current_map_media(campaign_id,session_id,reference_json,request_id) VALUES(?1,?2,?3,?4) ON CONFLICT(campaign_id) DO UPDATE SET session_id=excluded.session_id,reference_json=excluded.reference_json,request_id=excluded.request_id").bind(campaign.to_string()).bind(session.to_string()).bind(serde_json::to_string(reference).map_err(|_|reference_error("media_asset_invalid"))?).bind(request).execute(&mut **tx).await.map_err(|_|reference_error("media_asset_storage_unavailable"))?;
    Ok(())
}

#[async_trait]
impl MediaAssetRepository for crate::SqliteStore {
    async fn insert(&self, asset: OwnedMediaAsset) -> Result<(), MediaAssetError> {
        let MediaReference::Owned {
            asset_id,
            descriptor,
            ..
        } = &asset.reference
        else {
            return Err(MediaAssetError::Invalid);
        };
        if !asset.reference.valid() || descriptor.byte_len != asset.bytes.len() as u64 {
            return Err(MediaAssetError::Invalid);
        };
        let (kind, owner, actor) = owner_columns(&asset.owner);
        if owner == Uuid::nil().to_string()
            || actor.as_deref() == Some(Uuid::nil().to_string().as_str())
        {
            return Err(MediaAssetError::Invalid);
        };
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(storage)?;
        if let Some(old)=sqlx::query("SELECT a.reference_json,a.owner_kind,a.owner_id,a.actor_id,c.descriptor_json,c.payload FROM media_assets a JOIN media_content c ON c.sha256=a.sha256 WHERE a.asset_id=?1").bind(asset_id.to_string()).fetch_optional(&mut *tx).await.map_err(storage)? {
            return if decode(&old)?==asset {Ok(())} else {Err(MediaAssetError::Denied)};
        }
        if kind != "draft" {
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM campaigns WHERE id=?1)")
                    .bind(&owner)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(storage)?;
            if !exists {
                return Err(MediaAssetError::Denied);
            };
        }
        // Compare duplicates rather than trusting the supplied hash to overwrite bytes.
        sqlx::query("INSERT INTO media_content(sha256,descriptor_json,payload) VALUES(?1,?2,?3) ON CONFLICT(sha256) DO NOTHING").bind(&descriptor.sha256).bind(serde_json::to_string(descriptor).map_err(storage)?).bind(&asset.bytes).execute(&mut *tx).await.map_err(storage)?;
        let existing =
            sqlx::query("SELECT descriptor_json,payload FROM media_content WHERE sha256=?1")
                .bind(&descriptor.sha256)
                .fetch_one(&mut *tx)
                .await
                .map_err(storage)?;
        if existing.get::<Vec<u8>, _>("payload") != asset.bytes
            || serde_json::from_str::<MediaDescriptor>(
                &existing.get::<String, _>("descriptor_json"),
            )
            .map_err(storage)?
                != *descriptor
        {
            return Err(MediaAssetError::Corrupt);
        };
        sqlx::query("INSERT INTO media_assets(asset_id,sha256,reference_json,owner_kind,owner_id,actor_id) VALUES(?1,?2,?3,?4,?5,?6)").bind(asset_id.to_string()).bind(&descriptor.sha256).bind(serde_json::to_string(&asset.reference).map_err(storage)?).bind(kind).bind(owner).bind(actor).execute(&mut *tx).await.map_err(storage)?;
        tx.commit().await.map_err(storage)?;
        Ok(())
    }
    async fn read(
        &self,
        id: Uuid,
        scope: &MediaReadScope,
    ) -> Result<OwnedMediaAsset, MediaAssetError> {
        let mut tx = self.pool.begin().await.map_err(storage)?;
        let row=sqlx::query("SELECT a.reference_json,a.owner_kind,a.owner_id,a.actor_id,c.descriptor_json,c.payload FROM media_assets a JOIN media_content c ON c.sha256=a.sha256 WHERE a.asset_id=?1").bind(id.to_string()).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or(MediaAssetError::Missing)?;
        let asset = decode(&row)?;
        let allowed = match scope {
            MediaReadScope::Draft(draft) => asset.owner == MediaOwner::Draft(*draft),
            MediaReadScope::Campaign(campaign) => {
                let exists: bool =
                    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM campaigns WHERE id=?1)")
                        .bind(campaign.to_string())
                        .fetch_one(&mut *tx)
                        .await
                        .map_err(storage)?;
                let bound:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM media_bindings WHERE asset_id=?1 AND campaign_id=?2)").bind(id.to_string()).bind(campaign.to_string()).fetch_one(&mut *tx).await.map_err(storage)?;
                exists
                    && (bound
                        || matches!(asset.owner,MediaOwner::Campaign{campaign_id,..} if campaign_id==*campaign))
            }
            MediaReadScope::Room(lease) => {
                let room = crate::rooms::read(&mut tx, lease.room_id())
                    .await
                    .map_err(|_| MediaAssetError::Denied)?;
                crate::rooms::verify_lease(&room, lease).map_err(|_| MediaAssetError::Denied)?;
                let party = crate::party::load(&mut tx, room.campaign_id)
                    .await
                    .map_err(storage)?
                    .ok_or(MediaAssetError::Denied)?;
                if party.session_id != room.session_id {
                    return Err(MediaAssetError::Denied);
                };
                let own = asset.owner
                    == MediaOwner::RoomMember {
                        campaign_id: room.campaign_id.0,
                        member_id: lease.member_id(),
                    };
                // Public art is read from current active actors, never a whole private sheet.
                let map = current_map(&mut tx, room.campaign_id.0, room.session_id.0)
                    .await
                    .map_err(storage)?;
                let public_map = map.as_ref() == Some(&asset.reference)
                    && matches!(asset.owner,MediaOwner::Campaign{campaign_id,..} if campaign_id==room.campaign_id.0);
                own || public_map
                    || room.members.iter().filter(|m| m.active).any(|member| {
                        party
                            .roster
                            .members
                            .iter()
                            .find(|actor| actor.member_id == member.member_id)
                            .and_then(|actor| party.characters.get(&actor.character_id))
                            .is_some_and(|sheet| has_portrait(sheet, id))
                    })
            }
        };
        if !allowed {
            return Err(MediaAssetError::Denied);
        };
        tx.commit().await.map_err(storage)?;
        Ok(asset)
    }
}

/// Called inside the same transaction as an authoritative character commit.
/// A local owner may bind its draft; a guest may bind only its own uploaded art.
pub(crate) async fn bind_portrait(
    tx: &mut Transaction<'_, Sqlite>,
    campaign: Uuid,
    actor: Uuid,
    sheet: &Value,
    member: Option<Uuid>,
) -> Result<(), app_application::ports::repositories::RepositoryError> {
    use app_application::ports::repositories::RepositoryError;
    let error = |code| RepositoryError::Operation {
        operation: "media_asset_binding",
        code,
    };
    let Some(raw) = sheet.get("portraitMediaRef").filter(|v| !v.is_null()) else {
        return Ok(());
    };
    let reference: MediaReference =
        serde_json::from_value(raw.clone()).map_err(|_| error("media_asset_invalid"))?;
    if !reference.valid() {
        return Err(error("media_asset_invalid"));
    };
    let MediaReference::Owned {
        asset_id,
        category: MediaCategory::Portrait,
        ..
    } = &reference
    else {
        return if matches!(reference, MediaReference::Bundled { .. }) {
            Ok(())
        } else {
            Err(error("media_asset_invalid"))
        };
    };
    let row = sqlx::query(
        "SELECT reference_json,owner_kind,owner_id,actor_id FROM media_assets WHERE asset_id=?1",
    )
    .bind(asset_id.to_string())
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| error("media_asset_storage_unavailable"))?
    .ok_or_else(|| error("media_asset_missing"))?;
    let stored: MediaReference = serde_json::from_str(&row.get::<String, _>("reference_json"))
        .map_err(|_| error("media_asset_corrupt"))?;
    if stored != reference {
        return Err(error("media_asset_corrupt"));
    };
    let kind: String = row.get("owner_kind");
    let owner: String = row.get("owner_id");
    let created_actor: Option<String> = row.get("actor_id");
    let bound:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM media_bindings WHERE asset_id=?1 AND campaign_id=?2 AND actor_id=?3)").bind(asset_id.to_string()).bind(campaign.to_string()).bind(actor.to_string()).fetch_one(&mut **tx).await.map_err(|_|error("media_asset_storage_unavailable"))?;
    let allowed = bound
        || (kind == "draft" && member.is_none())
        || (kind == "campaign"
            && owner == campaign.to_string()
            && created_actor.as_deref() == Some(actor.to_string().as_str()))
        || (kind == "room_member"
            && owner == campaign.to_string()
            && member.is_some_and(|id| created_actor.as_deref() == Some(id.to_string().as_str())));
    if !allowed {
        return Err(error("media_asset_denied"));
    };
    sqlx::query("INSERT INTO media_bindings(asset_id,campaign_id,actor_id) VALUES(?1,?2,?3) ON CONFLICT DO NOTHING").bind(asset_id.to_string()).bind(campaign.to_string()).bind(actor.to_string()).execute(&mut **tx).await.map_err(|_|error("media_asset_storage_unavailable"))?;
    Ok(())
}
