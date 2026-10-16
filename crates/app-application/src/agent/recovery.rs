//! Player-visible progress projected from a repository-validated canonical ledger.
//! This projection grants no authority and never serializes private command data.
use super::human_commands::{
    CommandAttemptOutcome, CommandAuthority, CommandLedgerError, CommandScope, HumanAction,
    HumanCommandProgress, StoredHumanCommandLedger,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryPhase {
    Running,
    ProgressUnknown,
    ActionsPending,
    NeedsInput,
    NarrationPending,
    DeliveryPending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryAction {
    Wait,
    Resume,
    Clarify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryCommandState {
    Pending,
    Prepared,
    Finished,
    Skipped,
    NeedsResolution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryCommand {
    pub command_id: u16,
    pub state: RecoveryCommandState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryEffectKind {
    Adventure,
    Combat,
    Rest,
    Dice,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RecoveryOutcome {
    Performed { check_success: Option<bool> },
    Refused { code: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryEffect {
    pub command_id: u16,
    pub step: u16,
    pub action: RecoveryEffectKind,
    pub outcome: RecoveryOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryClarification {
    pub id: Uuid,
    pub command_id: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentRecoveryProjection {
    pub schema_version: u16,
    pub scope: CommandScope,
    pub model: Option<String>,
    pub phase: RecoveryPhase,
    pub allowed_action: RecoveryAction,
    pub progress_known: bool,
    pub commands: Vec<RecoveryCommand>,
    pub confirmed_effects: Vec<RecoveryEffect>,
    pub clarification: Option<RecoveryClarification>,
}

/// The adapter must validate live actor/timeline and canonical receipt signatures
/// before providing a ledger. No narration, source spans or tool JSON is copied.
pub fn project_recovery(
    scope: CommandScope,
    model: Option<String>,
    running: bool,
    ledger: Option<&StoredHumanCommandLedger>,
) -> Result<AgentRecoveryProjection, CommandLedgerError> {
    if [
        scope.request_id,
        scope.campaign_id,
        scope.session_id,
        scope.actor_id,
    ]
    .iter()
    .any(Uuid::is_nil)
        || matches!(&scope.authority, CommandAuthority::Room { room_id, member_id, .. }
            if room_id.is_nil() || member_id.is_nil())
    {
        return Err(CommandLedgerError::ScopeMismatch);
    }
    let mut projection = AgentRecoveryProjection {
        schema_version: 1,
        scope,
        model,
        phase: if running {
            RecoveryPhase::Running
        } else {
            RecoveryPhase::ProgressUnknown
        },
        allowed_action: if running {
            RecoveryAction::Wait
        } else {
            RecoveryAction::Resume
        },
        progress_known: ledger.is_some(),
        commands: vec![],
        confirmed_effects: vec![],
        clarification: None,
    };
    let Some(ledger) = ledger else {
        return Ok(projection);
    };
    if ledger.manifest.scope != projection.scope {
        return Err(CommandLedgerError::ScopeMismatch);
    }
    ledger.validate_clarifications()?;
    let progress = ledger
        .manifest
        .progress(&ledger.bindings, &ledger.receipts)?;
    let all_closed = progress.iter().all(|(_, state)| state.closed());
    projection.commands = progress
        .into_iter()
        .map(|(command_id, state)| RecoveryCommand {
            command_id,
            state: match state {
                HumanCommandProgress::Finished => RecoveryCommandState::Finished,
                HumanCommandProgress::NotTriggered => RecoveryCommandState::Skipped,
                HumanCommandProgress::Prepared => RecoveryCommandState::Prepared,
                HumanCommandProgress::NeedsResolution => RecoveryCommandState::NeedsResolution,
                _ => RecoveryCommandState::Pending,
            },
        })
        .collect();
    projection.confirmed_effects = ledger
        .receipts
        .iter()
        .map(|receipt| {
            let outcome = match &receipt.outcome {
                CommandAttemptOutcome::Performed { check_success } => RecoveryOutcome::Performed {
                    check_success: *check_success,
                },
                CommandAttemptOutcome::Refused { code } => {
                    if !super::intentions::definitive_rejection(code) {
                        return Err(CommandLedgerError::OutcomeInvalid);
                    }
                    RecoveryOutcome::Refused { code: code.clone() }
                }
            };
            Ok(RecoveryEffect {
                command_id: receipt.binding.command_id,
                step: receipt.binding.step,
                action: match receipt.binding.signature {
                    HumanAction::Adventure(_) => RecoveryEffectKind::Adventure,
                    HumanAction::Combat(_) => RecoveryEffectKind::Combat,
                    HumanAction::Rest(_) => RecoveryEffectKind::Rest,
                    HumanAction::Dice(_) => RecoveryEffectKind::Dice,
                },
                outcome,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    projection.clarification = ledger
        .clarifications
        .last()
        .filter(|entry| entry.reply.is_none())
        .map(|entry| RecoveryClarification {
            id: entry.id,
            command_id: entry.command_id,
        });
    if !running {
        projection.phase = if projection.clarification.is_some() {
            projection.allowed_action = RecoveryAction::Clarify;
            RecoveryPhase::NeedsInput
        } else if ledger.final_narration.is_some() {
            RecoveryPhase::DeliveryPending
        } else if all_closed {
            RecoveryPhase::NarrationPending
        } else {
            RecoveryPhase::ActionsPending
        };
    }
    Ok(projection)
}
