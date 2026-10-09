//! Private durable progress metadata. Structural validation is not permission.
//! Repositories must validate current authority and canonical receipts; callers
//! must independently verify the manifest and every candidate before effects.
use super::{
    commands::{AgentToolCommand, RollDiceCommand},
    hero_intentions::HeroCombatIntent,
};
use crate::models::{adventure::Action, rest::RestIntent};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub const HUMAN_COMMAND_VERSION: u16 = 1;
const MAX_COMMANDS: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CommandAuthority {
    Solo {
        restore_generation: u64,
    },
    Room {
        room_id: Uuid,
        member_id: Uuid,
        epoch: u64,
        reset_cursor: u64,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandScope {
    pub request_id: Uuid,
    pub campaign_id: Uuid,
    pub session_id: Uuid,
    pub actor_id: Uuid,
    pub authority: CommandAuthority,
}
impl CommandScope {
    fn valid(&self) -> bool {
        ![
            self.request_id,
            self.campaign_id,
            self.session_id,
            self.actor_id,
        ]
        .iter()
        .any(Uuid::is_nil)
            && !matches!(&self.authority,CommandAuthority::Room {room_id,member_id,..} if room_id.is_nil()||member_id.is_nil())
    }
}
/// UTF-8 byte positions derived by the server from the original saved request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandSpan {
    pub start: u32,
    pub end: u32,
    pub quote: String,
}
impl CommandSpan {
    pub fn from_quote(
        message: &str,
        quote: &str,
        occurrence: usize,
    ) -> Result<Self, CommandLedgerError> {
        if quote.trim().is_empty() {
            return Err(CommandLedgerError::InvalidManifest);
        }
        let start = message
            .match_indices(quote)
            .nth(occurrence)
            .map(|(start, _)| start)
            .ok_or(CommandLedgerError::InvalidManifest)?;
        Ok(Self {
            start: u32::try_from(start).map_err(|_| CommandLedgerError::InvalidManifest)?,
            end: u32::try_from(start + quote.len())
                .map_err(|_| CommandLedgerError::InvalidManifest)?,
            quote: quote.into(),
        })
    }
    fn valid(&self, message: &str) -> bool {
        self.start < self.end
            && !self.quote.trim().is_empty()
            && self.quote.chars().count() <= 1600
            && message.get(self.start as usize..self.end as usize) == Some(self.quote.as_str())
    }
    fn overlaps(&self, other: &Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "parameters",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum HumanAction {
    Adventure(Action),
    Combat(HeroCombatIntent),
    Rest(RestIntent),
    Dice(RollDiceCommand),
}
impl HumanAction {
    pub fn to_tool(&self) -> AgentToolCommand {
        match self {
            Self::Adventure(action) => {
                AgentToolCommand::AdventureAction(super::commands::AdventureActionCommand {
                    action: *action,
                })
            }
            Self::Combat(action) => AgentToolCommand::CombatAction(action.clone()),
            Self::Rest(action) => AgentToolCommand::RestAction(*action),
            Self::Dice(action) => AgentToolCommand::RollDice(action.clone()),
        }
    }
    pub fn from_tool(command: &AgentToolCommand) -> Result<Self, CommandLedgerError> {
        match command {
            AgentToolCommand::AdventureAction(value) => Ok(Self::Adventure(value.action)),
            AgentToolCommand::CombatAction(value) => Ok(Self::Combat(value.clone())),
            AgentToolCommand::RestAction(value) => Ok(Self::Rest(*value)),
            AgentToolCommand::RollDice(value) => Ok(Self::Dice(value.clone())),
            _ => Err(CommandLedgerError::SignatureMismatch),
        }
    }
    pub fn same_mechanics(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Dice(a), Self::Dice(b)) => {
                a.dice == b.dice
                    && a.modifier == b.modifier
                    && a.advantage.unwrap_or(false) == b.advantage.unwrap_or(false)
                    && a.disadvantage.unwrap_or(false) == b.disadvantage.unwrap_or(false)
                    && a.check == b.check
            }
            _ => self == other,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum HumanCommandGoal {
    Exact {
        action: HumanAction,
    },
    /// Keeps an unresolved future target/unsupported intent visible. It cannot
    /// be bound or silently completed until a trusted resolution is persisted.
    NeedsResolution {
        description: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandPredicate {
    CheckSucceeded,
    CheckFailed,
    AttemptRefused,
    AttemptRefusedBecause { code: String },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandCondition {
    pub command_id: u16,
    pub predicate: CommandPredicate,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandRepetition {
    /// One-based position within an explicitly quantified source clause.
    pub index: u16,
    pub total: u16,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HumanCommand {
    pub id: u16,
    pub source: CommandSpan,
    pub goal: HumanCommandGoal,
    pub condition: Option<CommandCondition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repetition: Option<CommandRepetition>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HumanCommandManifest {
    pub version: u16,
    pub scope: CommandScope,
    pub commands: Vec<HumanCommand>,
    pub constraints: Vec<CommandSpan>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedCommandBinding {
    pub scope: CommandScope,
    pub command_id: u16,
    /// Monotonic per-command step number, allocated inside preparation transaction.
    pub step: u16,
    pub tool_call_id: String,
    /// Exact typed effect signature, distinct from the unchanged legacy tool JSON.
    pub signature: HumanAction,
    pub terminal: bool,
    pub canonical_nonce: Option<Uuid>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CommandAttemptOutcome {
    Performed { check_success: Option<bool> },
    Refused { code: String },
}
/// Only an authoritative, scope/signature-verified repository may supply this.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmedCommandAttempt {
    pub binding: PreparedCommandBinding,
    pub outcome: CommandAttemptOutcome,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoredHumanCommandLedger {
    pub manifest: HumanCommandManifest,
    pub bindings: Vec<PreparedCommandBinding>,
    pub receipts: Vec<ConfirmedCommandAttempt>,
    pub clarifications: Vec<HumanCommandClarification>,
    pub final_narration: Option<HumanCommandFinalNarration>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HumanCommandFinalNarration {
    pub id: Uuid,
    pub text: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingHumanCommandClarification {
    pub operation: super::operations::AgentOperation,
    pub clarification: Option<HumanCommandClarification>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HumanCommandClarification {
    pub id: Uuid,
    pub command_id: u16,
    pub question: String,
    pub reply: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HumanCommandClarificationReply {
    pub id: Uuid,
    pub reply: String,
}
impl HumanCommandClarificationReply {
    pub fn valid(&self) -> bool {
        !self.id.is_nil() && !self.reply.trim().is_empty() && self.reply.len() <= 4096
    }
}
impl StoredHumanCommandLedger {
    pub fn record_final(
        &mut self,
        text: String,
    ) -> Result<HumanCommandFinalNarration, CommandLedgerError> {
        if text.trim().is_empty()
            || text.len() > 65536
            || !self
                .manifest
                .progress(&self.bindings, &self.receipts)?
                .iter()
                .all(|(_, status)| status.closed())
        {
            return Err(CommandLedgerError::NotReady);
        }
        if let Some(saved) = &self.final_narration {
            return if saved.text == text {
                Ok(saved.clone())
            } else {
                Err(CommandLedgerError::BindingConflict)
            };
        }
        let saved = HumanCommandFinalNarration {
            id: Uuid::new_v4(),
            text,
        };
        self.final_narration = Some(saved.clone());
        Ok(saved)
    }
    pub fn resolution_command(&self) -> Result<Option<u16>, CommandLedgerError> {
        Ok(self
            .manifest
            .progress(&self.bindings, &self.receipts)?
            .into_iter()
            .find(|(_, status)| *status == HumanCommandProgress::NeedsResolution)
            .map(|(id, _)| id))
    }
    pub fn validate_clarifications(&self) -> Result<(), CommandLedgerError> {
        if let Some(saved) = &self.final_narration {
            if saved.id.is_nil()
                || saved.text.trim().is_empty()
                || saved.text.len() > 65536
                || !self
                    .manifest
                    .progress(&self.bindings, &self.receipts)?
                    .iter()
                    .all(|(_, status)| status.closed())
            {
                return Err(CommandLedgerError::InvalidManifest);
            }
        }
        let mut ids = BTreeSet::new();
        for entry in &self.clarifications {
            if entry.id.is_nil()
                || !ids.insert(entry.id)
                || entry.question.trim().is_empty()
                || entry.question.chars().count() > 1600
                || entry
                    .reply
                    .as_ref()
                    .is_some_and(|reply| reply.trim().is_empty() || reply.len() > 4096)
                || !self
                    .manifest
                    .commands
                    .iter()
                    .any(|command| command.id == entry.command_id)
            {
                return Err(CommandLedgerError::InvalidManifest);
            }
        }
        // Only the latest question may await input. Earlier replies remain
        // immutable evidence of what this same human actually clarified.
        if self
            .clarifications
            .iter()
            .rev()
            .skip(1)
            .any(|entry| entry.reply.is_none())
        {
            return Err(CommandLedgerError::InvalidManifest);
        }
        if let Some(entry) = self
            .clarifications
            .last()
            .filter(|entry| entry.reply.is_none())
        {
            if self.resolution_command()? != Some(entry.command_id) {
                return Err(CommandLedgerError::InvalidManifest);
            }
        }
        Ok(())
    }
    pub fn record_question(
        &mut self,
        command_id: u16,
        question: String,
    ) -> Result<HumanCommandClarification, CommandLedgerError> {
        if question.trim().is_empty() || question.chars().count() > 1600 {
            return Err(CommandLedgerError::InvalidManifest);
        }
        if self.resolution_command()? != Some(command_id) {
            return Err(CommandLedgerError::NotReady);
        }
        if let Some(last) = self
            .clarifications
            .last()
            .filter(|entry| entry.reply.is_none())
        {
            return if last.command_id == command_id && last.question == question {
                Ok(last.clone())
            } else {
                Err(CommandLedgerError::BindingConflict)
            };
        }
        let entry = HumanCommandClarification {
            id: Uuid::new_v4(),
            command_id,
            question,
            reply: None,
        };
        self.clarifications.push(entry.clone());
        self.validate_clarifications()?;
        Ok(entry)
    }
    pub fn record_reply(&mut self, id: Uuid, reply: String) -> Result<(), CommandLedgerError> {
        if reply.trim().is_empty() || reply.len() > 4096 {
            return Err(CommandLedgerError::InvalidManifest);
        }
        let entry = self
            .clarifications
            .iter_mut()
            .find(|entry| entry.id == id)
            .ok_or(CommandLedgerError::BindingConflict)?;
        if let Some(old) = &entry.reply {
            return if *old == reply {
                Ok(())
            } else {
                Err(CommandLedgerError::BindingConflict)
            };
        }
        entry.reply = Some(reply);
        self.validate_clarifications()
    }
    /// Semantic resolution must be independently verified first. Previously
    /// bound/completed goals cannot be rewritten by a clarification.
    pub fn resolve_goal(
        &mut self,
        command_id: u16,
        action: HumanAction,
    ) -> Result<(), CommandLedgerError> {
        if self.resolution_command()? != Some(command_id)
            || self
                .bindings
                .iter()
                .any(|binding| binding.command_id == command_id)
            || self
                .clarifications
                .last()
                .is_some_and(|entry| entry.reply.is_none())
        {
            return Err(CommandLedgerError::NotReady);
        }
        let command = self
            .manifest
            .commands
            .iter_mut()
            .find(|command| command.id == command_id)
            .ok_or(CommandLedgerError::NotReady)?;
        command.goal = HumanCommandGoal::Exact { action };
        self.manifest.progress(&self.bindings, &self.receipts)?;
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct HumanCommandStepProposal {
    pub command_id: u16,
    pub tool_call_id: String,
    pub signature: HumanAction,
    pub terminal: bool,
}
/// Configured privately for one operation/current owner, or one authenticated
/// room subject. No client history or model output may provide receipt outcomes.
#[async_trait::async_trait]
pub trait HumanCommandLedgerRepository: Send + Sync {
    /// Protected original text and authority; never supplied by model/history.
    async fn original_request(
        &self,
    ) -> Result<(CommandScope, String), crate::ports::repositories::RepositoryError>;
    /// Recheck current authority/timeline and canonical receipt signatures before
    /// returning progress, including recovery of effect-before-transcript gaps.
    async fn load_current(
        &self,
    ) -> Result<Option<StoredHumanCommandLedger>, crate::ports::repositories::RepositoryError>;
    /// The application has independently verified semantic intent. The adapter
    /// must validate structure against the SAVED request and derived actor scope,
    /// reject changed retries, and commit before any new effect is prepared.
    async fn save_verified_manifest(
        &self,
        manifest: HumanCommandManifest,
    ) -> Result<StoredHumanCommandLedger, crate::ports::repositories::RepositoryError>;
    /// Permission is checked by the application first. Allocate the step number
    /// and canonical identity in the SAME transaction as actual tool preparation;
    /// preserve old command_json signatures and reject conflicting retries.
    async fn prepare_after_permission(
        &self,
        proposal: HumanCommandStepProposal,
    ) -> Result<PreparedCommandBinding, crate::ports::repositories::RepositoryError>;
    /// Settle only a recognized engine refusal, never a permission denial or
    /// uncertain transport failure. Canonical effects still outrank metadata.
    async fn record_refusal(
        &self,
        tool_id: &str,
        code: &str,
    ) -> Result<(), crate::ports::repositories::RepositoryError>;
    async fn record_clarification(
        &self,
        command_id: u16,
        question: String,
    ) -> Result<HumanCommandClarification, crate::ports::repositories::RepositoryError>;
    async fn record_final_narration(
        &self,
        text: String,
    ) -> Result<HumanCommandFinalNarration, crate::ports::repositories::RepositoryError>;
    async fn submit_clarification_reply(
        &self,
        id: Uuid,
        reply: String,
    ) -> Result<(), crate::ports::repositories::RepositoryError>;
    async fn resolve_verified_goal(
        &self,
        command_id: u16,
        action: HumanAction,
    ) -> Result<(), crate::ports::repositories::RepositoryError>;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HumanCommandProgress {
    Ready,
    Prepared,
    Finished,
    NotTriggered,
    WaitingForPrevious,
    WaitingForCondition,
    NeedsResolution,
}
impl HumanCommandProgress {
    pub fn closed(self) -> bool {
        matches!(self, Self::Finished | Self::NotTriggered)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CommandLedgerError {
    #[error("human command manifest invalid")]
    InvalidManifest,
    #[error("human command authority mismatch")]
    ScopeMismatch,
    #[error("human command effect signature mismatch")]
    SignatureMismatch,
    #[error("human command binding conflict")]
    BindingConflict,
    #[error("human command is not ready")]
    NotReady,
    #[error("human command outcome is not authoritative")]
    OutcomeInvalid,
}
impl HumanCommandManifest {
    /// Shape/provenance only: this never establishes that a quoted phrase is a
    /// positive human instruction. The semantic verifier remains mandatory.
    pub fn validate_structure(
        &self,
        saved_message: &str,
        expected_scope: &CommandScope,
    ) -> Result<(), CommandLedgerError> {
        if &self.scope != expected_scope {
            return Err(CommandLedgerError::ScopeMismatch);
        }
        if self.version != HUMAN_COMMAND_VERSION
            || !self.scope.valid()
            || saved_message.len() > 65536
            || self.commands.len() > MAX_COMMANDS
            || self.constraints.len() > MAX_COMMANDS
        {
            return Err(CommandLedgerError::InvalidManifest);
        }
        let mut ids = BTreeSet::new();
        let mut spans = Vec::new();
        for (position, command) in self.commands.iter().enumerate() {
            let repeated_source = if let Some(repetition) = &command.repetition {
                if repetition.total < 2
                    || usize::from(repetition.total) > MAX_COMMANDS
                    || repetition.index == 0
                    || repetition.index > repetition.total
                {
                    return Err(CommandLedgerError::InvalidManifest);
                }
                let first = position
                    .checked_sub(usize::from(repetition.index - 1))
                    .ok_or(CommandLedgerError::InvalidManifest)?;
                let group = self
                    .commands
                    .get(first..first + usize::from(repetition.total))
                    .ok_or(CommandLedgerError::InvalidManifest)?;
                if group.iter().enumerate().any(|(offset, item)| {
                    item.source != command.source
                        || item.repetition.as_ref().is_none_or(|r| {
                            r.total != repetition.total || usize::from(r.index) != offset + 1
                        })
                }) {
                    return Err(CommandLedgerError::InvalidManifest);
                }
                repetition.index > 1
            } else {
                false
            };
            if command.id == 0
                || ids.contains(&command.id)
                || !command.source.valid(saved_message)
                || spans.last().is_some_and(|previous: &&CommandSpan| {
                    previous.end > command.source.start && !repeated_source
                })
            {
                return Err(CommandLedgerError::InvalidManifest);
            }
            if let Some(condition) = &command.condition {
                if !ids.contains(&condition.command_id) {
                    return Err(CommandLedgerError::InvalidManifest);
                }
                if matches!(&condition.predicate,CommandPredicate::AttemptRefusedBecause {code} if !super::intentions::definitive_rejection(code))
                {
                    return Err(CommandLedgerError::InvalidManifest);
                }
                if matches!(condition.predicate,CommandPredicate::CheckSucceeded|CommandPredicate::CheckFailed)&&!self.commands.iter().any(|source|source.id==condition.command_id&&matches!(&source.goal,HumanCommandGoal::Exact {action:HumanAction::Dice(dice)} if dice.check.is_some())) {return Err(CommandLedgerError::InvalidManifest);}
            }
            match &command.goal {
                HumanCommandGoal::Exact {
                    action: HumanAction::Dice(dice),
                } if dice
                    .check
                    .as_ref()
                    .is_some_and(|check| check.character_id != self.scope.actor_id) =>
                {
                    return Err(CommandLedgerError::ScopeMismatch)
                }
                HumanCommandGoal::NeedsResolution { description }
                    if description.trim().is_empty() || description.chars().count() > 1600 =>
                {
                    return Err(CommandLedgerError::InvalidManifest)
                }
                _ => {}
            }
            ids.insert(command.id);
            spans.push(&command.source);
        }
        for constraint in &self.constraints {
            if !constraint.valid(saved_message)
                || spans.iter().any(|span| span.overlaps(constraint))
            {
                return Err(CommandLedgerError::InvalidManifest);
            }
            spans.push(constraint);
        }
        Ok(())
    }
    fn check_binding(&self, binding: &PreparedCommandBinding) -> Result<(), CommandLedgerError> {
        if binding.scope != self.scope {
            return Err(CommandLedgerError::ScopeMismatch);
        }
        if matches!(self.scope.authority, CommandAuthority::Room { .. })
            && binding.canonical_nonce.is_none()
        {
            return Err(CommandLedgerError::BindingConflict);
        }
        if binding.step == 0
            || binding.tool_call_id.is_empty()
            || binding.tool_call_id.len() > 256
            || binding.tool_call_id.chars().any(char::is_control)
            || binding.canonical_nonce.is_some_and(|nonce| nonce.is_nil())
        {
            return Err(CommandLedgerError::BindingConflict);
        }
        let command = self
            .commands
            .iter()
            .find(|command| command.id == binding.command_id)
            .ok_or(CommandLedgerError::BindingConflict)?;
        let HumanCommandGoal::Exact { action } = &command.goal else {
            return Err(CommandLedgerError::NotReady);
        };
        if binding.terminal {
            if !action.same_mechanics(&binding.signature) {
                return Err(CommandLedgerError::SignatureMismatch);
            }
        } else if !matches!((action,&binding.signature),(HumanAction::Adventure(Action::Move(destination)),HumanAction::Adventure(Action::Move(intermediate))) if destination!=intermediate)
        {
            return Err(CommandLedgerError::SignatureMismatch);
        }
        // Prerequisite metadata does not grant navigation permission. The caller
        // must still prove that this intermediate movement serves the request.
        Ok(())
    }
    pub fn progress(
        &self,
        bindings: &[PreparedCommandBinding],
        receipts: &[ConfirmedCommandAttempt],
    ) -> Result<Vec<(u16, HumanCommandProgress)>, CommandLedgerError> {
        let mut by_call = BTreeMap::new();
        let mut terminals = BTreeMap::new();
        let mut steps = BTreeMap::new();
        let mut nonces = BTreeMap::new();
        for binding in bindings {
            self.check_binding(binding)?;
            if let Some(nonce) = binding.canonical_nonce {
                if let Some(old) = nonces.insert(nonce, binding) {
                    if old != binding {
                        return Err(CommandLedgerError::BindingConflict);
                    }
                }
            }
            if let Some(old) = by_call.insert(binding.tool_call_id.as_str(), binding) {
                if old != binding {
                    return Err(CommandLedgerError::BindingConflict);
                }
            }
            if let Some(old) = steps.insert((binding.command_id, binding.step), binding) {
                if old != binding {
                    return Err(CommandLedgerError::BindingConflict);
                }
            }
            if binding.terminal {
                if let Some(old) = terminals.insert(binding.command_id, binding) {
                    if old != binding {
                        return Err(CommandLedgerError::BindingConflict);
                    }
                }
            }
        }
        let mut outcomes = BTreeMap::new();
        for receipt in receipts {
            self.check_binding(&receipt.binding)?;
            if by_call.get(receipt.binding.tool_call_id.as_str()).copied() != Some(&receipt.binding)
            {
                return Err(CommandLedgerError::SignatureMismatch);
            }
            if let CommandAttemptOutcome::Refused { code } = &receipt.outcome {
                if !super::intentions::definitive_rejection(code) {
                    return Err(CommandLedgerError::OutcomeInvalid);
                }
            }
            if let Some(old) =
                outcomes.insert(receipt.binding.tool_call_id.as_str(), &receipt.outcome)
            {
                if old != &receipt.outcome {
                    return Err(CommandLedgerError::BindingConflict);
                }
            }
        }
        for command in &self.commands {
            let mut expected = 1u16;
            let mut stopped = false;
            for ((_, step), binding) in steps.range((command.id, 0)..=(command.id, u16::MAX)) {
                if *step != expected || stopped {
                    return Err(CommandLedgerError::BindingConflict);
                }
                expected = expected
                    .checked_add(1)
                    .ok_or(CommandLedgerError::BindingConflict)?;
                stopped = binding.terminal
                    || !outcomes.contains_key(binding.tool_call_id.as_str())
                    || matches!(
                        outcomes.get(binding.tool_call_id.as_str()),
                        Some(CommandAttemptOutcome::Refused { .. })
                    );
            }
        }
        let mut progress: BTreeMap<u16, HumanCommandProgress> = BTreeMap::new();
        let mut output = Vec::new();
        for command in &self.commands {
            let earlier_open = output
                .iter()
                .any(|(_, status): &(u16, HumanCommandProgress)| !status.closed());
            let status = if earlier_open {
                HumanCommandProgress::WaitingForPrevious
            } else {
                let active = if let Some(condition) = &command.condition {
                    if progress.get(&condition.command_id)
                        == Some(&HumanCommandProgress::NotTriggered)
                    {
                        Some(false)
                    } else {
                        let outcome = terminals
                            .get(&condition.command_id)
                            .and_then(|binding| outcomes.get(binding.tool_call_id.as_str()))
                            .copied()
                            .or_else(|| {
                                bindings
                                    .iter()
                                    .filter(|binding| binding.command_id == condition.command_id)
                                    .find_map(|binding| {
                                        outcomes.get(binding.tool_call_id.as_str()).copied().filter(
                                            |outcome| {
                                                matches!(
                                                    outcome,
                                                    CommandAttemptOutcome::Refused { .. }
                                                )
                                            },
                                        )
                                    })
                            });
                        match (condition.predicate.clone(), outcome) {
                            (CommandPredicate::AttemptRefused, Some(outcome)) => {
                                Some(matches!(outcome, CommandAttemptOutcome::Refused { .. }))
                            }
                            (CommandPredicate::AttemptRefusedBecause { code }, Some(outcome)) => {
                                Some(
                                    matches!(outcome,CommandAttemptOutcome::Refused {code:actual} if *actual==code),
                                )
                            }
                            (
                                CommandPredicate::CheckSucceeded,
                                Some(CommandAttemptOutcome::Performed {
                                    check_success: Some(success),
                                }),
                            ) => Some(*success),
                            (
                                CommandPredicate::CheckFailed,
                                Some(CommandAttemptOutcome::Performed {
                                    check_success: Some(success),
                                }),
                            ) => Some(!success),
                            _ => None,
                        }
                    }
                } else {
                    Some(true)
                };
                match active {
                    None => HumanCommandProgress::WaitingForCondition,
                    Some(false) => {
                        if bindings
                            .iter()
                            .any(|binding| binding.command_id == command.id)
                        {
                            return Err(CommandLedgerError::BindingConflict);
                        }
                        HumanCommandProgress::NotTriggered
                    }
                    Some(true)
                        if bindings
                            .iter()
                            .filter(|binding| binding.command_id == command.id)
                            .any(|binding| {
                                matches!(
                                    outcomes.get(binding.tool_call_id.as_str()),
                                    Some(CommandAttemptOutcome::Refused { .. })
                                )
                            }) =>
                    {
                        HumanCommandProgress::Finished
                    }
                    Some(true)
                        if bindings
                            .iter()
                            .filter(|binding| binding.command_id == command.id)
                            .any(|binding| {
                                !outcomes.contains_key(binding.tool_call_id.as_str())
                            }) =>
                    {
                        HumanCommandProgress::Prepared
                    }
                    Some(true) => match terminals.get(&command.id) {
                        Some(binding) if outcomes.contains_key(binding.tool_call_id.as_str()) => {
                            HumanCommandProgress::Finished
                        }
                        Some(_) => HumanCommandProgress::Prepared,
                        None if matches!(
                            command.goal,
                            HumanCommandGoal::NeedsResolution { .. }
                        ) =>
                        {
                            HumanCommandProgress::NeedsResolution
                        }
                        None => HumanCommandProgress::Ready,
                    },
                }
            };
            if matches!(
                status,
                HumanCommandProgress::WaitingForPrevious
                    | HumanCommandProgress::WaitingForCondition
            ) && bindings
                .iter()
                .any(|binding| binding.command_id == command.id)
            {
                return Err(CommandLedgerError::BindingConflict);
            }
            progress.insert(command.id, status);
            output.push((command.id, status));
        }
        Ok(output)
    }
    /// Consistency check after permission. Preparing does not mean performed.
    pub fn prepare_binding(
        &self,
        new: &PreparedCommandBinding,
        bindings: &[PreparedCommandBinding],
        receipts: &[ConfirmedCommandAttempt],
    ) -> Result<(), CommandLedgerError> {
        self.check_binding(new)?;
        let progress = self.progress(bindings, receipts)?;
        if let Some(existing) = bindings
            .iter()
            .find(|binding| binding.tool_call_id == new.tool_call_id)
        {
            return if existing == new {
                Ok(())
            } else {
                Err(CommandLedgerError::BindingConflict)
            };
        }
        if new.canonical_nonce.is_some()
            && bindings
                .iter()
                .any(|binding| binding.canonical_nonce == new.canonical_nonce)
        {
            return Err(CommandLedgerError::BindingConflict);
        }
        let last = bindings
            .iter()
            .filter(|binding| binding.command_id == new.command_id)
            .map(|binding| binding.step)
            .max()
            .unwrap_or(0);
        if last.checked_add(1) != Some(new.step) {
            return Err(CommandLedgerError::BindingConflict);
        }
        if progress
            .iter()
            .find(|(id, _)| *id == new.command_id)
            .map(|(_, state)| *state)
            != Some(HumanCommandProgress::Ready)
        {
            return Err(CommandLedgerError::NotReady);
        }
        Ok(())
    }
}
