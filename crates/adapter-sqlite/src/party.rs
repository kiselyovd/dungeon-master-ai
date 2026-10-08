use app_application::{
    models::{
        identity::{CampaignId, SessionId},
        party::{EnrollPartyCharacter, InitializeParty, PartyRoster, PartyState},
    },
    ports::repositories::{PartyRepository, RepositoryError},
};
use serde_json::Value;
use sqlx::{Row, Sqlite, Transaction};
use std::collections::BTreeMap;
use uuid::Uuid;
fn error(code: &'static str) -> RepositoryError {
    RepositoryError::Operation {
        operation: "party",
        code,
    }
}

/// None preserves the trusted pre-room party API. Once a campaign has room history,
/// only the current room's active members participate; closure leaves host-only solo.
pub(crate) async fn active_members(
    tx: &mut Transaction<'_, Sqlite>,
    campaign: CampaignId,
    session: SessionId,
) -> Result<Option<std::collections::BTreeSet<Uuid>>, RepositoryError> {
    let room: Option<String> = sqlx::query_scalar(
        "SELECT room_id FROM coop_rooms WHERE campaign_id=?1 AND session_id=?2 AND active=1",
    )
    .bind(campaign.0.to_string())
    .bind(session.0.to_string())
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| error("party_room_read_failed"))?;
    if let Some(id) = room {
        let id = Uuid::parse_str(&id).map_err(|_| error("party_room_invalid"))?;
        let room = crate::rooms::read(tx, id).await?;
        if !room.active || room.campaign_id != campaign || room.session_id != session {
            return Err(error("party_room_invalid"));
        }
        return Ok(Some(
            room.members
                .into_iter()
                .filter(|m| m.active)
                .map(|m| m.member_id)
                .collect(),
        ));
    }
    let history: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM coop_rooms WHERE campaign_id=?1)")
            .bind(campaign.0.to_string())
            .fetch_one(&mut **tx)
            .await
            .map_err(|_| error("party_room_read_failed"))?;
    Ok(history.then(Default::default))
}

pub(crate) async fn persist_sheet(
    tx: &mut Transaction<'_, Sqlite>,
    campaign: CampaignId,
    character: Uuid,
    sheet: Value,
) -> Result<(), RepositoryError> {
    let mut sheet = sheet;
    app_application::defense::normalize(&mut sheet).map_err(error)?;
    if !app_application::models::campaign_state::within_character_budget(&sheet) {
        return Err(error("character_too_large"));
    }
    if !app_application::models::campaign_state::valid_character_document(&sheet)
        || !app_application::models::campaign_state::within_character_budget(&sheet)
    {
        return Err(error("party_character_invalid"));
    }
    let root: Option<String> =
        sqlx::query_scalar("SELECT character_id FROM campaign_characters WHERE campaign_id=?1")
            .bind(campaign.0.to_string())
            .fetch_optional(&mut **tx)
            .await
            .map_err(|_| error("party_read_failed"))?;
    if root.as_deref() == Some(character.to_string().as_str()) {
        return crate::campaign_state::persist_character(tx, campaign, character, sheet).await;
    }
    let updated = sqlx::query(
        "UPDATE party_characters SET character_json=?3 WHERE campaign_id=?1 AND character_id=?2",
    )
    .bind(campaign.0.to_string())
    .bind(character.to_string())
    .bind(sheet.to_string())
    .execute(&mut **tx)
    .await
    .map_err(|_| error("party_write_failed"))?;
    if updated.rows_affected() != 1 {
        return Err(error("party_binding_rejected"));
    }
    Ok(())
}

