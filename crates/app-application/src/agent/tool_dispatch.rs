use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use super::commands::AgentToolCommand;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCapability {
    Dice,
    Combat,
    Campaign,
    Rules,
    Media,
}

pub fn capability(command: &AgentToolCommand) -> ToolCapability {
    match command {
        AgentToolCommand::RollDice(_) => ToolCapability::Dice,
        AgentToolCommand::ApplyDamage(_)
        | AgentToolCommand::ApplyHealing(_)
        | AgentToolCommand::StartCombat(_)
        | AgentToolCommand::EndCombat(_)
        | AgentToolCommand::AddToken(_)
        | AgentToolCommand::UpdateToken(_)
        | AgentToolCommand::RemoveToken(_)
        | AgentToolCommand::CastSpell(_) => ToolCapability::Combat,
        AgentToolCommand::SetScene(_)
        | AgentToolCommand::RememberNpc(_)
        | AgentToolCommand::RecallNpc(_)
        | AgentToolCommand::JournalAppend(_)
        | AgentToolCommand::QuickSave(_) => ToolCapability::Campaign,
        AgentToolCommand::QueryRules(_) => ToolCapability::Rules,
        AgentToolCommand::GenerateMap(_)
        | AgentToolCommand::GenerateIllustration(_)
        | AgentToolCommand::GenerateVideo(_) => ToolCapability::Media,
    }
}

#[derive(Debug, Clone)]
pub struct ToolDispatchRequest {
    pub request_id: String,
    pub tool_call_id: String,
    pub campaign_id: Uuid,
    pub session_id: Uuid,
    pub command: AgentToolCommand,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GeneratedAgentMedia {
    Image {
        mime_type: String,
        data_b64: String,
        kind: String,
    },
    Video {
        mime_type: String,
        data_b64: String,
        kind: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolExecution {
    pub result: Value,
    pub is_error: bool,
    pub handled_by: String,
    pub media: Vec<GeneratedAgentMedia>,
}

#[derive(Debug, Error)]
#[error("tool dispatch failed with code {code}")]
pub struct ToolDispatchError {
    pub code: &'static str,
}

#[async_trait]
pub trait ToolDispatcher: Send + Sync {
    async fn execute(
        &self,
        request: ToolDispatchRequest,
    ) -> Result<ToolExecution, ToolDispatchError>;
}

#[async_trait]
pub trait ToolCapabilityHandler: Send + Sync {
    async fn execute_capability(
        &self,
        request: ToolDispatchRequest,
    ) -> Result<ToolExecution, ToolDispatchError>;
}

pub struct CapabilityToolDispatcher {
    dice: Arc<dyn ToolCapabilityHandler>,
    combat: Arc<dyn ToolCapabilityHandler>,
    campaign: Arc<dyn ToolCapabilityHandler>,
    rules: Arc<dyn ToolCapabilityHandler>,
    media: Arc<dyn ToolCapabilityHandler>,
}

impl CapabilityToolDispatcher {
    pub fn new(
        dice: Arc<dyn ToolCapabilityHandler>,
        combat: Arc<dyn ToolCapabilityHandler>,
        campaign: Arc<dyn ToolCapabilityHandler>,
        rules: Arc<dyn ToolCapabilityHandler>,
        media: Arc<dyn ToolCapabilityHandler>,
    ) -> Self {
        Self {
            dice,
            combat,
            campaign,
            rules,
            media,
        }
    }
}

#[async_trait]
impl ToolDispatcher for CapabilityToolDispatcher {
    async fn execute(
        &self,
        request: ToolDispatchRequest,
    ) -> Result<ToolExecution, ToolDispatchError> {
        let handler = match capability(&request.command) {
            ToolCapability::Dice => &self.dice,
            ToolCapability::Combat => &self.combat,
            ToolCapability::Campaign => &self.campaign,
            ToolCapability::Rules => &self.rules,
            ToolCapability::Media => &self.media,
        };
        handler.execute_capability(request).await
    }
}

#[derive(Debug, Error)]
#[error("runtime coordination failed with code {code}")]
pub struct RuntimeCoordinationError {
    pub code: &'static str,
}

#[async_trait]
pub trait RuntimeCoordinator: Send + Sync {
    async fn acquire_for(&self, tool_name: &str) -> Result<(), RuntimeCoordinationError>;
    async fn release_for(&self, tool_name: &str) -> Result<(), RuntimeCoordinationError>;
}

pub struct NoopRuntimeCoordinator;

#[async_trait]
impl RuntimeCoordinator for NoopRuntimeCoordinator {
    async fn acquire_for(&self, _tool_name: &str) -> Result<(), RuntimeCoordinationError> {
        Ok(())
    }

    async fn release_for(&self, _tool_name: &str) -> Result<(), RuntimeCoordinationError> {
        Ok(())
    }
}
