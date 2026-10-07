use crate::messages::repository_error;
use app_application::{
    models::identity::{CampaignId, SessionId},
    ports::repositories::RepositoryError,
};
use sqlx::{Row, Sqlite, Transaction};
use uuid::Uuid;

pub(crate) async fn passage(
    tx: &mut Transaction<'_, Sqlite>,
    session: SessionId,
    previous: u64,
    now: u64,
    engine: &mut app_application::recovery::RecoveryEngine,
) -> Result<Vec<app_application::models::room_recovery::RecoveryDeathSave>, RepositoryError> {
    if now < previous {
        return Err(repository_error("adventuring_clock_invalid"));
    }
    // Active combat owns its own projection and death-turn timing.
    if crate::rest::active_combat(tx, session).await? {
        return Ok(vec![]);
    }
    let campaign: String = sqlx::query_scalar("SELECT campaign_id FROM sessions WHERE id=?1")
        .bind(session.0.to_string())
        .fetch_one(&mut **tx)
        .await
        .map_err(|_| repository_error("recovery_read_failed"))?;
    let campaign = CampaignId(
        Uuid::parse_str(&campaign).map_err(|_| repository_error("recovery_read_failed"))?,
    );
    let rows=sqlx::query("SELECT character_id,character_json FROM campaign_characters WHERE campaign_id=?1 UNION ALL SELECT pc.character_id,pc.character_json FROM party_characters pc WHERE pc.campaign_id=?1 AND EXISTS(SELECT 1 FROM coop_rooms r,json_each(r.record_json,'$.members') m WHERE r.active=1 AND r.session_id=?2 AND r.campaign_id=pc.campaign_id AND json_extract(m.value,'$.member_id')=pc.member_id AND json_extract(m.value,'$.active')=1) ORDER BY character_id")
        .bind(campaign.0.to_string()).bind(session.0.to_string()).fetch_all(&mut **tx).await.map_err(|_|repository_error("recovery_read_failed"))?;
    let mut events = vec![];
    for row in rows {
        let id = Uuid::parse_str(&row.get::<String, _>("character_id"))
            .map_err(|_| repository_error("recovery_read_failed"))?;
        let mut sheet: serde_json::Value =
            serde_json::from_str(&row.get::<String, _>("character_json"))
                .map_err(|_| repository_error("recovery_read_failed"))?;
        events.extend(
            engine
                .passage(&mut sheet, id, previous, now)
                .map_err(repository_error)?,
        );
        crate::party::persist_sheet(tx, campaign, id, sheet).await?;
    }
    if !events.is_empty() {
        sqlx::query("INSERT INTO room_changes(room_id,kind,payload_json) SELECT room_id,'state',?1 FROM coop_rooms WHERE session_id=?2 AND active=1")
            .bind(serde_json::json!({"operation":"death_saves","results":events}).to_string()).bind(session.0.to_string())
            .execute(&mut **tx).await.map_err(|_|repository_error("recovery_publish_failed"))?;
    }
    Ok(events)
}
