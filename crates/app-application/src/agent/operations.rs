//! Durable delivery receipts. Incomplete operations are explicitly distinguishable
//! from completed turns; they must not be silently re-executed.
use crate::models::agent::{AgentEvent, AgentTurnRequest};
use crate::models::chat::ChatMessage;
use crate::ports::repositories::RepositoryError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentOperation {
    pub request_id: Uuid,
    pub request: AgentTurnRequest,
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OperationClaim {
    Started,
    Completed(Vec<AgentEvent>),
    Running,
    Interrupted,
    Resuming {
        messages: Vec<ChatMessage>,
        events: Vec<AgentEvent>,
    },
}

pub enum ToolStepClaim {
    Started,
    Pending,
    Completed(super::tool_dispatch::ToolExecution),
}

#[async_trait]
pub trait AgentOperationRepository: Send + Sync {
    async fn resume(
        &self,
        _input: AgentOperation,
        _owner: Uuid,
    ) -> Result<OperationClaim, RepositoryError> {
        Err(RepositoryError::Operation {
            operation: "resume_agent_turn",
            code: "agent_turn_recovery_unavailable",
        })
    }
    async fn claim(
        &self,
        input: AgentOperation,
        owner: Uuid,
    ) -> Result<OperationClaim, RepositoryError>;
    async fn append_event(
        &self,
        request_id: Uuid,
        owner: Uuid,
        sequence: u64,
        event: AgentEvent,
    ) -> Result<(), RepositoryError>;
    async fn finish(
        &self,
        request_id: Uuid,
        owner: Uuid,
        complete: bool,
    ) -> Result<(), RepositoryError>;
}

#[derive(Default)]
pub struct TurnCancellation {
    cancelled: AtomicBool,
    wake: tokio::sync::Notify,
}
impl TurnCancellation {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.wake.notify_waiters();
    }
}
#[async_trait]
impl super::turn::CancellationSignal for TurnCancellation {
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
    async fn cancelled(&self) {
        let notified = self.wake.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        if !self.is_cancelled() {
            notified.await;
        }
    }
}

#[async_trait]
pub trait AgentOperationDriver: Send + Sync {
    /// The same authority lease covers claim, checkpoint validation, execution,
    /// and final receipt. Restore cannot slip between those phases.
    async fn acquire(&self) -> Box<dyn Send> {
        Box::new(())
    }
    async fn run(
        &self,
        input: AgentOperation,
        events: mpsc::Sender<AgentEvent>,
        cancellation: Arc<TurnCancellation>,
    ) -> Result<(), &'static str>;
    async fn resume(
        &self,
        _input: AgentOperation,
        _messages: Vec<ChatMessage>,
        _events: mpsc::Sender<AgentEvent>,
        _cancellation: Arc<TurnCancellation>,
    ) -> Result<(), &'static str> {
        Err("driver_recovery_unavailable")
    }
}