pub(crate) async fn load(
    tx: &mut Transaction<'_, Sqlite>,
    campaign_id: CampaignId,
) -> Result<Option<PartyState>, RepositoryError> {
    let host: Option<String> =
        sqlx::query_scalar("SELECT host_member_id FROM campaign_parties WHERE campaign_id=?1")
            .bind(campaign_id.0.to_string())
            .fetch_optional(&mut **tx)
            .await
            .map_err(|_| error("party_read_failed"))?;
    let Some(host) = host else { return Ok(None) };
    let current = crate::campaign_state::load_in_transaction(tx, campaign_id).await?;
    let host_character = current
        .character_id
        .ok_or_else(|| error("party_host_character_required"))?;
    let mut roster = PartyRoster::new(
        Uuid::parse_str(&host).map_err(|_| error("party_identity_invalid"))?,
        host_character,
    )
    .map_err(|_| error("party_identity_invalid"))?;
    let mut characters = BTreeMap::from([(host_character, current.projection.character)]);
    let rows=sqlx::query("SELECT member_id,character_id,character_json FROM party_characters WHERE campaign_id=?1 ORDER BY rowid").bind(campaign_id.0.to_string()).fetch_all(&mut **tx).await.map_err(|_|error("party_read_failed"))?;
    for row in rows {
        let member = Uuid::parse_str(&row.get::<String, _>("member_id"))
            .map_err(|_| error("party_identity_invalid"))?;
        let character = Uuid::parse_str(&row.get::<String, _>("character_id"))
            .map_err(|_| error("party_identity_invalid"))?;
        let mut sheet: Value = serde_json::from_str(&row.get::<String, _>("character_json"))
            .map_err(|_| error("party_character_invalid"))?;
        app_application::defense::normalize_for_read(&mut sheet).map_err(error)?;
        if !app_application::models::campaign_state::valid_character_document(&sheet)
            || !app_application::models::campaign_state::within_character_budget(&sheet)
        {
            return Err(error("party_character_invalid"));
        }
        roster
            .join(member, character)
            .map_err(|_| error("party_identity_invalid"))?;
        characters.insert(character, sheet);
    }
    Ok(Some(PartyState {
        campaign_id,
        session_id: current.session_id,
        revision: current.campaign.revision,
        roster,
        characters,
    }))
}
async fn is_replay(
    tx: &mut Transaction<'_, Sqlite>,
    campaign: CampaignId,
    id: &str,
    request: &str,
) -> Result<bool, RepositoryError> {
    let old:Option<String>=sqlx::query_scalar("SELECT request_json FROM party_operation_receipts WHERE campaign_id=?1 AND causation_id=?2").bind(campaign.0.to_string()).bind(id).fetch_optional(&mut **tx).await.map_err(|_|error("party_receipt_read_failed"))?;
    match old {
        Some(old) if old != request => Err(error("party_causation_mismatch")),
        Some(_) => Ok(true),
        None => Ok(false),
    }
}
async fn receipt(
    tx: &mut Transaction<'_, Sqlite>,
    campaign: CampaignId,
    id: &str,
    request: &str,
) -> Result<(), RepositoryError> {
    sqlx::query("INSERT INTO party_operation_receipts(campaign_id,causation_id,request_json) VALUES(?1,?2,?3)").bind(campaign.0.to_string()).bind(id).bind(request).execute(&mut **tx).await.map_err(|_|error("party_receipt_write_failed"))?;
    Ok(())
}
fn valid_causation(id: &str) -> bool {
    !id.trim().is_empty() && id.len() <= 256
}

pub(crate) async fn capture_snapshot(
    tx: &mut Transaction<'_, Sqlite>,
    campaign_id: CampaignId,
    host: &Value,
) -> Result<Option<app_application::models::party::PartySnapshot>, RepositoryError> {
    use app_application::models::party::{PartyCharacterSnapshot, PartySnapshot};
    let leader: Option<String> =
        sqlx::query_scalar("SELECT host_member_id FROM campaign_parties WHERE campaign_id=?1")
            .bind(campaign_id.0.to_string())
            .fetch_optional(&mut **tx)
            .await
            .map_err(|_| error("party_read_failed"))?;
    let Some(leader) = leader else {
        return Ok(None);
    };
    let rows=sqlx::query("SELECT member_id,character_id,character_json FROM party_characters WHERE campaign_id=?1 ORDER BY rowid").bind(campaign_id.0.to_string()).fetch_all(&mut **tx).await.map_err(|_|error("party_read_failed"))?;
    if rows.is_empty() && host.get("characterId").is_none() {
        return Ok(None);
    }
    let mut members = Vec::new();
    for row in rows {
        members.push(PartyCharacterSnapshot {
            member_id: Uuid::parse_str(&row.get::<String, _>("member_id"))
                .map_err(|_| error("party_identity_invalid"))?,
            character_id: Uuid::parse_str(&row.get::<String, _>("character_id"))
                .map_err(|_| error("party_identity_invalid"))?,
            character: serde_json::from_str(&row.get::<String, _>("character_json"))
                .map_err(|_| error("party_character_invalid"))?,
        });
    }
    let snapshot = PartySnapshot {
        host_member_id: Uuid::parse_str(&leader).map_err(|_| error("party_identity_invalid"))?,
        members,
    };
    snapshot
        .validate(host)
        .map_err(|_| error("party_snapshot_invalid"))?;
    Ok(Some(snapshot))
}

