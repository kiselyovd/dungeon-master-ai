use app_application::{
    models::{
        chat::MessagePart,
        party::PartyRole,
        room::RoomLease,
        room_character::{OwnCharacterProposal, ProposalStatus, RoomCharacterProposal},
        room_state::*,
    },
    ports::repositories::RepositoryError,
};
use sqlx::{Row, Sqlite, Transaction};
use uuid::Uuid;

fn error(code: &'static str) -> RepositoryError {
    RepositoryError::Operation {
        operation: "room_state",
        code,
    }
}
fn proposal(row: sqlx::sqlite::SqliteRow) -> Result<RoomCharacterProposal, RepositoryError> {
    let parse = |key: &str| {
        Uuid::parse_str(&row.get::<String, _>(key)).map_err(|_| error("room_state_invalid"))
    };
    let proposal = RoomCharacterProposal {
        room_id: parse("room_id")?,
        proposal_id: parse("proposal_id")?,
        member_id: parse("member_id")?,
        character_id: parse("character_id")?,
        status: ProposalStatus::Pending,
        character: serde_json::from_str(&row.get::<String, _>("character_json"))
            .map_err(|_| error("room_state_invalid"))?,
    };
    proposal
        .summary()
        .ok_or_else(|| error("room_state_invalid"))?;
    Ok(proposal)
}
async fn state(
    tx: &mut Transaction<'_, Sqlite>,
    lease: &RoomLease,
    room: app_application::models::room::RoomRecord,
) -> Result<RoomState, RepositoryError> {
    let party = crate::party::load(tx, room.campaign_id)
        .await?
        .ok_or_else(|| error("room_access_denied"))?;
    if party.session_id != room.session_id || party.roster.host_member_id != room.host_member_id {
        return Err(error("room_access_denied"));
    }
    let current = crate::campaign_state::load_in_transaction(tx, room.campaign_id).await?;
    let public = party
        .view_for(party.roster.host_member_id)
        .map_err(|_| error("room_state_invalid"))?;
    let owner = party
        .roster
        .members
        .iter()
        .find(|member| member.member_id == lease.member_id() && member.role == lease.role())
        .map(|member| member.character_id);
    let own_character = owner.and_then(|id| party.characters.get(&id).cloned());
    let mut combat = if owner.is_some() {
        current
            .projection
            .combat
            .map(|value| serde_json::from_value(value).map_err(|_| error("room_state_invalid")))
            .transpose()?
    } else {
        None
    };
    if let (Some(owner), Some(projection)) = (owner, &mut combat) {
        app_application::models::room_combat::project_for_owner(projection, owner);
    }
    let rows=sqlx::query("SELECT * FROM room_character_proposals WHERE room_id=?1 AND status='pending' ORDER BY rowid").bind(room.room_id.to_string()).fetch_all(&mut **tx).await.map_err(|_|error("room_state_read_failed"))?;
    let mut own_proposal = None;
    let mut pending = Vec::new();
    for row in rows {
        let proposal = proposal(row)?;
        if !room
            .members
            .iter()
            .any(|member| member.member_id == proposal.member_id && member.active)
        {
            continue;
        }
        let summary = proposal
            .summary()
            .ok_or_else(|| error("room_state_invalid"))?;
        if proposal.member_id == lease.member_id() {
            own_proposal = Some(OwnCharacterProposal {
                summary: summary.clone(),
                character: proposal.character,
            });
        }
        if lease.role() == PartyRole::Host {
            pending.push(summary);
        }
    }
    let rows=sqlx::query("SELECT m.id,m.role,m.parts FROM messages m JOIN shared_messages sm ON sm.message_id=m.id AND sm.session_id=m.session_id WHERE m.session_id=?1 AND m.role IN('user','assistant') ORDER BY m.created_at DESC,m.id DESC LIMIT 100").bind(room.session_id.0.to_string()).fetch_all(&mut **tx).await.map_err(|_|error("room_state_read_failed"))?;
    let mut messages = Vec::new();
    for row in rows.into_iter().rev() {
        let parts: Vec<MessagePart> = serde_json::from_str(&row.get::<String, _>("parts"))
            .map_err(|_| error("room_state_invalid"))?;
        let full = parts
            .into_iter()
            .filter_map(|part| {
                if let MessagePart::Text { text } = part {
                    Some(text)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let truncated = full.chars().count() > 16384;
        messages.push(PublicRoomMessage {
            id: Uuid::parse_str(&row.get::<String, _>("id"))
                .map_err(|_| error("room_state_invalid"))?,
            role: row.get("role"),
            text: full.chars().take(16384).collect(),
            truncated,
        });
    }
    let agent_busy:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM agent_turn_operations WHERE session_id=?1 AND status='running') OR EXISTS(SELECT 1 FROM room_dm_turns d JOIN coop_rooms r ON r.room_id=d.room_id WHERE r.session_id=?1 AND d.status IN('running','interrupted') AND r.active=1)").bind(room.session_id.0.to_string()).fetch_one(&mut **tx).await.map_err(|_|error("room_state_read_failed"))?;
    Ok(RoomState {
        campaign: current.campaign.clone(),
        room: room.public_view(),
        title: current.campaign.title,
        revision: party.revision,
        actors: public.actors,
        own_character,
        own_proposal,
        pending_proposals: (lease.role() == PartyRole::Host).then_some(pending),
        messages,
        combat,
        scene: current.projection.scene,
        adventure: current.projection.adventure,
        clock: current.projection.adventuring.unwrap_or_default(),
        journal: current.projection.journal,
        npcs: current.projection.npcs,
        agent_busy,
    })
}
pub(crate) async fn updates(
    store: &crate::SqliteStore,
    lease: &RoomLease,
    after: Option<u64>,
    force_snapshot: bool,
) -> Result<Option<RoomUpdate>, RepositoryError> {
    let mut tx = store
        .pool
        .begin()
        .await
        .map_err(|_| error("room_state_begin_failed"))?;
    let room = crate::rooms::read(&mut tx, lease.room_id()).await?;
    crate::rooms::verify_lease(&room, lease)?;
    let head: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(sequence),0) FROM room_changes WHERE room_id=?1")
            .bind(room.room_id.to_string())
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| error("room_state_read_failed"))?;
    let reset_cursor:i64=sqlx::query_scalar("SELECT COALESCE(MAX(sequence),0) FROM room_changes WHERE room_id=?1 AND kind='timeline_reset'").bind(room.room_id.to_string()).fetch_one(&mut *tx).await.map_err(|_|error("room_state_read_failed"))?;
    let head = head as u64;
    let reset_cursor = reset_cursor as u64;
    if after == Some(head) && !force_snapshot {
        tx.commit()
            .await
            .map_err(|_| error("room_state_commit_failed"))?;
        return Ok(None);
    }
    let reset = after.is_none_or(|cursor| cursor > head || cursor < reset_cursor);
    let rows = if reset {
        let mut rows=sqlx::query("SELECT sequence,kind,payload_json FROM room_changes WHERE room_id=?1 AND sequence>=?2 ORDER BY sequence DESC LIMIT 128").bind(room.room_id.to_string()).bind(reset_cursor as i64).fetch_all(&mut *tx).await.map_err(|_|error("room_state_read_failed"))?;
        rows.reverse();
        rows
    } else {
        sqlx::query("SELECT sequence,kind,payload_json FROM room_changes WHERE room_id=?1 AND sequence>?2 ORDER BY sequence ASC LIMIT 128").bind(room.room_id.to_string()).bind(after.unwrap_or_default() as i64).fetch_all(&mut *tx).await.map_err(|_|error("room_state_read_failed"))?
    };
    let events = rows
        .into_iter()
        .map(|row| {
            Ok(RoomChange {
                sequence: row.get::<i64, _>("sequence") as u64,
                kind: row.get("kind"),
                payload: serde_json::from_str(&row.get::<String, _>("payload_json"))
                    .map_err(|_| error("room_state_invalid"))?,
            })
        })
        .collect::<Result<Vec<_>, RepositoryError>>()?;
    let cursor = if reset {
        head
    } else {
        events.last().map(|event| event.sequence).unwrap_or(head)
    };
    let state = state(&mut tx, lease, room).await?;
    tx.commit()
        .await
        .map_err(|_| error("room_state_commit_failed"))?;
    Ok(Some(RoomUpdate {
        cursor,
        head,
        reset,
        events,
        state,
    }))
}
