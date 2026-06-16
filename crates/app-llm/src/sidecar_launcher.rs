//! Abstraction over `tauri::process::Command::new_sidecar` so non-Tauri
//! contexts (Rust integration tests, headless CI) can drive sidecars without
//! a Tauri app handle. Production wires through `TauriSidecarLauncher` (lives
//! in `src-tauri`); `MockSidecarLauncher` covers unit tests here.

use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, Mutex as AsyncMutex};

#[derive(Debug, thiserror::Error)]
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

/// Declarative expectation supplied to `MockSidecarLauncher`. Each call to
/// `spawn` consumes one `SpawnSpec` and replays its `stdout_lines`.
#[derive(Debug, Clone)]
pub struct SpawnSpec {
    pub command: String,
    pub args: Vec<String>,
    pub stdout_lines: Vec<String>,
}

type KillFn = Box<dyn FnOnce() -> Result<(), SidecarError> + Send + Sync>;
/// Cheap, repeatable liveness check: `true` while the child is still running,
/// `false` once it has exited (or its status can no longer be read).
type LivenessFn = Arc<dyn Fn() -> bool + Send + Sync>;

pub struct SidecarHandle {
    pub child_pid: u32,
    stdout_rx: AsyncMutex<Option<mpsc::Receiver<String>>>,
    kill: KillFn,
    liveness: LivenessFn,
}

impl SidecarHandle {
    /// Awaits the next line from the sidecar's stdout. Returns `None` once the
    /// channel has been closed and drained. The receiver lives behind an async
    /// `Mutex` so the guard can safely span the `recv().await`.
    pub async fn first_stdout_line(&self) -> Option<String> {
        let mut guard = self.stdout_rx.lock().await;
        guard.as_mut()?.recv().await
    }

    pub fn kill(self) -> Result<(), SidecarError> {
        (self.kill)()
    }

    /// True while the child process is still running. Used by the runtime to
    /// fail fast if a sidecar dies during startup instead of polling its health
    /// endpoint for the whole probe budget. (A GGUF mistralrs run, for example,
    /// loads the model and then exits before binding its port on a non-TTY.)
    pub fn is_alive(&self) -> bool {
        (self.liveness)()
    }

    /// A cloneable handle to the liveness check, so a watcher task can outlive a
    /// borrow of `&self`.
    pub fn liveness(&self) -> LivenessFn {
        self.liveness.clone()
    }

    /// Constructor used by production launcher (Task B.3). Kept here so the
    /// `SidecarHandle` type remains the single source of truth for handle layout.
    /// Accepts any matching closures; the boxing happens internally so callers
    /// don't need to know about the private `KillFn`/`LivenessFn` aliases.
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
    /// Spawn a sidecar child process.
    ///
    /// `Ok(SidecarHandle)` only guarantees that the spawn syscall succeeded -
    /// the child may have exited immediately (e.g., bad CLI args, missing
    /// shared library). Callers MUST verify liveness with a health-probe
    /// before treating the handle as ready (see `LocalRuntime::probe_until_ready`).
    ///
    /// Errors:
    /// - `SidecarError::Spawn` - the underlying `Command::spawn` failed.
    /// - `SidecarError::UnexpectedExit` - reserved for a future `wait_for_exit`
    ///   helper; not produced by `spawn` itself.
    async fn spawn(&self, name: &str, args: &[&str]) -> Result<SidecarHandle, SidecarError>;
}

/// In-memory launcher used in unit tests. Configure expectations with
/// `expect_spawn`; calls to `spawn` consume them in LIFO order
/// (`Vec::pop` from the back).
#[derive(Default)]
pub struct MockSidecarLauncher {
    expectations: Mutex<Vec<SpawnSpec>>,
}

impl MockSidecarLauncher {
    pub fn expect_spawn(&mut self, spec: SpawnSpec) {
        self.expectations.lock().unwrap().push(spec);
    }
}

#[async_trait]
impl SidecarLauncher for MockSidecarLauncher {
    async fn spawn(&self, name: &str, _args: &[&str]) -> Result<SidecarHandle, SidecarError> {
        // Drain the expectation and the lines we want to replay before
        // touching the channel, so we never hold the Mutex across an await.
        let spec = {
            let mut q = self.expectations.lock().unwrap();
            q.pop()
                .ok_or_else(|| SidecarError::MockUnconfigured(name.into()))?
        };
        // Size the channel exactly to the configured stdout lines so we never
        // silently drop on `try_send`. `.max(1)` because `mpsc::channel` panics
        // on capacity 0.
        let cap = spec.stdout_lines.len().max(1);
        let (tx, rx) = mpsc::channel(cap);
        for line in spec.stdout_lines {
            let _ = tx.try_send(line);
        }
        Ok(SidecarHandle {
            child_pid: 0,
            stdout_rx: AsyncMutex::new(Some(rx)),
            kill: Box::new(|| Ok(())),
            liveness: Arc::new(|| true),
        })
    }
}

/// Stub launcher used as the default in `AppState::new` before a real
/// production launcher (Tauri sidecar plugin) is wired in. Every `spawn`
/// call fails with `SidecarError::Spawn` carrying a stable error message
/// so callers can detect "local runtime not configured" deterministically.
#[derive(Default)]
pub struct NullSidecarLauncher;

#[async_trait]
impl SidecarLauncher for NullSidecarLauncher {
    async fn spawn(&self, name: &str, _args: &[&str]) -> Result<SidecarHandle, SidecarError> {
        Err(SidecarError::Spawn {
            name: name.into(),
            source: std::io::Error::other("local runtime not configured"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_launcher_returns_configured_handle() {
        let mut mock = MockSidecarLauncher::default();
        mock.expect_spawn(SpawnSpec {
            command: "fake-bin".into(),
            args: vec!["--port".into(), "12345".into()],
            stdout_lines: vec!["LISTENING_ON_PORT=12345".into()],
        });
        let handle = mock.spawn("fake-bin", &["--port", "12345"]).await.unwrap();
        assert_eq!(
            handle.first_stdout_line().await.unwrap(),
            "LISTENING_ON_PORT=12345"
        );
    }

    #[tokio::test]
    async fn mock_launcher_drains_expectations_in_lifo_order() {
        let mut mock = MockSidecarLauncher::default();
        mock.expect_spawn(SpawnSpec {
            command: "first-pushed".into(),
            args: vec![],
            stdout_lines: vec!["FIRST".into()],
        });
        mock.expect_spawn(SpawnSpec {
            command: "second-pushed".into(),
            args: vec![],
            stdout_lines: vec!["SECOND".into()],
        });
        let handle1 = mock.spawn("any", &[]).await.unwrap();
        assert_eq!(handle1.first_stdout_line().await.unwrap(), "SECOND");
        let handle2 = mock.spawn("any", &[]).await.unwrap();
        assert_eq!(handle2.first_stdout_line().await.unwrap(), "FIRST");
    }

    #[tokio::test]
    async fn mock_launcher_returns_unconfigured_when_queue_empty() {
        let mock = MockSidecarLauncher::default();
        let result = mock.spawn("never-configured", &[]).await;
        assert!(
            matches!(result, Err(SidecarError::MockUnconfigured(name)) if name == "never-configured")
        );
    }
}