pub struct DurableAgentTurns<R, D> {
    repository: Arc<R>,
    driver: Arc<D>,
    owner: Uuid,
}
impl<R: AgentOperationRepository + 'static, D: AgentOperationDriver + 'static>
    DurableAgentTurns<R, D>
{
    pub fn new(repository: Arc<R>, driver: Arc<D>, owner: Uuid) -> Self {
        Self {
            repository,
            driver,
            owner,
        }
    }
    pub async fn execute(
        &self,
        input: AgentOperation,
    ) -> Result<mpsc::Receiver<AgentEvent>, RepositoryError> {
        self.execute_mode(input, false).await
    }
    pub async fn execute_mode(
        &self,
        input: AgentOperation,
        resume: bool,
    ) -> Result<mpsc::Receiver<AgentEvent>, RepositoryError> {
        let lease = self.driver.acquire().await;
        let claim = if resume {
            self.repository.resume(input.clone(), self.owner).await?
        } else {
            self.repository.claim(input.clone(), self.owner).await?
        };
        let (sender, receiver) = mpsc::channel(64);
        match claim {
            OperationClaim::Completed(events) => {
                drop(lease);
                tokio::spawn(async move {
                    for event in events {
                        if sender.send(event).await.is_err() {
                            break;
                        }
                    }
                });
            }
            OperationClaim::Running => {
                return Err(RepositoryError::Operation {
                    operation: "claim_agent_turn",
                    code: "agent_turn_running",
                })
            }
            OperationClaim::Interrupted => {
                return Err(RepositoryError::Operation {
                    operation: "claim_agent_turn",
                    code: "agent_turn_interrupted",
                })
            }
            claim @ (OperationClaim::Started | OperationClaim::Resuming { .. }) => {
                let (recovered_messages, recovered_events) = match claim {
                    OperationClaim::Resuming { messages, events } => (Some(messages), events),
                    _ => (None, vec![]),
                };
                let repository = self.repository.clone();
                let driver = self.driver.clone();
                let owner = self.owner;
                tokio::spawn(async move {
                    let _lease = lease;
                    let (engine_sender, mut engine_receiver) = mpsc::channel(64);
                    let cancellation = Arc::new(TurnCancellation::default());
                    let driver_cancellation = cancellation.clone();
                    let request_id = input.request_id;
                    let engine = tokio::spawn(async move {
                        match recovered_messages {
                            Some(messages) => {
                                driver
                                    .resume(input, messages, engine_sender, driver_cancellation)
                                    .await
                            }
                            None => driver.run(input, engine_sender, driver_cancellation).await,
                        }
                    });
                    let mut sequence = recovered_events.len() as u64;
                    for event in recovered_events {
                        if sender.send(event).await.is_err() {
                            cancellation.cancel();
                            break;
                        }
                    }
                    let mut saw_done = false;
                    let mut terminal_event = None;
                    let mut recording_ok = true;
                    let mut disconnected = false;
                    loop {
                        let event = tokio::select! {
                            _=sender.closed(),if !disconnected=>{disconnected=true;cancellation.cancel();continue;},
                            event=engine_receiver.recv()=>match event{Some(event)=>event,None=>break},
                        };
                        let event = scoped_event(request_id, event);
                        if repository
                            .append_event(request_id, owner, sequence, event.clone())
                            .await
                            .is_err()
                        {
                            recording_ok = false;
                            cancellation.cancel();
                            break;
                        }
                        sequence += 1;
                        saw_done |= matches!(event, AgentEvent::AgentDone { .. });
                        if matches!(event, AgentEvent::AgentDone { .. }) {
                            terminal_event = Some(event);
                            continue;
                        }
                        if !disconnected && sender.send(event).await.is_err() {
                            disconnected = true;
                            cancellation.cancel();
                        }
                    }
                    drop(engine_receiver);
                    let succeeded = matches!(engine.await, Ok(Ok(())));
                    let complete = recording_ok && succeeded && saw_done;
                    let finished = repository.finish(request_id, owner, complete).await;
                    if finished.is_err() {
                        tracing::error!(%request_id,code="agent_operation_finish_failed");
                    } else if complete && !disconnected {
                        if let Some(event) = terminal_event {
                            let _ = sender.send(event).await;
                        }
                    }
                });
            }
        }
        Ok(receiver)
    }
}

fn scoped_event(request_id: Uuid, mut event: AgentEvent) -> AgentEvent {
    match &mut event {
        AgentEvent::ToolCallStart { id, .. } | AgentEvent::ToolCallResult { id, .. } => {
            *id = format!("{request_id}:{id}")
        }
        AgentEvent::ImageGenerated { tool_call_id, .. }
        | AgentEvent::VideoGenerated { tool_call_id, .. } => {
            *tool_call_id = format!("{request_id}:{tool_call_id}")
        }
        _ => {}
    }
    event
}
