//! A prepared owner check, its outcome and shared revision commit atomically.
use crate::{messages::repository_error, SqliteStore};
use app_application::{
    agent::commands::RollDiceCommand,
    models::{room::RoomLease, room_agent::PreparedRoomTool},
    ports::repositories::RepositoryError,
};
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

pub(crate) async fn resolve(
    store: &SqliteStore,
    lease: &RoomLease,
    turn: Uuid,
    tool: &str,
    command: RollDiceCommand,
    seed: u64,
) -> Result<Value, RepositoryError> {
    let args =
        serde_json::to_value(&command).map_err(|_| repository_error("hero_check_invalid"))?;
    app_application::agent::tool_decoder::decode_tool_call("roll_dice", &args)
        .map_err(|_| repository_error("hero_check_invalid"))?;
    let check = command
        .check
        .as_ref()
        .ok_or_else(|| repository_error("hero_check_invalid"))?;
    let mut tx = store
        .pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|_| repository_error("room_check_begin_failed"))?;
    let (room, party, owner, current) = crate::room_world::context(&mut tx, lease).await?;
    if check.character_id != owner {
        return Err(RepositoryError::Operation {
            operation: "room_skill_check",
            code: "room_access_denied",
        });
    }
    crate::room_agent::timeline(&mut tx, lease, turn).await?;
    let row=sqlx::query("SELECT command_json,prepared_json FROM room_agent_tools WHERE room_id=?1 AND member_id=?2 AND turn_id=?3 AND tool_id=?4")
        .bind(room.room_id.to_string()).bind(lease.member_id().to_string()).bind(turn.to_string()).bind(tool).fetch_optional(&mut *tx).await.map_err(|_|repository_error("room_check_read_failed"))?.ok_or_else(||repository_error("room_check_not_prepared"))?;
    let request_signature = json!({"check":command}).to_string();
    if row.get::<String, _>("command_json") != request_signature {
        return Err(repository_error("room_world_conflict"));
    }
    let mut prepared: PreparedRoomTool =
        serde_json::from_str(&row.get::<String, _>("prepared_json"))
            .map_err(|_| repository_error("room_check_read_failed"))?;
    if let Some(mut result) = prepared.check_result {
        validate_receipt(&result, owner, check)?;
        result["duplicate"] = json!(true);
        tx.commit()
            .await
            .map_err(|_| repository_error("room_check_commit_failed"))?;
        return Ok(result);
    }
    if current.campaign.revision.0 != prepared.expected_revision {
        return Err(RepositoryError::Conflict {
            expected: app_application::models::identity::Revision(prepared.expected_revision),
            actual: current.campaign.revision,
        });
    }
    let mut result = app_application::hero_checks::roll_skill_seeded(
        &party.characters[&owner],
        check,
        command.advantage.unwrap_or(false),
        command.disadvantage.unwrap_or(false),
        seed,
    )
    .map_err(|code| {
        if code == "hero_check_incapacitated" {
            RepositoryError::Operation {
                operation: "room_skill_check",
                code,
            }
        } else {
            repository_error(code)
        }
    })?;
    let revision = crate::campaigns::advance_session_revision(
        &mut tx,
        room.campaign_id,
        room.session_id,
        app_application::models::identity::Revision(prepared.expected_revision),
        "skill_check",
        &format!(
            "room:{}:{}:{}",
            room.room_id,
            lease.member_id(),
            prepared.nonce
        ),
    )
    .await?;
    result["receipt_revision"] = json!(revision.0);
    result["duplicate"] = json!(false);
    prepared.check_result = Some(result.clone());
    sqlx::query("UPDATE room_agent_tools SET prepared_json=?1 WHERE room_id=?2 AND member_id=?3 AND turn_id=?4 AND tool_id=?5")
        .bind(serde_json::to_string(&prepared).map_err(|_|repository_error("room_check_write_failed"))?).bind(room.room_id.to_string()).bind(lease.member_id().to_string()).bind(turn.to_string()).bind(tool).execute(&mut *tx).await.map_err(|_|repository_error("room_check_write_failed"))?;
    sqlx::query("INSERT INTO room_changes(room_id,kind,payload_json) VALUES(?1,'state',?2)")
        .bind(room.room_id.to_string())
        .bind(
            json!({"operation":"skill_check","member_id":lease.member_id(),"receipt":result})
                .to_string(),
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| repository_error("room_check_write_failed"))?;
    tx.commit()
        .await
        .map_err(|_| repository_error("room_check_commit_failed"))?;
    Ok(result)
}

pub(crate) fn validate_receipt(
    result: &Value,
    owner: Uuid,
    check: &app_application::hero_checks::HeroSkillCheck,
) -> Result<(), RepositoryError> {
    if !result.is_object()
        || result["character_id"] != json!(owner)
        || result["skill"] != json!(check.skill)
        || result["total"].as_i64().is_none()
        || result["modifier"].as_i64().is_none()
        || result["receipt_revision"].as_u64().is_none()
        || result["rolls"].as_array().is_none_or(|rolls| {
            rolls.is_empty()
                || rolls.len() > 2
                || rolls
                    .iter()
                    .any(|roll| roll.as_i64().is_none_or(|roll| !(1..=20).contains(&roll)))
        })
    {
        return Err(repository_error("room_check_receipt_invalid"));
    }
    Ok(())
}
