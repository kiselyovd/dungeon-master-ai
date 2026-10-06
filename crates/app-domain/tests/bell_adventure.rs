use app_domain::adventure::{Action, AdventureState, Ending, Fact, Location};

fn act(state: &mut AdventureState, action: Action) -> u32 {
    let result = state.apply(action).unwrap();
    *state = result.state;
    result.gold_reward
}

fn rescue(state: &mut AdventureState) {
    act(state, Action::Move(Location::Quay));
    act(state, Action::InspectQuay);
    act(state, Action::Move(Location::Archive));
    act(state, Action::LocateTikhon);
    assert_eq!(act(state, Action::RescueTikhon), 10);
    assert!(!state.available_actions().contains(&Action::RescueTikhon));
}

#[test]
fn peaceful_ending_is_reachable_without_ledger_or_forced_rolls() {
    let mut state = AdventureState::new();
    rescue(&mut state);
    act(&mut state, Action::Move(Location::Quay));
    act(&mut state, Action::Move(Location::Town));
    act(&mut state, Action::Move(Location::Shrine));
    act(&mut state, Action::ConsultLeon);
    act(&mut state, Action::Move(Location::Town));
    act(&mut state, Action::Move(Location::Warehouse));
    act(&mut state, Action::NegotiateClapper);
    act(&mut state, Action::RepairClapper);
    act(&mut state, Action::Move(Location::Town));
    act(&mut state, Action::Move(Location::Quay));
    act(&mut state, Action::Move(Location::Archive));
    act(&mut state, Action::Move(Location::Belfry));
    act(&mut state, Action::AcknowledgeEcho);
    act(&mut state, Action::RestoreWard);
    act(&mut state, Action::Move(Location::Archive));
    act(&mut state, Action::Move(Location::Quay));
    act(&mut state, Action::Move(Location::Town));
    act(&mut state, Action::Finish);
    assert_eq!(state.ending, Some(Ending::RememberedRiver));
    assert!(state.closed);
    assert!(!state.facts.contains(&Fact::LedgerPublished));
    assert!(state.available_actions().is_empty());
}

#[test]
fn seal_is_a_distinct_resolution_and_ledger_is_an_independent_choice() {
    let mut state = AdventureState::new();
    rescue(&mut state);
    act(&mut state, Action::RecoverLedger);
    act(&mut state, Action::Move(Location::Belfry));
    act(&mut state, Action::SealSluice);
    act(&mut state, Action::Move(Location::Archive));
    act(&mut state, Action::Move(Location::Quay));
    act(&mut state, Action::Move(Location::Town));
    act(&mut state, Action::PublishLedger);
    act(&mut state, Action::Finish);
    assert_eq!(state.ending, Some(Ending::ClosedCrossing));
    assert!(!state.facts.contains(&Fact::WardRestored));
    assert!(state.facts.contains(&Fact::LedgerPublished));
}

#[test]
fn unresolved_retreat_is_honest_and_remains_resumable() {
    let state = AdventureState::new().apply(Action::Finish).unwrap().state;
    assert_eq!(state.ending, Some(Ending::UnfinishedDebt));
    assert!(!state.closed);
    assert!(!state.facts.contains(&Fact::RewardGranted));
    assert!(state
        .available_actions()
        .contains(&Action::Move(Location::Quay)));
}

#[test]
fn illegal_actions_cannot_invent_resolution_or_change_state() {
    let state = AdventureState::new();
    let original = state.clone();
    for action in [
        Action::RestoreWard,
        Action::RescueTikhon,
        Action::PublishLedger,
        Action::Move(Location::Belfry),
    ] {
        assert!(state.apply(action).is_err());
        assert_eq!(state, original);
    }
}
