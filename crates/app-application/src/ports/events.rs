use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

use crate::models::agent::AgentEvent;

#[derive(Debug, Clone)]
pub enum ApplicationEvent {
    Agent {
        session_id: Uuid,
        sequence: u64,
        event: AgentEvent,
    },
    CombatProjectionChanged {
        encounter_id: Uuid,
        revision: u64,
    },
}

#[derive(Debug, Error)]
#[error("event publication failed with code {code}")]
pub struct EventSinkError {
    pub code: &'static str,
}

#[async_trait]
pub trait ApplicationEventSink: Send + Sync {
    async fn publish(&self, event: ApplicationEvent) -> Result<(), EventSinkError>;
}

pub struct NoopApplicationEventSink;

#[async_trait]
impl ApplicationEventSink for NoopApplicationEventSink {
    async fn publish(&self, _event: ApplicationEvent) -> Result<(), EventSinkError> {
        Ok(())
    }
}
