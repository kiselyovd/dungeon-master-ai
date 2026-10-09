use app_application::{
    agent::{
        commands::{AgentToolCommand, ApplyHealingCommand, RollDiceCommand},
        hero_intentions::HeroCombatIntent,
        human_commands::*,
    },
    hero_checks::{HeroSkillCheck, Skill},
};
use app_domain::{
    adventure::{Action, Location},
    combat::loadout::WeaponMode,
};
use uuid::Uuid;

#[test]
fn clarification_keeps_scope_source_and_unfinished_intent_until_verified_resolution() {
    let (message, mut manifest) = compound();
    manifest.commands[0].goal = HumanCommandGoal::NeedsResolution {
        description: "Выбранное игроком место назначения".into(),
    };
    let original = manifest.clone();
    let mut ledger = StoredHumanCommandLedger {
        manifest,
        bindings: vec![],
        receipts: vec![],
        clarifications: vec![],
        final_narration: None,
    };
    let question = ledger
        .record_question(1, "Какой причал вы имеете в виду?".into())
        .unwrap();
    assert_eq!(ledger.resolution_command().unwrap(), Some(1));
    assert_eq!(
        ledger
            .record_question(1, question.question.clone())
            .unwrap(),
        question
    );
    assert!(ledger.record_question(2, "Другой вопрос".into()).is_err());
    assert!(ledger
        .resolve_goal(1, HumanAction::Adventure(Action::Move(Location::Quay)))
        .is_err());
    assert!(ledger
        .record_reply(Uuid::new_v4(), "Старый".into())
        .is_err());
    ledger
        .record_reply(question.id, "Старый причал".into())
        .unwrap();
    ledger
        .record_reply(question.id, "Старый причал".into())
        .unwrap();
    assert!(ledger.record_reply(question.id, "Архив".into()).is_err());
    ledger
        .resolve_goal(1, HumanAction::Adventure(Action::Move(Location::Quay)))
        .unwrap();
    assert_eq!(ledger.manifest.scope, original.scope);
    assert_eq!(
        ledger.manifest.commands[0].source,
        original.commands[0].source
    );
    assert_eq!(ledger.manifest.commands[1], original.commands[1]);
    assert_eq!(ledger.manifest.constraints, original.constraints);
    ledger
        .manifest
        .validate_structure(&message, &original.scope)
        .unwrap();
    assert!(ledger.record_question(1, "Повторить путь?".into()).is_err());
    assert!(ledger
        .resolve_goal(1, HumanAction::Adventure(Action::Move(Location::Archive)))
        .is_err());
}

fn scope() -> CommandScope {
    CommandScope {
        request_id: Uuid::new_v4(),
        campaign_id: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
        actor_id: Uuid::new_v4(),
        authority: CommandAuthority::Solo {
            restore_generation: 0,
        },
    }
}
fn compound() -> (String, HumanCommandManifest) {
    let message = "Иду на причал и осматриваю его. Ход не завершаю.".to_string();
    let manifest = HumanCommandManifest {
        version: HUMAN_COMMAND_VERSION,
        scope: scope(),
        commands: vec![
            HumanCommand {
                repetition: None,
                id: 1,
                source: CommandSpan::from_quote(&message, "Иду на причал", 0).unwrap(),
                goal: HumanCommandGoal::Exact {
                    action: HumanAction::Adventure(Action::Move(Location::Quay)),
                },
                condition: None,
            },
            HumanCommand {
                repetition: None,
                id: 2,
                source: CommandSpan::from_quote(&message, "осматриваю его", 0).unwrap(),
                goal: HumanCommandGoal::Exact {
                    action: HumanAction::Adventure(Action::InspectQuay),
                },
                condition: None,
            },
        ],
        constraints: vec![CommandSpan::from_quote(&message, "Ход не завершаю", 0).unwrap()],
    };
    (message, manifest)
}
fn binding(manifest: &HumanCommandManifest, id: u16, call: &str) -> PreparedCommandBinding {
    let HumanCommandGoal::Exact { action } = manifest
        .commands
        .iter()
        .find(|c| c.id == id)
        .unwrap()
        .goal
        .clone()
    else {
        panic!("exact goal")
    };
    PreparedCommandBinding {
        scope: manifest.scope.clone(),
        command_id: id,
        step: 1,
        tool_call_id: call.into(),
        signature: action,
        terminal: true,
        canonical_nonce: None,
    }
}
fn performed(binding: &PreparedCommandBinding) -> ConfirmedCommandAttempt {
    ConfirmedCommandAttempt {
        binding: binding.clone(),
        outcome: CommandAttemptOutcome::Performed {
            check_success: None,
        },
    }
}

