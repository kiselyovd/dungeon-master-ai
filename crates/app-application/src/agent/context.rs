use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AgentContextRequest {
    pub campaign_id: Uuid,
    pub player_message: String,
    pub base_system_prompt: String,
    pub embedding_model: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentContext {
    pub system_prompt: String,
    pub combat_active: bool,
}

#[derive(Debug, Error)]
#[error("agent context port failed with code {code}")]
pub struct AgentContextError {
    pub code: &'static str,
}

#[async_trait]
pub trait AgentContextPort: Send + Sync {
    async fn build(&self, request: AgentContextRequest) -> Result<AgentContext, AgentContextError>;
}
