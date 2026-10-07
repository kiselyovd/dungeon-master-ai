use app_application::{
    models::{
        campaign_state::CampaignSessionState,
        party::{PartyRole, PartyState},
        rest::{RestIntent, RestStatus},
        room::{RoomLease, RoomRecord},
        room_world::*,
    },
    ports::repositories::RepositoryError,
};
use sqlx::{Row, Sqlite, Transaction};
use uuid::Uuid;
fn error(code: &'static str) -> RepositoryError {
    RepositoryError::Operation {
        operation: "room_world",
        code,
    }
}
pub(crate) fn nonce(id: &str) -> Result<(), RepositoryError> {
    if id.trim().is_empty() || id.len() > 64 || id.chars().any(char::is_control) {
        return Err(error("room_action_invalid"));
    }
    Ok(())
}
pub(crate) async fn context(
    tx: &mut Transaction<'_, Sqlite>,
    lease: &RoomLease,
) -> Result<(RoomRecord, PartyState, Uuid, CampaignSessionState), RepositoryError> {
    let room = crate::rooms::read(tx, lease.room_id()).await?;
    crate::rooms::verify_lease(&room, lease)?;
    let party = crate::party::load(tx, room.campaign_id)
        .await?
        .ok_or_else(|| error("room_access_denied"))?;
    if party.session_id != room.session_id || party.roster.host_member_id != room.host_member_id {
        return Err(error("room_access_denied"));
    }
    let owner = party
        .roster
        .members
        .iter()
        .find(|member| member.member_id == lease.member_id() && member.role == lease.role())
        .ok_or_else(|| error("room_access_denied"))?
        .character_id;
    let mut current = crate::campaign_state::load_in_transaction(tx, room.campaign_id).await?;
    current.character_id = Some(owner);
    current.projection.character = party.characters[&owner].clone();
    Ok((room, party, owner, current))
}
async fn replay(
    tx: &mut Transaction<'_, Sqlite>,
    lease: &RoomLease,
    id: &str,
    kind: &str,
    request: &str,
) -> Result<Option<RoomWorldReceipt>, RepositoryError> {
    let row=sqlx::query("SELECT kind,request_json,receipt_json FROM room_world_receipts WHERE room_id=?1 AND member_id=?2 AND request_id=?3").bind(lease.room_id().to_string()).bind(lease.member_id().to_string()).bind(id).fetch_optional(&mut **tx).await.map_err(|_|error("room_world_read_failed"))?;
    let Some(row) = row else {
        return Ok(None);
    };
    if row.get::<String, _>("kind") != kind || row.get::<String, _>("request_json") != request {
        return Err(error("room_world_conflict"));
    }
    let mut receipt: RoomWorldReceipt = serde_json::from_str(&row.get::<String, _>("receipt_json"))
        .map_err(|_| error("room_world_read_failed"))?;
    receipt.duplicate = true;
    Ok(Some(receipt))
}
async fn record(
    tx: &mut Transaction<'_, Sqlite>,
    lease: &RoomLease,
    id: &str,
    kind: &str,
    request: String,
    receipt: &RoomWorldReceipt,
) -> Result<(), RepositoryError> {
    sqlx::query("INSERT INTO room_world_receipts(room_id,member_id,request_id,kind,request_json,receipt_json) VALUES(?1,?2,?3,?4,?5,?6)").bind(lease.room_id().to_string()).bind(lease.member_id().to_string()).bind(id).bind(kind).bind(request).bind(serde_json::to_string(receipt).map_err(|_|error("room_world_write_failed"))?).execute(&mut **tx).await.map_err(|_|error("room_world_write_failed"))?;
    Ok(())
}
pub(crate) async fn adventure(
    store: &crate::SqliteStore,
    lease: &RoomLease,
    command: RoomAdventureCommand,
) -> Result<RoomWorldReceipt, RepositoryError> {
    nonce(&command.request_id)?;
    let request = serde_json::to_string(&command).map_err(|_| error("room_action_invalid"))?;
    let mut tx = store
        .pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|_| error("room_world_begin_failed"))?;
    let (room, party, owner, _) = context(&mut tx, lease).await?;
    if let Some(receipt) =
        replay(&mut tx, lease, &command.request_id, "adventure", &request).await?
    {
        tx.commit()
            .await
            .map_err(|_| error("room_world_commit_failed"))?;
        return Ok(receipt);
    }
    let state = crate::adventure::act_in_transaction(
        &mut tx,
        app_application::models::adventure::AdventureCommand {
            campaign_id: room.campaign_id,
            session_id: room.session_id,
            character_id: owner,
            expected_revision: command.expected_revision,
            causation_id: format!(
                "room:{}:{}:{}",
                room.room_id,
                lease.member_id(),
                command.request_id
            ),
            action: command.action,
        },
        Some(party.characters[&owner].clone()),
    )
    .await?;
    let receipt = RoomWorldReceipt {
        receipt_revision: state.campaign.revision,
        duplicate: false,
        rest_outcome: None,
    };
    record(
        &mut tx,
        lease,
        &command.request_id,
        "adventure",
        request,
        &receipt,
    )
    .await?;
    tx.commit()
        .await
        .map_err(|_| error("room_world_commit_failed"))?;
    Ok(receipt)
}
pub(crate) async fn status(
    store: &crate::SqliteStore,
    lease: &RoomLease,
) -> Result<RestStatus, RepositoryError> {
    let mut tx = store
        .pool
        .begin()
        .await
        .map_err(|_| error("room_world_begin_failed"))?;
    let (_, _, _, current) = context(&mut tx, lease).await?;
    let mut status = app_application::rest::status_for_state(&current)?;
    status.combat_active |= crate::rest::active_combat(&mut tx, current.session_id).await?;
    tx.commit()
        .await
        .map_err(|_| error("room_world_commit_failed"))?;
    Ok(status)
}
pub(crate) async fn rest(
    store: &crate::SqliteStore,
    lease: &RoomLease,
    command: RoomRestCommand,
    rng_seed: u64,
) -> Result<RoomWorldReceipt, RepositoryError> {
    nonce(&command.request_id)?;
    if !matches!(command.intent, RestIntent::Short { .. }) && lease.role() != PartyRole::Host {
        return Err(error("room_access_denied"));
    }
    let request = serde_json::to_string(&command).map_err(|_| error("room_action_invalid"))?;
    let mut tx = store
        .pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|_| error("room_world_begin_failed"))?;
    let (room, party, owner, current) = context(&mut tx, lease).await?;
    if let Some(receipt) = replay(&mut tx, lease, &command.request_id, "rest", &request).await? {
        tx.commit()
            .await
            .map_err(|_| error("room_world_commit_failed"))?;
        return Ok(receipt);
    }
    if current.campaign.revision != command.expected_revision {
        return Err(RepositoryError::Conflict {
            expected: command.expected_revision,
            actual: current.campaign.revision,
        });
    }
    let group = matches!(command.intent, RestIntent::Long);
    if crate::rest::active_combat(&mut tx, room.session_id).await? {
        return Err(error("rest_unsafe"));
    }
    let mut own_outcome = None;
    for (index, (id, sheet)) in party.characters.iter().enumerate() {
        if !group && *id != owner {
            continue;
        }
        let mut actor = current.clone();
        actor.character_id = Some(*id);
        actor.projection.character = sheet.clone();
        let outcome = app_application::rest::resolve_for_state(
            &actor,
            command.intent,
            rng_seed.wrapping_add(index as u64),
        )?;
        let mut sheet = sheet.clone();
        app_application::rest::apply_rest_to_sheet(&mut sheet, &outcome).map_err(error)?;
        crate::party::persist_sheet(&mut tx, room.campaign_id, *id, sheet).await?;
        if *id == owner {
            own_outcome = Some(outcome);
        }
    }
    let outcome = own_outcome.ok_or_else(|| error("room_access_denied"))?;
    crate::rest::write_clock(&mut tx, room.session_id, outcome.elapsed_seconds).await?;
    crate::campaigns::advance_session_revision(
        &mut tx,
        room.campaign_id,
        room.session_id,
        command.expected_revision,
        "room_rest_completed",
        &format!(
            "room:{}:{}:{}",
            room.room_id,
            lease.member_id(),
            command.request_id
        ),
    )
    .await?;
    let receipt = RoomWorldReceipt {
        receipt_revision: app_application::models::identity::Revision(
            command.expected_revision.0 + 1,
        ),
        duplicate: false,
        rest_outcome: Some(outcome),
    };
    record(
        &mut tx,
        lease,
        &command.request_id,
        "rest",
        request,
        &receipt,
    )
    .await?;
    tx.commit()
        .await
        .map_err(|_| error("room_world_commit_failed"))?;
    Ok(receipt)
}