#[test]
fn exact_saved_source_and_scope_are_required_and_constraints_cannot_be_commands() {
    let (message, manifest) = compound();
    manifest
        .validate_structure(&message, &manifest.scope)
        .unwrap();
    let mut altered = manifest.clone();
    altered.commands[0].source.start += 1;
    assert_eq!(
        altered.validate_structure(&message, &manifest.scope),
        Err(CommandLedgerError::InvalidManifest)
    );
    assert_eq!(
        manifest.validate_structure("Другой запрос", &manifest.scope),
        Err(CommandLedgerError::InvalidManifest)
    );
    let mut different = manifest.scope.clone();
    different.actor_id = Uuid::new_v4();
    assert_eq!(
        manifest.validate_structure(&message, &different),
        Err(CommandLedgerError::ScopeMismatch)
    );
    let mut negated = manifest.clone();
    negated.commands[1].source = CommandSpan::from_quote(&message, "завершаю", 0).unwrap();
    assert_eq!(
        negated.validate_structure(&message, &manifest.scope),
        Err(CommandLedgerError::InvalidManifest)
    );
    let mut duplicated = manifest.clone();
    duplicated.commands[1].id = 1;
    assert!(duplicated
        .validate_structure(&message, &manifest.scope)
        .is_err());
}
#[test]
fn compound_progress_uses_confirmed_receipts_not_preparation_or_prose() {
    let (_, manifest) = compound();
    let travel = binding(&manifest, 1, "travel");
    let inspect = binding(&manifest, 2, "inspect");
    assert_eq!(
        manifest.progress(&[], &[]).unwrap(),
        vec![
            (1, HumanCommandProgress::Ready),
            (2, HumanCommandProgress::WaitingForPrevious)
        ]
    );
    assert_eq!(
        manifest
            .progress(std::slice::from_ref(&travel), &[])
            .unwrap()[0]
            .1,
        HumanCommandProgress::Prepared
    );
    assert_eq!(
        manifest.prepare_binding(&inspect, std::slice::from_ref(&travel), &[]),
        Err(CommandLedgerError::NotReady)
    );
    let first = performed(&travel);
    assert_eq!(
        manifest
            .progress(std::slice::from_ref(&travel), std::slice::from_ref(&first))
            .unwrap()[1]
            .1,
        HumanCommandProgress::Ready
    );
    manifest
        .prepare_binding(
            &inspect,
            std::slice::from_ref(&travel),
            std::slice::from_ref(&first),
        )
        .unwrap();
    let bindings = vec![travel.clone(), inspect.clone()];
    let receipts = vec![first, performed(&inspect)];
    assert!(manifest
        .progress(&bindings, &receipts)
        .unwrap()
        .iter()
        .all(|(_, state)| state.closed()));
    manifest
        .prepare_binding(&inspect, &bindings, &receipts)
        .unwrap();
    let mut repeat = inspect;
    repeat.tool_call_id = "repeat-inspect".into();
    repeat.step = 2;
    assert_eq!(
        manifest.prepare_binding(&repeat, &bindings, &receipts),
        Err(CommandLedgerError::NotReady)
    );
}

fn quantified() -> (String, HumanCommandManifest) {
    let message = "Жду минуту дважды. Дальше не иду".to_string();
    let mut manifest = compound().1;
    manifest.commands = (1..=2)
        .map(|index| HumanCommand {
            id: index,
            source: CommandSpan::from_quote(&message, "Жду минуту дважды", 0).unwrap(),
            goal: HumanCommandGoal::Exact {
                action: HumanAction::Rest(app_domain::rest::RestIntent::Wait { minutes: 1 }),
            },
            condition: None,
            repetition: Some(CommandRepetition { index, total: 2 }),
        })
        .collect();
    manifest.constraints = vec![CommandSpan::from_quote(&message, "Дальше не иду", 0).unwrap()];
    (message, manifest)
}

