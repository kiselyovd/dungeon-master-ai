use app_domain::adventure::{Action, AdventureState, DefenseOutcome, Fact, Location};
use uuid::Uuid;

fn archive() -> AdventureState {
    let mut state = AdventureState::new();
    state.location = Location::Archive;
    state.facts.insert(Fact::TikhonLocated);
    state
}
#[test]
fn defense_is_an_optional_linked_branch_and_does_not_repair_or_rescue() {
    let state = archive();
    assert!(state.available_actions().contains(&Action::StartDefense));
    assert!(state.available_actions().contains(&Action::RescueTikhon));
    let mut active = state.apply(Action::StartDefense).unwrap().state;
    let id = Uuid::new_v4();
    active.defense_encounter_id = Some(id);
    assert!(active.available_actions().is_empty());
    let resolved = active.resolve_defense(id, DefenseOutcome::Held).unwrap();
    assert!(resolved.facts.contains(&Fact::DefenseHeld));
    assert!(!resolved.facts.contains(&Fact::WardRestored));
    assert!(!resolved.facts.contains(&Fact::TikhonRescued));
    assert!(resolved.available_actions().contains(&Action::RescueTikhon));
}
#[test]
fn retreat_and_defeat_keep_the_adventure_resumable() {
    for (outcome, location) in [
        (DefenseOutcome::Retreated, Location::Quay),
        (DefenseOutcome::Defeated, Location::Town),
    ] {
        let mut state = archive().apply(Action::StartDefense).unwrap().state;
        let id = Uuid::new_v4();
        state.defense_encounter_id = Some(id);
        let resolved = state.resolve_defense(id, outcome).unwrap();
        assert_eq!(resolved.location, location);
        assert!(!resolved.closed);
        assert!(!resolved.facts.contains(&Fact::DefenseHeld));
    }
}
#[test]
fn unrelated_encounters_cannot_resolve_the_authored_defense() {
    let mut state = archive().apply(Action::StartDefense).unwrap().state;
    state.defense_encounter_id = Some(Uuid::new_v4());
    assert!(state
        .resolve_defense(Uuid::new_v4(), DefenseOutcome::Held)
        .is_err());
}