pub(crate) async fn restore_snapshot(
    tx: &mut Transaction<'_, Sqlite>,
    campaign_id: CampaignId,
    snapshot: Option<&app_application::models::party::PartySnapshot>,
    host: &Value,
) -> Result<(), RepositoryError> {
    use std::collections::BTreeSet;
    let leader: Option<String> =
        sqlx::query_scalar("SELECT host_member_id FROM campaign_parties WHERE campaign_id=?1")
            .bind(campaign_id.0.to_string())
            .fetch_optional(&mut **tx)
            .await
            .map_err(|_| error("party_read_failed"))?;
    let rows =
        sqlx::query("SELECT member_id,character_id FROM party_characters WHERE campaign_id=?1")
            .bind(campaign_id.0.to_string())
            .fetch_all(&mut **tx)
            .await
            .map_err(|_| error("party_read_failed"))?;
    let Some(snapshot) = snapshot else {
        return if rows.is_empty() {
            Ok(())
        } else {
            Err(error("party_snapshot_roster_mismatch"))
        };
    };
    if leader.as_deref() != Some(snapshot.host_member_id.to_string().as_str()) {
        return Err(error("party_snapshot_roster_mismatch"));
    }
    snapshot
        .validate(host)
        .map_err(|_| error("party_snapshot_invalid"))?;
    let current: BTreeSet<_> = rows
        .iter()
        .map(|row| {
            (
                row.get::<String, _>("member_id"),
                row.get::<String, _>("character_id"),
            )
        })
        .collect();
    let saved: BTreeSet<_> = snapshot
        .members
        .iter()
        .map(|member| {
            (
                member.member_id.to_string(),
                member.character_id.to_string(),
            )
        })
        .collect();
    if current != saved {
        return Err(error("party_snapshot_roster_mismatch"));
    }
    for member in &snapshot.members {
        let updated=sqlx::query("UPDATE party_characters SET character_json=?4 WHERE campaign_id=?1 AND member_id=?2 AND character_id=?3").bind(campaign_id.0.to_string()).bind(member.member_id.to_string()).bind(member.character_id.to_string()).bind(member.character.to_string()).execute(&mut **tx).await.map_err(|_|error("party_restore_failed"))?;
        if updated.rows_affected() != 1 {
            return Err(error("party_restore_failed"));
        }
    }
    Ok(())
}

