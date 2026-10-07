use crate::{
    models::{identity::CampaignId, solo_recovery::*},
    ports::repositories::{RepositoryError, SoloRecoveryRepository},
};
use std::sync::Arc;
pub struct RecoverSolo<R>(Arc<R>);
impl<R: SoloRecoveryRepository> RecoverSolo<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self(repository)
    }
    pub async fn status(&self, id: CampaignId) -> Result<SoloRecoveryStatus, RepositoryError> {
        self.0.recovery_status(id).await
    }
    pub async fn execute(
        &self,
        command: SoloRecoveryCommand,
    ) -> Result<SoloRecoveryResult, RepositoryError> {
        if command.campaign_id.0.is_nil()
            || command.session_id.0.is_nil()
            || command.character_id.is_nil()
            || command.causation_id.trim().is_empty()
            || command.causation_id.len() > 64
            || command.causation_id.chars().any(char::is_control)
        {
            return Err(RepositoryError::Operation {
                operation: "solo_recovery",
                code: "rest_intention_invalid",
            });
        }
        self.0
            .recover_solo(command, uuid::Uuid::new_v4().as_u128() as u64)
            .await
    }
}
pub fn status_for_state(
    state: &crate::models::campaign_state::CampaignSessionState,
    combat_active: bool,
) -> Result<SoloRecoveryStatus, &'static str> {
    use crate::{
        models::room_recovery::{PublicHeroRecovery, RecoveryIntent, RecoveryStatus},
        vitality,
    };
    let id = state.character_id.ok_or("rest_character_required")?;
    let sheet = &state.projection.character;
    let hp = i32::try_from(sheet["hp"].as_i64().ok_or("hero_vitality_invalid")?)
        .map_err(|_| "hero_vitality_invalid")?;
    let max_hp = i32::try_from(sheet["hpMax"].as_i64().ok_or("hero_vitality_invalid")?)
        .map_err(|_| "hero_vitality_invalid")?;
    let vitality = vitality::read(sheet, id)?;
    let dead = vitality.as_ref().is_some_and(|v| v.is_dead);
    let stable = vitality.as_ref().is_some_and(|v| v.death_saves.stable);
    let safe = crate::recovery::safe(state);
    let mut actions = vec![];
    if !combat_active && hp == 0 && !dead {
        if stable && safe {
            actions.push(RecoveryIntent::Wait {
                target_character_id: id,
            });
        } else if !stable {
            actions.push(RecoveryIntent::DeathSave {});
        }
    }
    Ok(SoloRecoveryStatus {
        campaign_id: state.campaign.id.0,
        session_id: state.session_id.0,
        character_id: id,
        revision: state.campaign.revision.0,
        hp,
        max_hp,
        safe,
        combat_active,
        elapsed_seconds: state
            .projection
            .adventuring
            .as_ref()
            .map_or(0, |s| s.elapsed_seconds),
        recovery: PublicHeroRecovery {
            character_id: id,
            status: if dead {
                RecoveryStatus::Dead
            } else if hp > 0 {
                RecoveryStatus::Healthy
            } else if stable {
                RecoveryStatus::Stable
            } else {
                RecoveryStatus::Dying
            },
            successes: vitality.as_ref().map_or(0, |v| v.death_saves.successes),
            failures: vitality.as_ref().map_or(0, |v| v.death_saves.failures),
            ready_at_seconds: vitality.and_then(|v| v.natural_recovery.map(|t| t.ready_at_seconds)),
            actions,
        },
    })
}
