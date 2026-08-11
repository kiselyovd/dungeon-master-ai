use std::sync::Arc;

use async_trait::async_trait;
use thiserror::Error;
use tokio::sync::{mpsc, Mutex as AsyncMutex};

use crate::models::local_models::{RuntimeStartRequest, RuntimeStatus};

#[derive(Debug, Error)]
pub enum SidecarError {
    #[error("failed to spawn sidecar `{name}`: {source}")]
    Spawn {
        name: String,
        source: std::io::Error,
    },
    #[error("sidecar `{name}` exited unexpectedly: code={code:?}")]
    UnexpectedExit { name: String, code: Option<i32> },
    #[error("mock launcher: no expectation configured for `{0}`")]
    MockUnconfigured(String),
}

#[derive(Debug, Clone)]
pub struct SpawnSpec {
    pub command: String,
    pub args: Vec<String>,
    pub stdout_lines: Vec<String>,
}

type KillFn = Box<dyn FnOnce() -> Result<(), SidecarError> + Send + Sync>;
pub type LivenessCheck = Arc<dyn Fn() -> bool + Send + Sync>;

pub struct SidecarHandle {
    pub child_pid: u32,
    stdout_rx: AsyncMutex<Option<mpsc::Receiver<String>>>,
    kill: KillFn,
    liveness: LivenessCheck,
}

impl SidecarHandle {
    pub async fn first_stdout_line(&self) -> Option<String> {
        let mut guard = self.stdout_rx.lock().await;
        guard.as_mut()?.recv().await
    }

    pub fn kill(self) -> Result<(), SidecarError> {
        (self.kill)()
    }

    pub fn is_alive(&self) -> bool {
        (self.liveness)()
    }

    pub fn liveness(&self) -> LivenessCheck {
        self.liveness.clone()
    }

    pub fn from_parts(
        pid: u32,
        rx: mpsc::Receiver<String>,
        is_alive: impl Fn() -> bool + Send + Sync + 'static,
        kill: impl FnOnce() -> Result<(), SidecarError> + Send + Sync + 'static,
    ) -> Self {
        Self {
            child_pid: pid,
            stdout_rx: AsyncMutex::new(Some(rx)),
            kill: Box::new(kill),
            liveness: Arc::new(is_alive),
        }
    }
}

#[async_trait]
pub trait SidecarLauncher: Send + Sync {
    async fn spawn(&self, name: &str, args: &[&str]) -> Result<SidecarHandle, SidecarError>;
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("runtime operation {operation} failed with code {code}")]
    Operation {
        operation: &'static str,
        code: &'static str,
    },
}

#[async_trait]
pub trait RuntimeControl: Send + Sync {
    async fn start(&self, request: RuntimeStartRequest) -> Result<RuntimeStatus, RuntimeError>;
    async fn stop(&self) -> Result<RuntimeStatus, RuntimeError>;
    async fn status(&self) -> Result<RuntimeStatus, RuntimeError>;
}
