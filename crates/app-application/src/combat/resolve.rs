use std::collections::HashMap;
use std::sync::Arc;

use app_domain::combat::resolver::{CombatAction, CombatResolver, ValidationError};
use app_domain::rng::SeededRng;
use thiserror::Error;
use tokio::sync::Mutex;

use crate::models::combat::CombatProjection;
use crate::ports::repositories::{CombatRepository, RepositoryError};

use super::commands::{CombatActionCommand, ResolveCombatCommand};
use super::projection::{replace_resolved_state, resolver_state};

#[derive(Debug, Clone)]
pub struct ResolveCombatResult {
    pub projection: CombatProjection,
    pub duplicate: bool,
}

#[derive(Debug, Error)]
pub enum ResolveCombatError {
    #[error("combat encounter not found")]
    NotFound,
    #[error("stale combat revision: expected {expected}, actual {actual}")]
    StaleRevision { expected: u64, actual: u64 },
    #[error("combat action rejected: {0}")]
    Rejected(#[from] ValidationError),
    #[error("combat persistence failed")]
    Persistence,
}

pub struct ResolveCombatAction {
    repository: Arc<dyn CombatRepository>,
    completed: Mutex<HashMap<String, CombatProjection>>,
}

impl ResolveCombatAction {
    pub fn new(repository: Arc<dyn CombatRepository>) -> Self {
        Self {
            repository,
            completed: Mutex::new(HashMap::new()),
        }
    }

    #[tracing::instrument(skip_all, fields(
        request_id = %command.request_id,
        encounter_id = %command.encounter_id,
        expected_revision = command.expected_revision,
    ))]
    pub async fn execute(
        &self,
        command: ResolveCombatCommand,
    ) -> Result<ResolveCombatResult, ResolveCombatError> {
        let mut completed = self.completed.lock().await;
        if let Some(projection) = completed.get(&command.request_id) {
            return Ok(ResolveCombatResult {
                projection: projection.clone(),
                duplicate: true,
            });
        }

        let mut projection = self
            .repository
            .get(command.encounter_id)
            .await
            .map_err(|_| ResolveCombatError::Persistence)?
            .ok_or(ResolveCombatError::NotFound)?;
        if projection.revision != command.expected_revision {
            return Err(ResolveCombatError::StaleRevision {
                expected: command.expected_revision,
                actual: projection.revision,
            });
        }

        let (combatants, order) = resolver_state(&projection.snapshot);
        let mut resolver =
            CombatResolver::new(combatants, order, SeededRng::from_seed(command.rng_seed));
        let action = match command.action {
            CombatActionCommand::Attack {
                attacker,
                target,
                attack_modifier,
                damage_expr,
                damage_type,
            } => CombatAction::Attack {
                attacker,
                target,
                attack_modifier,
                damage_expr,
                damage_type,
            },
            CombatActionCommand::Cast { combatant } => CombatAction::Cast { combatant },
            CombatActionCommand::Move { combatant, to } => CombatAction::Move { combatant, to },
            CombatActionCommand::EndTurn { combatant } => CombatAction::EndTurn { combatant },
        };
        let events = resolver.resolve(action)?;
        replace_resolved_state(&mut projection, resolver.combatants, resolver.order, events);

        self.repository
            .compare_and_set(command.expected_revision, projection.clone())
            .await
            .map_err(|error| match error {
                RepositoryError::RevisionConflict { actual, .. } => {
                    ResolveCombatError::StaleRevision {
                        expected: command.expected_revision,
                        actual,
                    }
                }
                _ => ResolveCombatError::Persistence,
            })?;
        completed.insert(command.request_id, projection.clone());
        tracing::info!(revision = projection.revision, "combat action committed");
        Ok(ResolveCombatResult {
            projection,
            duplicate: false,
        })
    }
}
