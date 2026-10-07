use std::sync::Arc;

use app_domain::combat::resolver::{CombatAction, CombatResolver, ValidationError};
use app_domain::rng::SeededRng;
use thiserror::Error;

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
    #[error("combat request identity or state is invalid")]
    InvalidState,
    #[error("combat request ID was reused for another intention")]
    RequestMismatch,
    #[error("stale combat revision: expected {expected}, actual {actual}")]
    StaleRevision { expected: u64, actual: u64 },
    #[error("combat action rejected: {0}")]
    Rejected(#[from] ValidationError),
    #[error("combat persistence failed")]
    Persistence,
}

pub struct ResolveCombatAction {
    repository: Arc<dyn CombatRepository>,
}

impl ResolveCombatAction {
    pub fn new(repository: Arc<dyn CombatRepository>) -> Self {
        Self { repository }
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
        if command.request_id.trim().is_empty() || command.request_id.len() > 256 {
            return Err(ResolveCombatError::InvalidState);
        }
        // RNG is an execution input, not a client's immutable intention. A
        // retry may receive a fresh server seed but must reuse recorded rolls.
        let signature = serde_json::to_string(&serde_json::json!({
            "encounter_id":command.encounter_id,
            "expected_revision":command.expected_revision,
            "action":command.action,
        }))
        .map_err(|_| ResolveCombatError::InvalidState)?;
        if let Some(projection) = self
            .repository
            .find_action(command.encounter_id, &command.request_id, &signature)
            .await
            .map_err(repository_error)?
        {
            return Ok(ResolveCombatResult {
                projection,
                duplicate: true,
            });
        }

        let current = self
            .repository
            .get(command.encounter_id)
            .await
            .map_err(|_| ResolveCombatError::Persistence)?;
        let projection = match current {
            Some(projection) => projection,
            None => {
                if let Some(projection) = self
                    .repository
                    .find_action(command.encounter_id, &command.request_id, &signature)
                    .await
                    .map_err(repository_error)?
                {
                    return Ok(ResolveCombatResult {
                        projection,
                        duplicate: true,
                    });
                }
                return Err(ResolveCombatError::NotFound);
            }
        };
        if projection.revision != command.expected_revision {
            // An exact simultaneous request may have committed after the
            // initial receipt read but before this projection read.
            if let Some(projection) = self
                .repository
                .find_action(command.encounter_id, &command.request_id, &signature)
                .await
                .map_err(repository_error)?
            {
                return Ok(ResolveCombatResult {
                    projection,
                    duplicate: true,
                });
            }
            return Err(ResolveCombatError::StaleRevision {
                expected: command.expected_revision,
                actual: projection.revision,
            });
        }

        let projection = prepare_action(projection, command.action, command.rng_seed)?;

        let committed = self
            .repository
            .commit_action(
                command.expected_revision,
                projection,
                &command.request_id,
                &signature,
            )
            .await
            .map_err(repository_error)?;
        tracing::info!(
            revision = committed.projection.revision,
            duplicate = committed.duplicate,
            "combat action committed"
        );
        Ok(ResolveCombatResult {
            projection: committed.projection,
            duplicate: committed.duplicate,
        })
    }
}

/// Pure mechanics shared by solo and transaction-scoped authenticated rooms.
pub fn prepare_action(
    mut projection: CombatProjection,
    action: CombatActionCommand,
    rng_seed: u64,
) -> Result<CombatProjection, ResolveCombatError> {
    if !projection.snapshot.active {
        return Err(ResolveCombatError::InvalidState);
    }
    let (combatants, order) =
        resolver_state(&projection.snapshot).ok_or(ResolveCombatError::InvalidState)?;
    let mut resolver = CombatResolver::new(combatants, order, SeededRng::from_seed(rng_seed));
    if let Some(authored) = &projection.authored {
        resolver
            .set_map(authored.map.clone())
            .map_err(|_| ResolveCombatError::InvalidState)?;
    }
    resolver.set_turn_boundary(
        projection.authored.as_ref().map(|_| 4),
        projection.authored.is_some(),
    );
    let retreat = matches!(&action, CombatActionCommand::Retreat { .. });
    let action = match action {
        CombatActionCommand::Retreat { combatant } => CombatAction::Retreat { combatant },
        CombatActionCommand::Spell {
            caster,
            spell_id,
            targets,
            slot_level,
        } => CombatAction::Spell {
            caster,
            spell_id,
            targets,
            slot_level,
        },
        CombatActionCommand::WeaponAttack {
            attacker,
            target,
            weapon_id,
            mode,
            two_handed,
        } => CombatAction::WeaponAttack {
            attacker,
            target,
            weapon_id,
            mode,
            two_handed,
        },
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
    let first = resolver.resolve(action)?;
    let mut events = vec![first];
    super::automation::settle_npcs_until(
        &mut resolver,
        &mut events,
        projection.authored.as_ref().map(|_| 4),
    )
    .map_err(|_| ResolveCombatError::InvalidState)?;
    replace_resolved_state(
        &mut projection,
        resolver.combatants,
        resolver.order,
        Default::default(),
    );
    projection.events = events;
    super::automation::finish_if_resolved(&mut projection);
    super::authored::finalize(&mut projection, retreat);

    Ok(projection)
}

fn repository_error(error: RepositoryError) -> ResolveCombatError {
    match error {
        RepositoryError::RevisionConflict { expected, actual } => {
            ResolveCombatError::StaleRevision { expected, actual }
        }
        RepositoryError::Operation {
            code: "combat_request_mismatch",
            ..
        } => ResolveCombatError::RequestMismatch,
        _ => ResolveCombatError::Persistence,
    }
}
