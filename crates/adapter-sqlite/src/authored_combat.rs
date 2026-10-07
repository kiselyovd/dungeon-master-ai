use crate::{adventure, messages::repository_error};
use app_application::models::{
    adventure::{AdventureProjection, DefenseOutcome},
    combat::CombatProjection,
    identity::{CampaignId, SessionId},
};
use app_application::ports::repositories::RepositoryError;
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

pub(crate) async fn resolve(
    tx: &mut Transaction<'_, Sqlite>,
    session_id: Uuid,
    projection: &mut CombatProjection,
) -> Result<(), RepositoryError> {
    if projection.authored.is_none() {
        return Ok(());
    }
    let campaign: String = sqlx::query_scalar("SELECT campaign_id FROM sessions WHERE id=?1")
        .bind(session_id.to_string())
        .fetch_one(&mut **tx)
        .await
        .map_err(|_| repository_error("read_authored_campaign"))?;
    let id = CampaignId(
        Uuid::parse_str(&campaign).map_err(|_| repository_error("authored_campaign_invalid"))?,
    );
    let current = adventure::read(tx, id)
        .await?
        .ok_or_else(|| repository_error("authored_adventure_missing"))?;
    if current.state.defense_encounter_id != Some(projection.encounter_id) {
        return Err(repository_error("authored_encounter_mismatch"));
    }
    let outcome = app_application::combat::authored::finalize(projection, false);
    if let Some(outcome) = outcome {
        if current.state.defense != Some(DefenseOutcome::Active) {
            if current.state.defense != Some(outcome) {
                return Err(repository_error("authored_outcome_mismatch"));
            }
            projection.adventure = Some(current);
            return Ok(());
        }
        let resolved = current
            .state
            .resolve_defense(projection.encounter_id, outcome)
            .map_err(|_| repository_error("authored_outcome_invalid"))?;
        if outcome == DefenseOutcome::Defeated {
            app_application::combat::authored::recover_defeated_hero(projection)
                .map_err(repository_error)?;
            crate::rest::advance_clock(tx, SessionId(session_id), 3600).await?;
        }
        adventure::persist(tx, id, Some(&resolved)).await?;
        projection.adventure = Some(AdventureProjection::from(resolved));
    } else {
        projection.adventure = Some(current);
    }
    Ok(())
}
