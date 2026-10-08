use crate::{messages::repository_error, SqliteStore};
use app_application::hero_checks::HeroCheckSheetRepository;
use serde_json::Value;
use uuid::Uuid;

#[async_trait::async_trait]
impl HeroCheckSheetRepository for SqliteStore {
    async fn check_sheet(
        &self,
        campaign: Uuid,
        session: Uuid,
        character: Uuid,
    ) -> Result<Value, app_application::ports::repositories::RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| repository_error("hero_check_read_failed"))?;
        let current: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sessions s WHERE s.id=?1 AND s.campaign_id=?2 AND s.id=(SELECT latest.id FROM sessions latest WHERE latest.campaign_id=?2 ORDER BY CAST(latest.created_at AS INTEGER) DESC,latest.id DESC LIMIT 1))")
            .bind(session.to_string()).bind(campaign.to_string()).fetch_one(&mut *tx).await.map_err(|_|repository_error("hero_check_read_failed"))?;
        if !current {
            return Err(repository_error("hero_check_identity_mismatch"));
        }
        let rows: Vec<String> = sqlx::query_scalar("SELECT character_json FROM campaign_characters WHERE campaign_id=?1 AND character_id=?3 UNION ALL SELECT pc.character_json FROM party_characters pc WHERE pc.campaign_id=?1 AND pc.character_id=?3 AND EXISTS(SELECT 1 FROM coop_rooms r,json_each(r.record_json,'$.members') m WHERE r.active=1 AND r.session_id=?2 AND r.campaign_id=pc.campaign_id AND json_extract(m.value,'$.member_id')=pc.member_id AND json_extract(m.value,'$.active')=1)")
            .bind(campaign.to_string()).bind(session.to_string()).bind(character.to_string()).fetch_all(&mut *tx).await.map_err(|_|repository_error("hero_check_read_failed"))?;
        if rows.len() != 1 {
            return Err(repository_error("hero_check_identity_mismatch"));
        }
        let sheet: Value = serde_json::from_str(&rows[0])
            .map_err(|_| repository_error("hero_check_read_failed"))?;
        if !app_application::models::campaign_state::valid_character_document(&sheet)
            || !app_application::models::campaign_state::within_character_budget(&sheet)
        {
            return Err(repository_error("hero_check_invalid"));
        }
        tx.commit()
            .await
            .map_err(|_| repository_error("hero_check_read_failed"))?;
        Ok(sheet)
    }
}
