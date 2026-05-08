//! State machine for a single sidecar process (LLM or image-gen).
//!
//! The runtime owns the spawned `SidecarHandle` plus a snapshot of the last
//! reported `RuntimeStatus`. `start` runs the spawn syscall, then drives the
//! caller-supplied probe to confirm liveness; `stop` kills the child and
//! resets to `Off`. Multi-sidecar orchestration lives in the parent module
//! (`RuntimeRegistry`, added later).

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use app_llm::sidecar_launcher::{SidecarHandle, SidecarLauncher};
use tokio::sync::Mutex;

use crate::local_runtime::health::{probe_until_ready, ProbeConfig, ProbeError};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum RuntimeStatus {
    Off,
    Starting,
    Ready { port: u16 },
    Failed { reason: String },
}

/// Erased async closure used to probe a sidecar's HTTP health endpoint.
/// Production wires `probe_real(ProbeConfig)`; tests pass `probe_always_ok` /
/// `probe_always_fail` / a counter-based helper.
pub type ProbeFn = Arc<
    dyn Fn(&str) -> Pin<Box<dyn Future<Output = Result<(), ProbeError>> + Send>> + Send + Sync,
>;

pub struct LocalRuntime {
    launcher: Arc<dyn SidecarLauncher>,
    probe: ProbeFn,
    handle: Mutex<Option<SidecarHandle>>,
    status: Mutex<RuntimeStatus>,
}

impl LocalRuntime {
    pub fn new(launcher: Arc<dyn SidecarLauncher>, probe: ProbeFn) -> Self {
        Self {
            launcher,
            probe,
            handle: Mutex::new(None),
            status: Mutex::new(RuntimeStatus::Off),
        }
    }

    /// Spawn the sidecar and run the probe at `http://127.0.0.1:{port}/health`.
    /// Caller supplies `port` because `discover_free_port` runs before spawn
    /// and is passed to the child via `--port` (mistralrs-server) or `--port`
    /// (Python SDXL); the runtime never parses port out of stdout.
    pub async fn start(
        &self,
        name: &str,
        args: &[&str],
        port: u16,
    ) -> Result<RuntimeStatus, std::io::Error> {
        *self.status.lock().await = RuntimeStatus::Starting;
        let handle = self
            .launcher
            .spawn(name, args)
            .await
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        let url = format!("http://127.0.0.1:{port}/health");
        match (self.probe)(&url).await {
            Ok(()) => {
                *self.handle.lock().await = Some(handle);
                let status = RuntimeStatus::Ready { port };
                *self.status.lock().await = status.clone();
                Ok(status)
            }
            Err(e) => {
                let _ = handle.kill();
                let status = RuntimeStatus::Failed {
                    reason: e.to_string(),
                };
                *self.status.lock().await = status.clone();
                Ok(status)
            }
        }
    }

    pub async fn stop(&self) -> Result<(), std::io::Error> {
        if let Some(h) = self.handle.lock().await.take() {
            let _ = h.kill();
        }
        *self.status.lock().await = RuntimeStatus::Off;
        Ok(())
    }

    pub async fn status(&self) -> RuntimeStatus {
        self.status.lock().await.clone()
    }
}

pub fn probe_always_ok() -> ProbeFn {
    Arc::new(|_url| Box::pin(async { Ok(()) }))
}

pub fn probe_always_fail() -> ProbeFn {
    Arc::new(|_url| {
        Box::pin(async { Err(ProbeError::ExhaustedAttempts { attempts: 0 }) })
    })
}

pub fn probe_real(cfg: ProbeConfig) -> ProbeFn {
    Arc::new(move |url: &str| {
        let url = url.to_string();
        Box::pin(async move { probe_until_ready(&url, cfg).await })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_llm::sidecar_launcher::{MockSidecarLauncher, SpawnSpec};

    #[tokio::test]
    async fn ready_after_successful_spawn_and_probe() {
        let mut launcher = MockSidecarLauncher::default();
        launcher.expect_spawn(SpawnSpec {
            command: "mistralrs-server".into(),
            args: vec![],
            stdout_lines: vec![],
        });
        let runtime = LocalRuntime::new(Arc::new(launcher), probe_always_ok());
        let status = runtime.start("mistralrs-server", &[], 37000).await.unwrap();
        assert!(matches!(status, RuntimeStatus::Ready { port: 37000 }));
    }

    #[tokio::test]
    async fn failed_when_probe_exhausts_retries() {
        let mut launcher = MockSidecarLauncher::default();
        launcher.expect_spawn(SpawnSpec {
            command: "mistralrs-server".into(),
            args: vec![],
            stdout_lines: vec![],
        });
        let runtime = LocalRuntime::new(Arc::new(launcher), probe_always_fail());
        let result = runtime.start("mistralrs-server", &[], 37001).await;
        assert!(matches!(result, Ok(RuntimeStatus::Failed { .. })));
    }

    #[tokio::test]
    async fn stop_resets_status_to_off() {
        let mut launcher = MockSidecarLauncher::default();
        launcher.expect_spawn(SpawnSpec {
            command: "mistralrs-server".into(),
            args: vec![],
            stdout_lines: vec![],
        });
        let runtime = LocalRuntime::new(Arc::new(launcher), probe_always_ok());
        let _ = runtime.start("mistralrs-server", &[], 37002).await.unwrap();
        runtime.stop().await.unwrap();
        assert!(matches!(runtime.status().await, RuntimeStatus::Off));
    }
}
