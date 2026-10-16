use crate::SqliteStore;
use app_application::agent::hero_intentions::{
    HeroCombatIntent, HeroIntention, HeroIntentionRepository,
};
use app_application::combat::{
    commands::ResolveCombatCommand,
    resolve::{ResolveCombatAction, ResolveCombatError},
};
use app_application::models::{
    campaign_state::CampaignSessionState,
    combat::CombatProjection,
    rest::{RestCommand, RestIntent},
};
use app_application::ports::repositories::{
    CampaignStateRepository, RepositoryError, RestRepository,
};
use async_trait::async_trait;
use sqlx::Row;
use std::sync::Arc;

fn rejected(code: &'static str) -> RepositoryError {
    RepositoryError::Operation {
        operation: "hero_intention",
        code,
    }
}

async fn current<T>(
    store: &SqliteStore,
    intent: &HeroIntention<T>,
) -> Result<CampaignSessionState, RepositoryError> {
    if intent.causation_id.trim().is_empty() || intent.causation_id.len() > 256 {
        return Err(rejected("hero_intention_invalid"));
    }
    let state = store.load_current(intent.campaign_id).await?;
    if state.session_id != intent.session_id || state.character_id.is_none() {
        return Err(rejected("hero_intention_identity_mismatch"));
    }
    Ok(state)
}

#[async_trait]
impl HeroIntentionRepository for SqliteStore {
    async fn combat_as_dm(
        &self,
        intent: HeroIntention<HeroCombatIntent>,
        rng_seed: u64,
    ) -> Result<CampaignSessionState, RepositoryError> {
        let state = current(self, &intent).await?;
        // Read a recorded signature before deriving a new revision. A crash after
        // combat commit but before the agent checkpoint must not repeat the effect.
        let receipt=sqlx::query("SELECT r.signature,r.projection_json FROM combat_action_receipts r JOIN combat_encounters e ON e.id=r.encounter_id WHERE e.session_id=?1 AND r.request_id=?2")
            .bind(intent.session_id.0.to_string()).bind(&intent.causation_id).fetch_optional(&self.pool).await.map_err(|_|rejected("hero_combat_receipt_failed"))?;
        let prepared = if receipt.is_none() {
            let mut tx = self
                .pool
                .begin_with("BEGIN IMMEDIATE")
                .await
                .map_err(|_| rejected("hero_combat_prepare_failed"))?;
            let prepared = crate::human_commands::require_prepared_context(
                &mut tx,
                &intent.causation_id,
                intent.campaign_id.0,
                intent.session_id.0,
            )
            .await?;
            tx.commit()
                .await
                .map_err(|_| rejected("hero_combat_prepare_failed"))?;
            prepared
        } else {
            None
        };
        let (projection, expected_revision) = if let Some(row) = receipt {
            let projection: CombatProjection =
                serde_json::from_str(&row.get::<String, _>("projection_json"))
                    .map_err(|_| rejected("hero_combat_receipt_invalid"))?;
            let signature: serde_json::Value =
                serde_json::from_str(&row.get::<String, _>("signature"))
                    .map_err(|_| rejected("hero_combat_receipt_invalid"))?;
            let revision = signature["expected_revision"]
                .as_u64()
                .ok_or_else(|| rejected("hero_combat_receipt_invalid"))?;
            (projection, Some((revision, signature)))
        } else {
            let projection: CombatProjection = serde_json::from_value(
                state
                    .projection
                    .combat
                    .clone()
                    .ok_or_else(|| rejected("hero_combat_missing"))?,
            )
            .map_err(|_| rejected("hero_combat_invalid"))?;
            (projection, None)
        };
        let scope = projection
            .campaign_scope
            .ok_or_else(|| rejected("hero_combat_unbound"))?;
        if scope.campaign_id != intent.campaign_id.0
            || scope.session_id != intent.session_id.0
            || Some(scope.character_id) != state.character_id
        {
            return Err(rejected("hero_intention_identity_mismatch"));
        }
        let heroes: Vec<_> = projection
            .snapshot
            .combatants
            .iter()
            .filter(|actor| actor.character_id.map(|id| id.0) == state.character_id)
            .collect();
        if heroes.len() != 1 {
            return Err(rejected("hero_combat_unbound"));
        }
        let action = intent.action.for_actor(heroes[0].id);
        let revision = if let Some((revision, signature)) = expected_revision {
            if signature["action"]
                != serde_json::to_value(&action).map_err(|_| rejected("hero_intention_invalid"))?
            {
                return Err(rejected("hero_intention_mismatch"));
            }
            revision
        } else if let Some(prepared) = prepared {
            let expected = prepared
                .stamp
                .combat
                .ok_or_else(|| rejected("human_command_context_invalid"))?;
            if prepared.binding.signature
                != app_application::agent::human_commands::HumanAction::Combat(
                    intent.action.clone(),
                )
                || expected.encounter_id != projection.encounter_id
                || expected.actor_token != heroes[0].id.0
            {
                return Err(rejected("human_command_signature_mismatch"));
            }
            expected.revision
        } else {
            projection.revision
        };
        ResolveCombatAction::new(Arc::new(self.clone()))
            .execute(ResolveCombatCommand {
                request_id: intent.causation_id,
                encounter_id: projection.encounter_id,
                expected_revision: revision,
                rng_seed,
                action,
            })
            .await
            .map_err(|error| {
                rejected(match error {
                    ResolveCombatError::StaleRevision { .. } => "hero_combat_stale",
                    ResolveCombatError::RequestMismatch => "hero_intention_mismatch",
                    ResolveCombatError::Rejected(error) => {
                        app_application::combat::resolve::validation_code(&error)
                    }
                    _ => "hero_combat_failed",
                })
            })?;
        let mut current = self.load_current(intent.campaign_id).await?;
        if current.projection.combat.is_none() {
            // Durable current state omits ended encounters. Tool delivery still
            // needs a terminal projection to close the renderer without reload.
            // Receipts are historical: even an earlier Move can be replayed
            // after closure. Use the latest finished encounter's saved final
            // projection, never promote an active historical receipt.
            let raw:String=sqlx::query_scalar("SELECT projection FROM combat_encounters WHERE session_id=?1 AND ended_at IS NOT NULL ORDER BY rowid DESC LIMIT 1")
                .bind(intent.session_id.0.to_string()).fetch_one(&self.pool).await.map_err(|_|rejected("hero_combat_terminal_missing"))?;
            let mut terminal: CombatProjection =
                serde_json::from_str(&raw).map_err(|_| rejected("hero_combat_terminal_invalid"))?;
            if terminal.snapshot.active {
                return Err(rejected("hero_combat_terminal_invalid"));
            }
            if let Some(scope) = terminal.campaign_scope.as_mut() {
                scope.revision = current.campaign.revision.0;
            }
            terminal.adventure = current.projection.adventure.clone();
            for actor in &mut terminal.snapshot.combatants {
                if actor.character_id.map(|id| id.0) == current.character_id {
                    actor.current_hp = current.projection.character["hp"]
                        .as_i64()
                        .ok_or_else(|| rejected("hero_combat_invalid"))?
                        as i32;
                    actor.max_hp = current.projection.character["hpMax"]
                        .as_i64()
                        .ok_or_else(|| rejected("hero_combat_invalid"))?
                        as i32;
                    actor.ac = current.projection.character["ac"]
                        .as_i64()
                        .ok_or_else(|| rejected("hero_combat_invalid"))?
                        as i32;
                }
            }
            current.projection.combat =
                Some(serde_json::to_value(terminal).map_err(|_| rejected("hero_combat_invalid"))?);
        }
        Ok(current)
    }

