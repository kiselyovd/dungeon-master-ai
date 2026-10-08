use app_application::{
    models::{
        identity::Revision,
        party::{EnrollPartyCharacter, PartyRole, PartyState},
        room::{RoomLease, RoomRecord},
        room_character::*,
    },
    ports::repositories::RepositoryError,
};
use sqlx::{Row, Sqlite, Transaction};
use uuid::Uuid;

fn error(code: &'static str) -> RepositoryError {
    RepositoryError::Operation {
        operation: "room_character",
        code,
    }
}
fn uuid(row: &sqlx::sqlite::SqliteRow, key: &str) -> Result<Uuid, RepositoryError> {
    Uuid::parse_str(&row.get::<String, _>(key)).map_err(|_| error("room_character_invalid"))
}
fn decode(row: sqlx::sqlite::SqliteRow) -> Result<RoomCharacterProposal, RepositoryError> {
    let proposal = RoomCharacterProposal {
        room_id: uuid(&row, "room_id")?,
        proposal_id: uuid(&row, "proposal_id")?,
        member_id: uuid(&row, "member_id")?,
        character_id: uuid(&row, "character_id")?,
        status: match row.get::<String, _>("status").as_str() {
            "pending" => ProposalStatus::Pending,
            "accepted" => ProposalStatus::Accepted,
            "superseded" => ProposalStatus::Superseded,
            _ => return Err(error("room_character_invalid")),
        },
        character: serde_json::from_str(&row.get::<String, _>("character_json"))
            .map_err(|_| error("room_character_invalid"))?,
    };
    proposal
        .summary()
        .ok_or_else(|| error("room_character_invalid"))?;
    Ok(proposal)
}
async fn authorized(
    tx: &mut Transaction<'_, Sqlite>,
    lease: &RoomLease,
    host: bool,
) -> Result<(RoomRecord, PartyState), RepositoryError> {
    let room = crate::rooms::read(tx, lease.room_id()).await?;
    crate::rooms::verify_lease(&room, lease)?;
    if host && lease.role() != PartyRole::Host {
        return Err(error("room_access_denied"));
    }
    let party = crate::party::load(tx, room.campaign_id)
        .await?
        .ok_or_else(|| error("room_character_unavailable"))?;
    if party.session_id != room.session_id {
        return Err(error("room_access_denied"));
    }
    Ok((room, party))
}
async fn proposal(
    tx: &mut Transaction<'_, Sqlite>,
    id: Uuid,
) -> Result<Option<RoomCharacterProposal>, RepositoryError> {
    sqlx::query("SELECT * FROM room_character_proposals WHERE proposal_id=?1")
        .bind(id.to_string())
        .fetch_optional(&mut **tx)
        .await
        .map_err(|_| error("room_character_read_failed"))?
        .map(decode)
        .transpose()
}
pub(crate) async fn submit(
    store: &crate::SqliteStore,
    lease: &RoomLease,
    mut command: SubmitRoomCharacter,
) -> Result<ProposalSummary, RepositoryError> {
    if lease.role() != PartyRole::Player
        || command.proposal_id.is_nil()
        || command.character_id.is_nil()
    {
        return Err(error("room_access_denied"));
    }
    if !app_application::models::campaign_state::valid_character_document(&command.character)
        || !app_application::models::campaign_state::within_character_budget(&command.character)
    {
        return Err(error("room_character_invalid"));
    }
    app_application::vitality::strip_client_state(&mut command.character);
    app_application::defense::normalize(&mut command.character).map_err(error)?;
    if !app_application::models::campaign_state::within_character_budget(&command.character) {
        return Err(error("character_too_large"));
    }
    command.character["characterId"] = serde_json::json!(command.character_id);
    let mut tx = store
        .pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|_| error("room_character_begin_failed"))?;
    let (room, party) = authorized(&mut tx, lease, false).await?;
    if let Some(old) = proposal(&mut tx, command.proposal_id).await? {
        if old.room_id != room.room_id || old.member_id != lease.member_id() {
            return Err(error("room_access_denied"));
        }
        if old.character_id != command.character_id || old.character != command.character {
            return Err(error("room_character_conflict"));
        }
        let summary = old
            .summary()
            .ok_or_else(|| error("room_character_invalid"))?;
        tx.commit()
            .await
            .map_err(|_| error("room_character_commit_failed"))?;
        return Ok(summary);
    }
    if party.roster.members.iter().any(|member| {
        member.member_id == lease.member_id() || member.character_id == command.character_id
    }) {
        return Err(error("room_character_conflict"));
    }
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM room_character_proposals WHERE room_id=?1 AND member_id=?2",
    )
    .bind(room.room_id.to_string())
    .bind(lease.member_id().to_string())
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| error("room_character_read_failed"))?;
    if count >= 16 {
        return Err(error("room_character_conflict"));
    }
    sqlx::query("UPDATE room_character_proposals SET status='superseded' WHERE room_id=?1 AND member_id=?2 AND status='pending'").bind(room.room_id.to_string()).bind(lease.member_id().to_string()).execute(&mut *tx).await.map_err(|_|error("room_character_write_failed"))?;
    sqlx::query("INSERT INTO room_character_proposals(proposal_id,room_id,member_id,character_id,status,character_json) VALUES(?1,?2,?3,?4,'pending',?5)").bind(command.proposal_id.to_string()).bind(room.room_id.to_string()).bind(lease.member_id().to_string()).bind(command.character_id.to_string()).bind(command.character.to_string()).execute(&mut *tx).await.map_err(|_|error("room_character_write_failed"))?;
    let summary = RoomCharacterProposal {
        room_id: room.room_id,
        proposal_id: command.proposal_id,
        member_id: lease.member_id(),
        character_id: command.character_id,
        status: ProposalStatus::Pending,
        character: command.character,
    }
    .summary()
    .ok_or_else(|| error("room_character_invalid"))?;
    tx.commit()
        .await
        .map_err(|_| error("room_character_commit_failed"))?;
    Ok(summary)
}
pub(crate) async fn own(
    store: &crate::SqliteStore,
    lease: &RoomLease,
) -> Result<Option<OwnCharacterProposal>, RepositoryError> {
    let mut tx = store
        .pool
        .begin()
        .await
        .map_err(|_| error("room_character_begin_failed"))?;
    authorized(&mut tx, lease, false).await?;
    let row=sqlx::query("SELECT * FROM room_character_proposals WHERE room_id=?1 AND member_id=?2 ORDER BY rowid DESC LIMIT 1").bind(lease.room_id().to_string()).bind(lease.member_id().to_string()).fetch_optional(&mut *tx).await.map_err(|_|error("room_character_read_failed"))?;
    let result = row
        .map(decode)
        .transpose()?
        .map(|proposal| OwnCharacterProposal {
            summary: proposal.summary().expect("decoded proposal"),
            character: proposal.character,
        });
    tx.commit()
        .await
        .map_err(|_| error("room_character_commit_failed"))?;
    Ok(result)
}
pub(crate) async fn pending(
    store: &crate::SqliteStore,
    lease: &RoomLease,
) -> Result<Vec<ProposalSummary>, RepositoryError> {
    let mut tx = store
        .pool
        .begin()
        .await
        .map_err(|_| error("room_character_begin_failed"))?;
    let (room, _) = authorized(&mut tx, lease, true).await?;
    let rows=sqlx::query("SELECT * FROM room_character_proposals WHERE room_id=?1 AND status='pending' ORDER BY rowid").bind(lease.room_id().to_string()).fetch_all(&mut *tx).await.map_err(|_|error("room_character_read_failed"))?;
    let mut result = Vec::new();
    for row in rows {
        let proposal = decode(row)?;
        if room.members.iter().any(|member| {
            member.member_id == proposal.member_id
                && member.active
                && member.role == PartyRole::Player
        }) {
            result.push(
                proposal
                    .summary()
                    .ok_or_else(|| error("room_character_invalid"))?,
            );
        }
    }
    tx.commit()
        .await
        .map_err(|_| error("room_character_commit_failed"))?;
    Ok(result)
}
pub(crate) async fn approve(
    store: &crate::SqliteStore,
    lease: &RoomLease,
    command: ApproveRoomCharacter,
) -> Result<CharacterApproval, RepositoryError> {
    if command.proposal_id.is_nil()
        || command.causation_id.trim().is_empty()
        || command.causation_id.len() > 64
    {
        return Err(error("room_character_invalid"));
    }
    let request = serde_json::to_string(&command).map_err(|_| error("room_character_invalid"))?;
    let mut tx = store
        .pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|_| error("room_character_begin_failed"))?;
    let (room, _) = authorized(&mut tx, lease, true).await?;
    let mut proposal = proposal(&mut tx, command.proposal_id)
        .await?
        .ok_or(RepositoryError::NotFound)?;
    if proposal.room_id != room.room_id
        || !room.members.iter().any(|member| {
            member.member_id == proposal.member_id
                && member.active
                && member.role == PartyRole::Player
        })
    {
        return Err(error("room_access_denied"));
    }
    if let Some(row)=sqlx::query("SELECT request_json,revision FROM room_character_approvals WHERE room_id=?1 AND causation_id=?2").bind(room.room_id.to_string()).bind(&command.causation_id).fetch_optional(&mut *tx).await.map_err(|_|error("room_character_read_failed"))? {
        if row.get::<String,_>("request_json")!=request || proposal.status!=ProposalStatus::Accepted { return Err(error("room_character_conflict")); }
        let approval=CharacterApproval { proposal:proposal.summary().ok_or_else(||error("room_character_invalid"))?, revision:Revision(row.get::<i64,_>("revision") as u64) };
        tx.commit().await.map_err(|_|error("room_character_commit_failed"))?;
        return Ok(approval);
    }
    if proposal.status != ProposalStatus::Pending {
        return Err(error("room_character_conflict"));
    }
    let party = crate::party::enroll_in_transaction(
        &mut tx,
        EnrollPartyCharacter {
            campaign_id: room.campaign_id,
            session_id: room.session_id,
            expected_revision: command.expected_revision,
            causation_id: format!("room-approve:{}:{}", room.room_id, command.causation_id),
            verified_host_member_id: room.host_member_id,
            member_id: proposal.member_id,
            character_id: proposal.character_id,
            character: proposal.character.clone(),
        },
    )
    .await?;
    let updated=sqlx::query("UPDATE room_character_proposals SET status='accepted' WHERE proposal_id=?1 AND status='pending'").bind(proposal.proposal_id.to_string()).execute(&mut *tx).await.map_err(|_|error("room_character_write_failed"))?;
    if updated.rows_affected() != 1 {
        return Err(error("room_character_conflict"));
    }
    sqlx::query("INSERT INTO room_character_approvals(room_id,causation_id,proposal_id,request_json,revision) VALUES(?1,?2,?3,?4,?5)").bind(room.room_id.to_string()).bind(command.causation_id).bind(proposal.proposal_id.to_string()).bind(request).bind(party.revision.0 as i64).execute(&mut *tx).await.map_err(|_|error("room_character_write_failed"))?;
    proposal.status = ProposalStatus::Accepted;
    let approval = CharacterApproval {
        proposal: proposal
            .summary()
            .ok_or_else(|| error("room_character_invalid"))?,
        revision: party.revision,
    };
    tx.commit()
        .await
        .map_err(|_| error("room_character_commit_failed"))?;
    Ok(approval)
}