#[async_trait::async_trait]
impl PartyRepository for crate::SqliteStore {
    async fn party(&self, campaign_id: CampaignId) -> Result<Option<PartyState>, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| error("party_begin_failed"))?;
        let state = load(&mut tx, campaign_id).await?;
        tx.commit()
            .await
            .map_err(|_| error("party_commit_failed"))?;
        Ok(state)
    }
    async fn initialize_party(
        &self,
        command: InitializeParty,
    ) -> Result<PartyState, RepositoryError> {
        if !valid_causation(&command.causation_id) {
            return Err(error("party_command_invalid"));
        }
        let request =
            serde_json::to_string(&command).map_err(|_| error("party_command_invalid"))?;
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| error("party_begin_failed"))?;
        let current =
            crate::campaign_state::load_in_transaction(&mut tx, command.campaign_id).await?;
        if current.session_id != command.session_id {
            return Err(error("party_session_mismatch"));
        }
        current
            .character_id
            .ok_or_else(|| error("party_host_character_required"))?;
        if !is_replay(
            &mut tx,
            command.campaign_id,
            &command.causation_id,
            &request,
        )
        .await?
        {
            if current.campaign.revision != command.expected_revision {
                return Err(RepositoryError::Conflict {
                    expected: command.expected_revision,
                    actual: current.campaign.revision,
                });
            }
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM campaign_parties WHERE campaign_id=?1)",
            )
            .bind(command.campaign_id.0.to_string())
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| error("party_read_failed"))?;
            if !exists {
                sqlx::query(
                    "INSERT INTO campaign_parties(campaign_id,host_member_id) VALUES(?1,?2)",
                )
                .bind(command.campaign_id.0.to_string())
                .bind(Uuid::new_v4().to_string())
                .execute(&mut *tx)
                .await
                .map_err(|_| error("party_write_failed"))?;
                crate::campaigns::advance_session_revision(
                    &mut tx,
                    command.campaign_id,
                    command.session_id,
                    command.expected_revision,
                    "party_initialized",
                    &command.causation_id,
                )
                .await?;
            }
            receipt(
                &mut tx,
                command.campaign_id,
                &command.causation_id,
                &request,
            )
            .await?;
        }
        let state = load(&mut tx, command.campaign_id)
            .await?
            .ok_or_else(|| error("party_missing"))?;
        tx.commit()
            .await
            .map_err(|_| error("party_commit_failed"))?;
        Ok(state)
    }
    async fn enroll_party_character(
        &self,
        command: EnrollPartyCharacter,
    ) -> Result<PartyState, RepositoryError> {
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| error("party_begin_failed"))?;
        let state = enroll_in_transaction(&mut tx, command).await?;
        tx.commit()
            .await
            .map_err(|_| error("party_commit_failed"))?;
        Ok(state)
    }
}

pub(crate) async fn enroll_in_transaction(
    tx: &mut Transaction<'_, Sqlite>,
    command: EnrollPartyCharacter,
) -> Result<PartyState, RepositoryError> {
    use app_application::models::campaign_state::{
        valid_character_document, within_character_budget,
    };
    if !valid_causation(&command.causation_id)
        || !valid_character_document(&command.character)
        || !within_character_budget(&command.character)
    {
        return Err(error("party_character_invalid"));
    }
    let request = serde_json::to_string(&command).map_err(|_| error("party_command_invalid"))?;
    let mut current = load(tx, command.campaign_id)
        .await?
        .ok_or_else(|| error("party_missing"))?;
    if current.session_id != command.session_id {
        return Err(error("party_session_mismatch"));
    }
    if command.verified_host_member_id != current.roster.host_member_id {
        return Err(error("party_host_required"));
    }
    if !is_replay(tx, command.campaign_id, &command.causation_id, &request).await? {
        if current.revision != command.expected_revision {
            return Err(RepositoryError::Conflict {
                expected: command.expected_revision,
                actual: current.revision,
            });
        }
        let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM combat_encounters WHERE session_id=?1 AND ended_at IS NULL)").bind(command.session_id.0.to_string()).fetch_one(&mut **tx).await.map_err(|_|error("party_read_failed"))?;
        if active {
            return Err(error("party_combat_active"));
        }
        current
            .roster
            .join(command.member_id, command.character_id)
            .map_err(|_| error("party_binding_rejected"))?;
        if current.characters.contains_key(&command.character_id) {
            return Err(error("party_character_already_enrolled"));
        }
        let mut sheet = command.character.clone();
        app_application::vitality::strip_client_state(&mut sheet);
        app_application::defense::normalize(&mut sheet).map_err(error)?;
        sheet["characterId"] = serde_json::json!(command.character_id);
        if !within_character_budget(&sheet) {
            return Err(error("character_too_large"));
        }
        sqlx::query("INSERT INTO party_characters(campaign_id,member_id,character_id,character_json) VALUES(?1,?2,?3,?4)").bind(command.campaign_id.0.to_string()).bind(command.member_id.to_string()).bind(command.character_id.to_string()).bind(sheet.to_string()).execute(&mut **tx).await.map_err(|_|error("party_write_failed"))?;
        crate::campaigns::advance_session_revision(
            tx,
            command.campaign_id,
            command.session_id,
            command.expected_revision,
            "party_character_enrolled",
            &command.causation_id,
        )
        .await?;
        receipt(tx, command.campaign_id, &command.causation_id, &request).await?;
    }
    let state = load(tx, command.campaign_id)
        .await?
        .ok_or_else(|| error("party_missing"))?;
    Ok(state)
}
