//! One shared Dungeon Master lane per campaign, with owner-scoped durable delivery.
use super::agent::RoomAgentPorts;
use crate::{
    agent::{
        operations::{
            AgentOperation, AgentOperationDriver, AgentOperationRepository, DurableAgentTurns,
            OperationClaim, TurnCancellation,
        },
        tool_dispatch::NoopRuntimeCoordinator,
        turn::{AgentTurnCommand, AgentTurnConfig, AgentTurnService, CancellationSignal},
    },
    models::{agent::AgentEvent, chat::ChatMessage},
    ports::{events::NoopApplicationEventSink, llm::LlmProvider, repositories::RepositoryError},
};
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, OwnedMutexGuard};
use uuid::Uuid;
fn rejected(code: &'static str) -> RepositoryError {
    RepositoryError::Operation {
        operation: "room_dm_turn",
        code,
    }
}
#[async_trait::async_trait]
impl AgentOperationRepository for RoomAgentPorts {
    async fn claim(
        &self,
        input: AgentOperation,
        owner: Uuid,
    ) -> Result<OperationClaim, RepositoryError> {
        if input.request_id != self.turn_id {
            return Err(rejected("agent_request_mismatch"));
        }
        self.rooms
            .rooms
            .claim_room_dm_turn(self.subject.lease(), input, owner, false)
            .await
    }
    async fn resume(
        &self,
        input: AgentOperation,
        owner: Uuid,
    ) -> Result<OperationClaim, RepositoryError> {
        if input.request_id != self.turn_id {
            return Err(rejected("agent_request_mismatch"));
        }
        self.rooms
            .rooms
            .claim_room_dm_turn(self.subject.lease(), input, owner, true)
            .await
    }
    async fn append_event(
        &self,
        turn: Uuid,
        owner: Uuid,
        sequence: u64,
        event: AgentEvent,
    ) -> Result<(), RepositoryError> {
        if turn != self.turn_id {
            return Err(rejected("agent_request_mismatch"));
        }
        self.rooms
            .rooms
            .append_room_dm_event(self.subject.lease(), turn, owner, sequence, event)
            .await
    }
    async fn finish(&self, turn: Uuid, owner: Uuid, complete: bool) -> Result<(), RepositoryError> {
        if turn != self.turn_id {
            return Err(rejected("agent_request_mismatch"));
        }
        self.rooms
            .rooms
            .finish_room_dm_turn(self.subject.lease(), turn, owner, complete)
            .await
    }
}
/// Providers/configuration and the campaign lane come from trusted host composition.
/// This is a request executor for the shared DM, never a player-character agent.
pub struct RoomDmRunner {
    ports: Arc<RoomAgentPorts>,
    provider: Arc<dyn LlmProvider>,
    config: AgentTurnConfig,
    lane: Arc<Mutex<()>>,
    process_owner: Uuid,
}
impl RoomDmRunner {
    pub fn new(
        ports: Arc<RoomAgentPorts>,
        provider: Arc<dyn LlmProvider>,
        config: AgentTurnConfig,
        lane: Arc<Mutex<()>>,
        process_owner: Uuid,
    ) -> Self {
        Self {
            ports,
            provider,
            config,
            lane,
            process_owner,
        }
    }
    #[tracing::instrument(skip_all,fields(turn_id=%self.ports.turn_id,resume))]
    pub async fn execute(
        &self,
        player_message: String,
        resume: bool,
    ) -> Result<futures::stream::BoxStream<'static, AgentEvent>, RepositoryError> {
        let lane = self
            .lane
            .clone()
            .try_lock_owned()
            .map_err(|_| rejected("agent_turn_running"))?;
        let command = self
            .ports
            .command(player_message)
            .await
            .map_err(|error| match error {
                super::RoomError::Denied => rejected("room_access_denied"),
                super::RoomError::Conflict => rejected("agent_request_mismatch"),
                _ => rejected("room_dm_request_invalid"),
            })?;
        let mut config = self.config.clone();
        let existing = self
            .ports
            .rooms
            .rooms
            .room_dm_request(self.ports.subject.lease(), self.ports.turn_id)
            .await?;
        let input = existing.unwrap_or(AgentOperation {
            request_id: self.ports.turn_id,
            request: command.request,
            model: Some(config.model.clone()),
        });
        if let Some(model) = &input.model {
            config.model = model.clone();
        }
        config.tools = super::agent::scoped_catalog(config.tools);
        let driver = Arc::new(RoomDmDriver {
            ports: self.ports.clone(),
            provider: self.provider.clone(),
            config,
            lane: std::sync::Mutex::new(Some(lane)),
        });
        let receiver = DurableAgentTurns::new(self.ports.clone(), driver, self.process_owner)
            .execute_mode(input, resume)
            .await?;
        Ok(authorized_delivery(self.ports.clone(), receiver))
    }
}
struct RoomDmDriver {
    ports: Arc<RoomAgentPorts>,
    provider: Arc<dyn LlmProvider>,
    config: AgentTurnConfig,
    lane: std::sync::Mutex<Option<OwnedMutexGuard<()>>>,
}
struct RoomCancellation {
    ports: Arc<RoomAgentPorts>,
    caller: Arc<TurnCancellation>,
}
#[async_trait::async_trait]
impl CancellationSignal for RoomCancellation {
    fn is_cancelled(&self) -> bool {
        self.caller.is_cancelled()
    }
    async fn cancelled(&self) {
        loop {
            tokio::select! {_=self.caller.cancelled()=>return,_=tokio::time::sleep(std::time::Duration::from_millis(250))=>{if self.ports.checkpoint().await.is_err() {return;}}}
        }
    }
}
impl RoomDmDriver {
    async fn execute(
        &self,
        input: AgentOperation,
        messages: Option<Vec<ChatMessage>>,
        sender: mpsc::Sender<AgentEvent>,
        cancel: Arc<TurnCancellation>,
    ) -> Result<(), &'static str> {
        let cancellation = Arc::new(RoomCancellation {
            ports: self.ports.clone(),
            caller: cancel,
        });
        let engine = Arc::new(AgentTurnService::new(
            self.provider.clone(),
            self.ports.clone(),
            self.ports.clone(),
            self.ports.clone(),
            Arc::new(NoopRuntimeCoordinator),
            Arc::new(NoopApplicationEventSink),
            cancellation,
            self.config.clone(),
        ));
        let mut stream = engine.execute_recovered(
            AgentTurnCommand {
                request_id: input.request_id.to_string(),
                request: input.request,
            },
            messages,
        );
        while let Some(event) = stream.next().await {
            let event = event.map_err(|_| "room_dm_engine_failed")?;
            sender
                .send(event)
                .await
                .map_err(|_| "room_dm_delivery_closed")?;
        }
        let checkpoint = self.ports.checkpoint().await.map_err(|error| {
            tracing::warn!(error_code=%error,"DM completion checkpoint unavailable");
            "room_dm_checkpoint_failed"
        })?;
        if !matches!(checkpoint.last().map(|entry|&entry.message),Some(ChatMessage::Assistant {content}) if !content.trim().is_empty())
        {
            return Err("room_dm_incomplete");
        }
        Ok(())
    }
}
#[async_trait::async_trait]
impl AgentOperationDriver for RoomDmDriver {
    async fn acquire(&self) -> Box<dyn Send> {
        Box::new(
            self.lane
                .lock()
                .expect("DM lane lock poisoned")
                .take()
                .expect("DM driver lease acquired once"),
        )
    }