#[test]
fn quantified_groups_require_complete_contiguous_ordinals_and_exact_source() {
    let (message, manifest) = quantified();
    manifest
        .validate_structure(&message, &manifest.scope)
        .unwrap();
    for change in 0..7 {
        let mut bad = manifest.clone();
        match change {
            0 => {
                bad.commands.pop();
            }
            1 => bad.commands[1].repetition.as_mut().unwrap().index = 1,
            2 => bad.commands[1].repetition.as_mut().unwrap().total = 3,
            3 => bad.commands[1].repetition = None,
            4 => {
                bad.commands[1].source = CommandSpan::from_quote(&message, "Жду минуту", 0).unwrap()
            }
            5 => bad.commands[0].repetition.as_mut().unwrap().index = 0,
            _ => bad.constraints = vec![bad.commands[0].source.clone()],
        }
        assert!(
            bad.validate_structure(&message, &manifest.scope).is_err(),
            "change {change}"
        );
    }
    let (_, ordinary) = compound();
    let wire = serde_json::to_value(&ordinary).unwrap();
    assert!(wire["commands"][0].get("repetition").is_none());
    let legacy: HumanCommandManifest = serde_json::from_value(wire).unwrap();
    assert_eq!(legacy, ordinary);
}

#[test]
fn quantified_attempts_have_independent_receipts_and_refusal_is_not_a_retry() {
    let (_, manifest) = quantified();
    let first = binding(&manifest, 1, "wait-1");
    let second = binding(&manifest, 2, "wait-2");
    assert!(manifest.prepare_binding(&second, &[], &[]).is_err());
    let first_receipt = performed(&first);
    manifest
        .prepare_binding(
            &second,
            std::slice::from_ref(&first),
            std::slice::from_ref(&first_receipt),
        )
        .unwrap();
    let refused = ConfirmedCommandAttempt {
        binding: second.clone(),
        outcome: CommandAttemptOutcome::Refused {
            code: "rest_unsafe".into(),
        },
    };
    let bindings = vec![first, second.clone()];
    let receipts = vec![first_receipt, refused];
    assert!(manifest
        .progress(&bindings, &receipts)
        .unwrap()
        .iter()
        .all(|(_, state)| state.closed()));
    // Re-delivery of the same attempted command is allowed, a third effect is not.
    manifest
        .prepare_binding(&second, &bindings, &receipts)
        .unwrap();
    let mut extra = second;
    extra.step += 1;
    extra.tool_call_id = "wait-extra".into();
    assert!(manifest
        .prepare_binding(&extra, &bindings, &receipts)
        .is_err());
}
#[test]
fn intermediate_movement_cannot_finish_a_different_destination_or_bypass_pending_step() {
    let (_, mut manifest) = compound();
    manifest.commands.truncate(1);
    manifest.commands[0].goal = HumanCommandGoal::Exact {
        action: HumanAction::Adventure(Action::Move(Location::Archive)),
    };
    let mut via = binding(&manifest, 1, "via-quay");
    via.signature = HumanAction::Adventure(Action::Move(Location::Quay));
    via.terminal = false;
    manifest.prepare_binding(&via, &[], &[]).unwrap();
    assert_eq!(
        manifest.progress(std::slice::from_ref(&via), &[]).unwrap()[0].1,
        HumanCommandProgress::Prepared
    );
    let mut arrive = binding(&manifest, 1, "archive");
    arrive.step = 2;
    assert_eq!(
        manifest.prepare_binding(&arrive, std::slice::from_ref(&via), &[]),
        Err(CommandLedgerError::NotReady)
    );
    let receipt = performed(&via);
    assert_eq!(
        manifest
            .progress(std::slice::from_ref(&via), std::slice::from_ref(&receipt))
            .unwrap()[0]
            .1,
        HumanCommandProgress::Ready
    );
    manifest
        .prepare_binding(
            &arrive,
            std::slice::from_ref(&via),
            std::slice::from_ref(&receipt),
        )
        .unwrap();
    let mut wrong = via.clone();
    wrong.terminal = true;
    assert_eq!(
        manifest.prepare_binding(&wrong, &[], &[]),
        Err(CommandLedgerError::SignatureMismatch)
    );
}
#[test]
fn definitive_refusal_closes_attempt_and_uncertain_or_permission_errors_do_not() {
    let (_, manifest) = compound();
    let step = binding(&manifest, 1, "move");
    let mut receipt = performed(&step);
    receipt.outcome = CommandAttemptOutcome::Refused {
        code: "adventure_action_unavailable".into(),
    };
    assert_eq!(
        manifest
            .progress(std::slice::from_ref(&step), std::slice::from_ref(&receipt))
            .unwrap()[0]
            .1,
        HumanCommandProgress::Finished
    );
    for code in [
        "player_intention_not_requested",
        "tool_checkpoint_failed",
        "tool_args_invalid",
    ] {
        receipt.outcome = CommandAttemptOutcome::Refused { code: code.into() };
        assert_eq!(
            manifest.progress(std::slice::from_ref(&step), std::slice::from_ref(&receipt)),
            Err(CommandLedgerError::OutcomeInvalid)
        );
    }
}
#[test]
fn identical_attack_phrases_remain_distinct_commands() {
    let message = "Атакую. Атакую.";
    let mut manifest = compound().1;
    manifest.constraints.clear();
    let attack = HumanAction::Combat(HeroCombatIntent::Weapon {
        target: Uuid::new_v4(),
        weapon_id: "longbow".into(),
        mode: WeaponMode::Ranged,
        two_handed: false,
    });
    manifest.commands = (0..2)
        .map(|index| HumanCommand {
            repetition: None,
            id: index as u16 + 1,
            source: CommandSpan::from_quote(message, "Атакую", index).unwrap(),
            goal: HumanCommandGoal::Exact {
                action: attack.clone(),
            },
            condition: None,
        })
        .collect();
    manifest
        .validate_structure(message, &manifest.scope)
        .unwrap();
    let first = binding(&manifest, 1, "shot-1");
    let second = binding(&manifest, 2, "shot-2");
    let receipt = performed(&first);
    assert_eq!(
        manifest
            .progress(std::slice::from_ref(&first), std::slice::from_ref(&receipt))
            .unwrap()[1]
            .1,
        HumanCommandProgress::Ready
    );
    manifest
        .prepare_binding(
            &second,
            std::slice::from_ref(&first),
            std::slice::from_ref(&receipt),
        )
        .unwrap();
    assert_ne!(
        manifest.commands[0].source.start,
        manifest.commands[1].source.start
    );
}
#[test]
fn outcome_conditions_use_saved_check_result_and_unknown_is_not_false() {
    let (message, mut manifest) = compound();
    manifest.commands[0].goal = HumanCommandGoal::Exact {
        action: HumanAction::Dice(RollDiceCommand {
            dice: "1d20".into(),
            modifier: None,
            advantage: None,
            disadvantage: None,
            reason: None,
            check: Some(HeroSkillCheck {
                character_id: manifest.scope.actor_id,
                skill: Skill::Stealth,
                dc: Some(12),
            }),
        }),
    };
    manifest.commands[1].condition = Some(CommandCondition {
        command_id: 1,
        predicate: CommandPredicate::CheckFailed,
    });
    manifest
        .validate_structure(&message, &manifest.scope)
        .unwrap();
    let check = binding(&manifest, 1, "check");
    for (success, expected) in [
        (Some(true), HumanCommandProgress::NotTriggered),
        (Some(false), HumanCommandProgress::Ready),
        (None, HumanCommandProgress::WaitingForCondition),
    ] {
        let receipt = ConfirmedCommandAttempt {
            binding: check.clone(),
            outcome: CommandAttemptOutcome::Performed {
                check_success: success,
            },
        };
        assert_eq!(
            manifest
                .progress(std::slice::from_ref(&check), &[receipt])
                .unwrap()[1]
                .1,
            expected
        );
    }
    let mut forward = manifest.clone();
    forward.commands[0].condition = Some(CommandCondition {
        command_id: 2,
        predicate: CommandPredicate::AttemptRefused,
    });
    assert!(forward
        .validate_structure(&message, &manifest.scope)
        .is_err());
}
#[test]
fn scope_signature_nonce_and_preparation_order_cannot_be_forged() {
    let (_, manifest) = compound();
    let step = binding(&manifest, 1, "move");
    let receipt = performed(&step);
    let mut forged = receipt.clone();
    forged.binding.scope.authority = CommandAuthority::Solo {
        restore_generation: 1,
    };
    assert_eq!(
        manifest.progress(std::slice::from_ref(&step), &[forged]),
        Err(CommandLedgerError::ScopeMismatch)
    );
    let mut forged = receipt.clone();
    forged.binding.canonical_nonce = Some(Uuid::new_v4());
    assert_eq!(
        manifest.progress(std::slice::from_ref(&step), &[forged]),
        Err(CommandLedgerError::SignatureMismatch)
    );
    let mut later = step.clone();
    later.step = 2;
    later.tool_call_id = "another".into();
    assert_eq!(
        manifest.progress(&[step.clone(), later], std::slice::from_ref(&receipt)),
        Err(CommandLedgerError::BindingConflict)
    );
    let mut jump = step.clone();
    jump.step = 2;
    assert_eq!(
        manifest.prepare_binding(&jump, &[], &[]),
        Err(CommandLedgerError::BindingConflict)
    );
    assert_eq!(
        manifest.progress(&[], std::slice::from_ref(&receipt)),
        Err(CommandLedgerError::SignatureMismatch)
    );
}
#[test]
fn unresolved_future_command_is_retained_without_synthetic_authorization() {
    let (_, mut manifest) = compound();
    manifest.commands.truncate(1);
    manifest.commands[0].goal = HumanCommandGoal::NeedsResolution {
        description: "Attack the first echo after it spawns".into(),
    };
    assert_eq!(
        manifest.progress(&[], &[]).unwrap()[0].1,
        HumanCommandProgress::NeedsResolution
    );
    assert!(
        HumanAction::from_tool(&AgentToolCommand::ApplyHealing(ApplyHealingCommand {
            token_id: "human".into(),
            amount: 10
        }))
        .is_err()
    );
}

