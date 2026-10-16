use app_application::agent::{human_commands::*, recovery::*};
use app_application::models::adventure::{Action, Location};
use uuid::Uuid;

fn ledger() -> StoredHumanCommandLedger {
    StoredHumanCommandLedger {
        manifest: HumanCommandManifest {
            version: HUMAN_COMMAND_VERSION,
            scope: CommandScope {
                request_id: Uuid::new_v4(),
                campaign_id: Uuid::new_v4(),
                session_id: Uuid::new_v4(),
                actor_id: Uuid::new_v4(),
                authority: CommandAuthority::Solo {
                    restore_generation: 2,
                },
            },
            commands: vec![HumanCommand {
                id: 1,
                source: CommandSpan {
                    start: 0,
                    end: 6,
                    quote: "SECRET".into(),
                },
                goal: HumanCommandGoal::Exact {
                    action: HumanAction::Adventure(Action::Move(Location::Quay)),
                },
                condition: None,
                repetition: None,
            }],
            constraints: vec![],
        },
        bindings: vec![],
        receipts: vec![],
        clarifications: vec![],
        final_narration: None,
    }
}

fn complete(ledger: &mut StoredHumanCommandLedger) {
    let binding = PreparedCommandBinding {
        scope: ledger.manifest.scope.clone(),
        command_id: 1,
        step: 1,
        tool_call_id: "PRIVATE_TOOL".into(),
        signature: HumanAction::Adventure(Action::Move(Location::Quay)),
        terminal: true,
        canonical_nonce: None,
    };
    ledger.bindings.push(binding.clone());
    ledger.receipts.push(ConfirmedCommandAttempt {
        binding,
        outcome: CommandAttemptOutcome::Performed {
            check_success: None,
        },
    });
}

#[test]
fn an_interrupted_effect_is_confirmed_without_serializing_private_data() {
    let mut input = ledger();
    complete(&mut input);
    input.final_narration = Some(HumanCommandFinalNarration {
        id: Uuid::new_v4(),
        text: "PRIVATE_NARRATION".into(),
    });
    let view = project_recovery(
        input.manifest.scope.clone(),
        Some("model-a".into()),
        false,
        Some(&input),
    )
    .unwrap();
    assert_eq!(view.phase, RecoveryPhase::DeliveryPending);
    assert_eq!(view.allowed_action, RecoveryAction::Resume);
    assert_eq!(view.confirmed_effects.len(), 1);
    assert_eq!(view.commands[0].state, RecoveryCommandState::Finished);
    let json = serde_json::to_string(&view).unwrap();
    for secret in [
        "SECRET",
        "PRIVATE_NARRATION",
        "PRIVATE_TOOL",
        "source",
        "signature",
        "bindings",
        "nonce",
    ] {
        assert!(
            !json.contains(secret),
            "recovery projection contains a private ledger field"
        );
    }
}

#[test]
fn committed_effect_without_narration_is_not_reported_as_delivered() {
    let mut input = ledger();
    complete(&mut input);
    let view = project_recovery(input.manifest.scope.clone(), None, false, Some(&input)).unwrap();
    assert_eq!(view.phase, RecoveryPhase::NarrationPending);
    assert!(view.progress_known);
}

#[test]
fn a_current_running_turn_cannot_be_resumed_concurrently() {
    let input = ledger();
    let view = project_recovery(input.manifest.scope.clone(), None, true, Some(&input)).unwrap();
    assert_eq!(view.phase, RecoveryPhase::Running);
    assert_eq!(view.allowed_action, RecoveryAction::Wait);
}

#[test]
fn a_legacy_turn_has_unknown_progress_instead_of_invented_effect_counts() {
    let input = ledger();
    let view = project_recovery(input.manifest.scope, None, false, None).unwrap();
    assert_eq!(view.phase, RecoveryPhase::ProgressUnknown);
    assert!(!view.progress_known);
    assert!(view.confirmed_effects.is_empty());
}

#[test]
fn clarification_exports_only_its_identity_and_command() {
    let mut input = ledger();
    input.manifest.commands[0].goal = HumanCommandGoal::NeedsResolution {
        description: "PRIVATE_TARGET".into(),
    };
    input.clarifications.push(HumanCommandClarification {
        id: Uuid::new_v4(),
        command_id: 1,
        question: "PRIVATE_QUESTION".into(),
        reply: None,
    });
    let view = project_recovery(input.manifest.scope.clone(), None, false, Some(&input)).unwrap();
    assert_eq!(view.phase, RecoveryPhase::NeedsInput);
    assert_eq!(view.allowed_action, RecoveryAction::Clarify);
    assert_eq!(view.clarification.unwrap().id, input.clarifications[0].id);
}

#[test]
fn a_different_actor_or_restored_timeline_rejects_the_old_progress() {
    let input = ledger();
    let mut scope = input.manifest.scope.clone();
    scope.actor_id = Uuid::new_v4();
    assert_eq!(
        project_recovery(scope, None, false, Some(&input)).unwrap_err(),
        CommandLedgerError::ScopeMismatch
    );
    let mut scope = input.manifest.scope.clone();
    scope.authority = CommandAuthority::Solo {
        restore_generation: 3,
    };
    assert_eq!(
        project_recovery(scope, None, false, Some(&input)).unwrap_err(),
        CommandLedgerError::ScopeMismatch
    );
}

#[test]
fn a_definitive_refusal_is_not_a_performed_effect() {
    let mut input = ledger();
    complete(&mut input);
    input.receipts[0].outcome = CommandAttemptOutcome::Refused {
        code: "adventure_action_unavailable".into(),
    };
    let view = project_recovery(input.manifest.scope.clone(), None, false, Some(&input)).unwrap();
    assert!(
        matches!(&view.confirmed_effects[0].outcome, RecoveryOutcome::Refused { code } if code == "adventure_action_unavailable")
    );
    input.receipts[0].outcome = CommandAttemptOutcome::Refused {
        code: "PRIVATE_REASON".into(),
    };
    assert_eq!(
        project_recovery(input.manifest.scope.clone(), None, false, Some(&input)).unwrap_err(),
        CommandLedgerError::OutcomeInvalid
    );
}
