use crate::messages::repository_error;
use app_application::{
    models::{identity::Revision, room::RoomLease, room_recovery::*},
    ports::repositories::RepositoryError,
    vitality,
};
use sqlx::Row;
fn error(code: &'static str) -> RepositoryError {
    repository_error(code)
}

pub(crate) async fn recover(
    store: &crate::SqliteStore,
    lease: &RoomLease,
    command: RoomRecoveryCommand,
    seed: u64,
) -> Result<RoomRecoveryReceipt, RepositoryError> {
    crate::room_world::nonce(&command.request_id)?;
    let request = serde_json::to_string(&command).map_err(|_| error("room_action_invalid"))?;
    let mut tx = store
        .pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|_| error("recovery_begin_failed"))?;
    let (room, party, owner, current) = crate::room_world::context(&mut tx, lease).await?;
    let reset:i64=sqlx::query_scalar("SELECT COALESCE(MAX(sequence),0) FROM room_changes WHERE room_id=?1 AND kind='timeline_reset'")
        .bind(room.room_id.to_string()).fetch_one(&mut *tx).await.map_err(|_|error("recovery_read_failed"))?;
    if let Some(row)=sqlx::query("SELECT request_json,receipt_json,reset_cursor FROM room_recovery_receipts WHERE room_id=?1 AND member_id=?2 AND request_id=?3")
        .bind(room.room_id.to_string()).bind(lease.member_id().to_string()).bind(&command.request_id).fetch_optional(&mut *tx).await.map_err(|_|error("recovery_read_failed"))? {
        if row.get::<String,_>("request_json")!=request || row.get::<i64,_>("reset_cursor")!=reset { return Err(error("room_world_conflict")); }
        let mut result:RoomRecoveryReceipt=serde_json::from_str(&row.get::<String,_>("receipt_json")).map_err(|_|error("recovery_read_failed"))?;
        result.duplicate=true;tx.commit().await.map_err(|_|error("recovery_commit_failed"))?;return Ok(result);
    }
    if current.campaign.revision != command.expected_revision {
        return Err(RepositoryError::Conflict {
            expected: command.expected_revision,
            actual: current.campaign.revision,
        });
    }
    if crate::rest::active_combat(&mut tx, room.session_id).await? {
        return Err(error("recovery_in_combat"));
    }
    let now = current
        .projection
        .adventuring
        .as_ref()
        .map_or(0, |s| s.elapsed_seconds);
    let helper =
        app_application::recovery::actor(&party.characters[&owner], owner).map_err(error)?;
    let target_id = match &command.intent {
        RecoveryIntent::Aid {
            target_character_id,
        }
        | RecoveryIntent::Wait {
            target_character_id,
        } => *target_character_id,
        RecoveryIntent::DeathSave {} => owner,
    };
    let member = party
        .roster
        .members
        .iter()
        .find(|member| member.character_id == target_id)
        .ok_or_else(|| error("room_access_denied"))?;
    if !room.members.iter().any(|present| {
        present.member_id == member.member_id && present.active && present.role == member.role
    }) {
        return Err(error("room_access_denied"));
    }
    let mut sheet = party
        .characters
        .get(&target_id)
        .ok_or_else(|| error("room_access_denied"))?
        .clone();
    let mut target = app_application::recovery::actor(&sheet, target_id).map_err(error)?;
    if target.current_hp != 0 || target.is_dead {
        return Err(error("hero_recovery_not_needed"));
    }
    let mut rng = app_application::recovery::RecoveryEngine::new(seed);
    let mut medicine = None;
    let until = match command.intent {
        RecoveryIntent::Aid { .. } => {
            if target_id == owner
                || helper.current_hp <= 0
                || helper.is_dead
                || helper.is_incapacitated()
                || target.death_saves.stable
            {
                return Err(error("hero_aid_invalid"));
            }
            let check = rng
                .medicine(&helper, &party.characters[&owner])
                .map_err(error)?;
            let total = check.total;
            medicine = Some(check);
            let until = now
                .checked_add(6)
                .ok_or_else(|| error("adventuring_clock_invalid"))?;
            if let Some(timer) = rng.stabilize(&mut target, total, until).map_err(error)? {
                vitality::store_actor(&mut sheet, &target).map_err(error)?;
                vitality::schedule_natural_recovery(&mut sheet, target_id, timer).map_err(error)?;
                crate::party::persist_sheet(&mut tx, room.campaign_id, target_id, sheet).await?;
            }
            until
        }
        RecoveryIntent::Wait { .. } => {
            if !target.death_saves.stable
                || (owner != target_id
                    && (helper.current_hp <= 0 || helper.is_dead || helper.is_incapacitated()))
            {
                return Err(error("hero_aid_invalid"));
            }
            if !app_application::recovery::safe(&current) {
                return Err(error("rest_unsafe"));
            }
            let timer = if let Some(timer) = vitality::read(&sheet, target_id)
                .map_err(error)?
                .and_then(|s| s.natural_recovery)
            {
                timer
            } else {
                let timer = rng.natural_deadline(&target, now).map_err(error)?;
                vitality::store_actor(&mut sheet, &target).map_err(error)?;
                vitality::schedule_natural_recovery(&mut sheet, target_id, timer.clone())
                    .map_err(error)?;
                crate::party::persist_sheet(&mut tx, room.campaign_id, target_id, sheet).await?;
                timer
            };
            timer.ready_at_seconds.max(now)
        }
        RecoveryIntent::DeathSave {} => {
            if target.death_saves.stable {
                return Err(error("hero_recovery_not_needed"));
            }
            now.checked_add(6)
                .ok_or_else(|| error("adventuring_clock_invalid"))?
        }
    };
    let death_saves =
        crate::rest::write_clock_with_rng(&mut tx, room.session_id, until, &mut rng).await?;
    let final_party = crate::party::load(&mut tx, room.campaign_id)
        .await?
        .ok_or_else(|| error("party_missing"))?;
    let final_sheet = &final_party.characters[&target_id];
    let status = vitality::read(final_sheet, target_id).map_err(error)?;
    let revision = crate::campaigns::advance_session_revision(
        &mut tx,
        room.campaign_id,
        room.session_id,
        command.expected_revision,
        "hero_recovery",
        &format!(
            "room:{}:{}:{}",
            room.room_id,
            lease.member_id(),
            command.request_id
        ),
    )
    .await?;
    let result = RoomRecoveryReceipt {
        receipt_revision: Revision(revision.0),
        duplicate: false,
        target_character_id: target_id,
        stabilized: status.as_ref().is_some_and(|s| s.death_saves.stable),
        recovered: final_sheet["hp"].as_i64().is_some_and(|hp| hp > 0),
        medicine,
        death_saves,
        ready_at_seconds: status.and_then(|s| s.natural_recovery.map(|r| r.ready_at_seconds)),
    };
    sqlx::query("INSERT INTO room_recovery_receipts(room_id,member_id,request_id,request_json,receipt_json,reset_cursor) VALUES(?1,?2,?3,?4,?5,?6)")
        .bind(room.room_id.to_string()).bind(lease.member_id().to_string()).bind(command.request_id).bind(request)
        .bind(serde_json::to_string(&result).map_err(|_|error("recovery_write_failed"))?).bind(reset).execute(&mut *tx).await.map_err(|_|error("recovery_write_failed"))?;
    tx.commit()
        .await
        .map_err(|_| error("recovery_commit_failed"))?;
    Ok(result)
}