#[test]
fn one_canonical_effect_cannot_complete_two_commands_but_exact_replay_is_allowed() {
    let (_, manifest) = compound();
    let nonce = Uuid::new_v4();
    let mut first = binding(&manifest, 1, "travel");
    first.canonical_nonce = Some(nonce);
    let mut second = binding(&manifest, 2, "inspect");
    second.canonical_nonce = Some(nonce);
    let receipt = performed(&first);
    manifest
        .prepare_binding(
            &first,
            std::slice::from_ref(&first),
            std::slice::from_ref(&receipt),
        )
        .unwrap();
    assert_eq!(
        manifest.prepare_binding(
            &second,
            std::slice::from_ref(&first),
            std::slice::from_ref(&receipt)
        ),
        Err(CommandLedgerError::BindingConflict)
    );
    assert_eq!(
        manifest.progress(
            &[first.clone(), second.clone()],
            &[receipt, performed(&second)]
        ),
        Err(CommandLedgerError::BindingConflict)
    );
}
#[test]
fn specific_refusal_condition_does_not_activate_for_another_failure_reason() {
    let (message, mut manifest) = compound();
    manifest.commands[1].condition = Some(CommandCondition {
        command_id: 1,
        predicate: CommandPredicate::AttemptRefusedBecause {
            code: "adventure_action_unavailable".into(),
        },
    });
    manifest
        .validate_structure(&message, &manifest.scope)
        .unwrap();
    let step = binding(&manifest, 1, "travel");
    for (code, expected) in [
        ("adventure_action_unavailable", HumanCommandProgress::Ready),
        (
            "adventure_combat_active",
            HumanCommandProgress::NotTriggered,
        ),
    ] {
        let receipt = ConfirmedCommandAttempt {
            binding: step.clone(),
            outcome: CommandAttemptOutcome::Refused { code: code.into() },
        };
        assert_eq!(
            manifest
                .progress(std::slice::from_ref(&step), &[receipt])
                .unwrap()[1]
                .1,
            expected
        );
    }
    manifest.commands[1].condition = Some(CommandCondition {
        command_id: 1,
        predicate: CommandPredicate::AttemptRefusedBecause {
            code: "provider_network".into(),
        },
    });
    assert!(manifest
        .validate_structure(&message, &manifest.scope)
        .is_err());
}
#[test]
fn room_metadata_requires_nonce_and_rejects_another_member_or_epoch() {
    let (_, mut manifest) = compound();
    manifest.scope.authority = CommandAuthority::Room {
        room_id: Uuid::new_v4(),
        member_id: Uuid::new_v4(),
        epoch: 3,
        reset_cursor: 8,
    };
    let mut step = binding(&manifest, 1, "travel");
    assert_eq!(
        manifest.prepare_binding(&step, &[], &[]),
        Err(CommandLedgerError::BindingConflict)
    );
    step.canonical_nonce = Some(Uuid::new_v4());
    manifest.prepare_binding(&step, &[], &[]).unwrap();
    for change_member in [true, false] {
        let mut other = step.clone();
        let CommandAuthority::Room {
            member_id, epoch, ..
        } = &mut other.scope.authority
        else {
            panic!("room")
        };
        if change_member {
            *member_id = Uuid::new_v4();
        } else {
            *epoch += 1;
        }
        assert_eq!(
            manifest.prepare_binding(&other, &[], &[]),
            Err(CommandLedgerError::ScopeMismatch)
        );
    }
}