    async fn run(
        &self,
        input: AgentOperation,
        events: mpsc::Sender<AgentEvent>,
        cancel: Arc<TurnCancellation>,
    ) -> Result<(), &'static str> {
        self.execute(input, None, events, cancel).await
    }
    async fn resume(
        &self,
        input: AgentOperation,
        messages: Vec<ChatMessage>,
        events: mpsc::Sender<AgentEvent>,
        cancel: Arc<TurnCancellation>,
    ) -> Result<(), &'static str> {
        self.execute(input, Some(messages), events, cancel).await
    }
}

fn authorized_delivery(
    ports: Arc<RoomAgentPorts>,
    receiver: mpsc::Receiver<AgentEvent>,
) -> futures::stream::BoxStream<'static, AgentEvent> {
    // Validate at consumption, including completed replay with queued/backpressured events.
    futures::stream::unfold((ports, receiver), |(ports, mut receiver)| async move {
        loop {
            if let Err(error) = ports.checkpoint().await {
                tracing::warn!(error_code=%error,"DM delivery validation stopped");
                return None;
            }
            let event = tokio::select! {
                event=receiver.recv()=>match event {Some(event)=>event,None=>return None},
                _=tokio::time::sleep(std::time::Duration::from_millis(250))=>continue,
            };
            if let Err(error) = ports.checkpoint().await {
                tracing::warn!(error_code=%error,"DM delivery validation stopped");
                return None;
            }
            return Some((event, (ports, receiver)));
        }
    })
    .boxed()
}
