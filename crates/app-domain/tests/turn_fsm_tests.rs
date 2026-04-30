use app_domain::combat::turn_fsm::{TurnPhase, TurnState, advance_phase};

#[test]
fn turn_starts_at_validate() {
    let state = TurnState::new();
    assert_eq!(state.phase, TurnPhase::Validate);
}

#[test]
fn validate_advances_to_resolve() {
    let mut state = TurnState::new();
    advance_phase(&mut state, TurnPhase::Validate).expect("advance ok");
    assert_eq!(state.phase, TurnPhase::Resolve);
}

#[test]
fn resolve_advances_to_narrate() {
    let mut state = TurnState::new();
    state.phase = TurnPhase::Resolve;
    advance_phase(&mut state, TurnPhase::Resolve).expect("advance ok");
    assert_eq!(state.phase, TurnPhase::Narrate);
}

#[test]
fn narrate_advances_to_persist() {
    let mut state = TurnState::new();
    state.phase = TurnPhase::Narrate;
    advance_phase(&mut state, TurnPhase::Narrate).expect("advance ok");
    assert_eq!(state.phase, TurnPhase::Persist);
}

#[test]
fn wrong_phase_transition_is_error() {
    let mut state = TurnState::new();
    // Cannot advance from Validate by claiming current phase is Resolve.
    let err = advance_phase(&mut state, TurnPhase::Resolve);
    assert!(err.is_err(), "wrong phase must be rejected");
}

#[test]
fn persist_phase_marks_turn_complete() {
    let mut state = TurnState::new();
    state.phase = TurnPhase::Persist;
    advance_phase(&mut state, TurnPhase::Persist).expect("advance to complete");
    assert!(state.complete);
}
