use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnPhase {
    /// Phase 1: rules engine validates the proposed action.
    Validate,
    /// Phase 2: engine resolves all dice, damage, conditions.
    Resolve,
    /// Phase 3: LLM narrates ResultEvents via SSE. Engine does not mutate here.
    Narrate,
    /// Phase 4: persist Snapshot to SQLite.
    Persist,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnState {
    pub phase: TurnPhase,
    pub complete: bool,
}

impl TurnState {
    pub fn new() -> Self {
        Self { phase: TurnPhase::Validate, complete: false }
    }
}

impl Default for TurnState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TurnFsmError {
    #[error("expected phase {expected:?} but current phase is {current:?}")]
    WrongPhase { current: TurnPhase, expected: TurnPhase },
    #[error("turn is already complete")]
    AlreadyComplete,
}

/// Advance from the expected phase to the next phase.
/// Returns `Err` if the current phase does not match `expected_current`.
pub fn advance_phase(
    state: &mut TurnState,
    expected_current: TurnPhase,
) -> Result<(), TurnFsmError> {
    if state.complete {
        return Err(TurnFsmError::AlreadyComplete);
    }
    if state.phase != expected_current {
        return Err(TurnFsmError::WrongPhase {
            current: state.phase,
            expected: expected_current,
        });
    }
    state.phase = match state.phase {
        TurnPhase::Validate => TurnPhase::Resolve,
        TurnPhase::Resolve => TurnPhase::Narrate,
        TurnPhase::Narrate => TurnPhase::Persist,
        TurnPhase::Persist => {
            state.complete = true;
            return Ok(());
        }
    };
    Ok(())
}
