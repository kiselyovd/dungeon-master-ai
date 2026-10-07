use app_application::{
    combat::resolve::{prepare_action, ResolveCombatError},
    models::{combat::CombatProjection, room::RoomLease, room_combat::*},
    ports::repositories::RepositoryError,
};
use sqlx::{Row, Sqlite, Transaction};
use uuid::Uuid;

fn error(code: &'static str) -> RepositoryError {
    RepositoryError::Operation {
        operation: "room_combat",
        code,
    }
}
async fn context(
    tx: &mut Transaction<'_, Sqlite>,
    lease: &RoomLease,
    encounter: Uuid,
) -> Result<(Uuid, CombatProjection), RepositoryError> {
    let room = crate::rooms::read(tx, lease.room_id()).await?;
    crate::rooms::verify_lease(&room, lease)?;
    let row =
        sqlx::query("SELECT session_id,projection,ended_at FROM combat_encounters WHERE id=?1")
            .bind(encounter.to_string())
            .fetch_optional(&mut **tx)
            .await
            .map_err(|_| error("room_combat_read_failed"))?
            .ok_or_else(|| error("room_access_denied"))?;
    if row.get::<String, _>("session_id") != room.session_id.0.to_string() {
        return Err(error("room_access_denied"));
    }
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
    let raw = row
        .get::<Option<String>, _>("projection")
        .ok_or_else(|| error("room_combat_read_failed"))?;
    let mut projection: CombatProjection =
        serde_json::from_str(&raw).map_err(|_| error("room_combat_read_failed"))?;
    if projection.encounter_id != encounter
        || projection.campaign_scope.as_ref().is_none_or(|scope| {
            scope.campaign_id != room.campaign_id.0
                || scope.session_id != room.session_id.0
                || party
                    .roster
                    .members
                    .iter()
                    .find(|member| member.member_id == party.roster.host_member_id)
                    .is_none_or(|host| host.character_id != scope.character_id)
        })
    {
        return Err(error("room_access_denied"));
    }
    if row.get::<Option<String>, _>("ended_at").is_none() {
        crate::combat_binding::recover_loadouts(tx, &mut projection).await?;
    } else {
        if projection.snapshot.active {
            return Err(error("room_combat_conflict"));
        }
        // Historical terminal encounters retain their original roster and profiles.
        // A newly accepted hero cannot act in or read someone else's old encounter.
        let bound = projection
            .snapshot
            .combatants
            .iter()
            .filter_map(|actor| actor.character_id.map(|id| id.0))
            .collect::<Vec<_>>();
        let unique = bound
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        if bound.len() != unique.len() || bound.iter().any(|id| !party.characters.contains_key(id))
        {
            return Err(error("room_access_denied"));
        }
    }
    if projection
        .snapshot
        .combatants
        .iter()
        .filter(|actor| actor.character_id.is_some_and(|id| id.0 == owner))
        .count()
        != 1
    {
        return Err(error("room_access_denied"));
    }
    Ok((owner, projection))
}
pub(crate) async fn read(
    store: &crate::SqliteStore,
    lease: &RoomLease,
    encounter: Uuid,
) -> Result<CombatProjection, RepositoryError> {
    let mut tx = store
        .pool
        .begin()
        .await
        .map_err(|_| error("room_combat_begin_failed"))?;
    let (_, projection) = context(&mut tx, lease, encounter).await?;
    tx.commit()
        .await
        .map_err(|_| error("room_combat_commit_failed"))?;
    Ok(projection)
}
pub(crate) async fn act(
    store: &crate::SqliteStore,
    lease: &RoomLease,
    encounter: Uuid,
    command: RoomCombatCommand,
    rng_seed: u64,
) -> Result<RoomCombatResult, RepositoryError> {
    if encounter.is_nil()
        || command.request_id.trim().is_empty()
        || command.request_id.len() > 64
        || command.request_id.chars().any(char::is_control)
    {
        return Err(error("room_action_invalid"));
    }
    let mut tx = store
        .pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|_| error("room_combat_begin_failed"))?;
    let (owner, projection) = context(&mut tx, lease, encounter).await?;
    let request_id = format!(
        "room:{}:{}:{}",
        lease.room_id(),
        lease.member_id(),
        command.request_id
    );
    let signature=serde_json::to_string(&serde_json::json!({"room":lease.room_id(),"member":lease.member_id(),"character":owner,"encounter":encounter,"expected_revision":command.expected_revision,"action":command.action})).map_err(|_|error("room_action_invalid"))?;
    if let Some(row)=sqlx::query("SELECT signature,projection_json FROM combat_action_receipts WHERE encounter_id=?1 AND request_id=?2").bind(encounter.to_string()).bind(&request_id).fetch_optional(&mut *tx).await.map_err(|_|error("room_combat_read_failed"))? {
        if row.get::<String,_>("signature")!=signature {return Err(error("combat_request_mismatch"));}
        let recorded:CombatProjection=serde_json::from_str(&row.get::<String,_>("projection_json")).map_err(|_|error("room_combat_read_failed"))?;
        tx.commit().await.map_err(|_|error("room_combat_commit_failed"))?;
        return Ok(RoomCombatResult {projection,duplicate:true,receipt_revision:recorded.revision,receipt_events:recorded.events});
    }
    if projection.revision != command.expected_revision {
        return Err(RepositoryError::RevisionConflict {
            expected: command.expected_revision,
            actual: projection.revision,
        });
    }
    let actor = projection
        .snapshot
        .combatants
        .iter()
        .find(|actor| actor.character_id.is_some_and(|id| id.0 == owner))
        .ok_or_else(|| error("room_access_denied"))?
        .id;
    let mut projection = prepare_action(projection, command.action.for_actor(actor), rng_seed)
        .map_err(|failure| match failure {
            ResolveCombatError::Rejected(_) => error("room_action_rejected"),
            _ => error("room_combat_conflict"),
        })?;
    crate::combat::persist_projection(&mut tx, command.expected_revision, &mut projection).await?;
    sqlx::query("INSERT INTO combat_action_receipts(encounter_id,request_id,signature,projection_json,created_at) VALUES(?1,?2,?3,?4,?5)").bind(encounter.to_string()).bind(request_id).bind(signature).bind(serde_json::to_string(&projection).map_err(|_|error("room_combat_write_failed"))?).bind(chrono::Utc::now().to_rfc3339()).execute(&mut *tx).await.map_err(|_|error("room_combat_write_failed"))?;
    tx.commit()
        .await
        .map_err(|_| error("room_combat_commit_failed"))?;
    let receipt_revision = projection.revision;
    let receipt_events = projection.events.clone();
    Ok(RoomCombatResult {
        projection,
        duplicate: false,
        receipt_revision,
        receipt_events,
    })
}