    async fn rest_as_dm(
        &self,
        intent: HeroIntention<RestIntent>,
        rng_seed: u64,
    ) -> Result<CampaignSessionState, RepositoryError> {
        let state = current(self, &intent).await?;
        let character_id = state
            .character_id
            .ok_or_else(|| rejected("hero_intention_identity_mismatch"))?;
        let raw: Option<String> = sqlx::query_scalar(
            "SELECT request_json FROM rest_operations WHERE session_id=?1 AND causation_id=?2",
        )
        .bind(intent.session_id.0.to_string())
        .bind(&intent.causation_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| rejected("hero_rest_receipt_failed"))?;
        let command = if let Some(raw) = raw {
            let command: RestCommand =
                serde_json::from_str(&raw).map_err(|_| rejected("hero_rest_receipt_invalid"))?;
            if command.campaign_id != intent.campaign_id
                || command.session_id != intent.session_id
                || command.character_id != character_id
                || command.intent != intent.action
            {
                return Err(rejected("hero_intention_mismatch"));
            }
            command
        } else {
            RestCommand {
                campaign_id: intent.campaign_id,
                session_id: intent.session_id,
                character_id,
                expected_revision: state.campaign.revision,
                causation_id: intent.causation_id,
                intent: intent.action,
            }
        };
        self.rest(command, rng_seed).await?;
        self.load_current(intent.campaign_id).await
    }
}
