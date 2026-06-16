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
use std::time::Duration;

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
pub type ProbeFn =
    Arc<dyn Fn(&str) -> Pin<Box<dyn Future<Output = Result<(), ProbeError>> + Send>> + Send + Sync>;

pub struct LocalRuntime {
    launcher: Arc<dyn SidecarLauncher>,
    probe: ProbeFn,
    health_path: String,
    handle: Mutex<Option<SidecarHandle>>,
    status: Mutex<RuntimeStatus>,
}

impl LocalRuntime {
    pub fn new(
        launcher: Arc<dyn SidecarLauncher>,
        probe: ProbeFn,
        health_path: impl Into<String>,
    ) -> Self {
        Self {
            launcher,
            probe,
            health_path: health_path.into(),
            handle: Mutex::new(None),
            status: Mutex::new(RuntimeStatus::Off),
        }
    }

    /// Spawn the sidecar and run the probe at
    /// `http://127.0.0.1:{port}{health_path}`, where `health_path` was supplied
    /// to `new` (per-runtime: mistralrs-server uses `/health`, the Python image
    /// sidecar uses `/healthz`). Caller supplies `port` because
    /// `discover_free_port` runs before spawn and is passed to the child via
    /// `--port`; the runtime never parses port out of stdout.
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
        let url = format!("http://127.0.0.1:{port}{}", self.health_path);

        // Race the health probe against a liveness watcher. If the sidecar dies
        // mid-startup (e.g. mistralrs loads a GGUF then exits before binding its
        // port on a non-TTY), fail fast instead of polling its health endpoint
        // for the whole probe budget.
        let liveness = handle.liveness();
        let probe = (self.probe)(&url);
        tokio::pin!(probe);
        let result: Result<(), String> = tokio::select! {
            res = &mut probe => res.map_err(|e| e.to_string()),
            () = async {
                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    if !liveness() {
                        return;
                    }
                }
            } => Err("sidecar process exited during startup".to_string()),
        };

        match result {
            Ok(()) => {
                *self.handle.lock().await = Some(handle);
                let status = RuntimeStatus::Ready { port };
                *self.status.lock().await = status.clone();
                Ok(status)
            }
            Err(reason) => {
                let _ = handle.kill();
                let status = RuntimeStatus::Failed { reason };
                *self.status.lock().await = status.clone();
                Ok(status)
            }
        }
    }

    /// Like `start`, but retries up to `max_attempts` times with exponential
    /// backoff when the probe fails. Used by callers that want to absorb a
    /// transient sidecar startup race (e.g. mistralrs-server still loading
    /// model weights).
    pub async fn start_with_retry(
        &self,
        name: &str,
        args: &[&str],
        port: u16,
        max_attempts: u32,
    ) -> Result<RuntimeStatus, std::io::Error> {
        let mut delay = Duration::from_millis(500);
        for attempt in 0..max_attempts {
            let status = self.start(name, args, port).await?;
            if matches!(status, RuntimeStatus::Ready { .. }) {
                return Ok(status);
            }
            if attempt + 1 < max_attempts {
                tokio::time::sleep(delay).await;
                delay *= 2;
            }
        }
        Ok(self.status().await)
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
    Arc::new(|_url| Box::pin(async { Err(ProbeError::ExhaustedAttempts { attempts: 0 }) }))
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
        let runtime = LocalRuntime::new(Arc::new(launcher), probe_always_ok(), "/health");
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
        let runtime = LocalRuntime::new(Arc::new(launcher), probe_always_fail(), "/health");
        let result = runtime.start("mistralrs-server", &[], 37001).await;
        assert!(matches!(result, Ok(RuntimeStatus::Failed { .. })));
    }

    /// A launcher whose handle reports the child as already dead, so the
    /// liveness watcher fires while the (forever-blocking) probe is still
    /// pending - exercising the fast-fail path.
    struct DeadLauncher;

    #[async_trait::async_trait]
    impl app_llm::sidecar_launcher::SidecarLauncher for DeadLauncher {
        async fn spawn(
            &self,
            _name: &str,
            _args: &[&str],
        ) -> Result<app_llm::sidecar_launcher::SidecarHandle, app_llm::sidecar_launcher::SidecarError>
        {
            let (_tx, rx) = tokio::sync::mpsc::channel(1);
            Ok(app_llm::sidecar_launcher::SidecarHandle::from_parts(
                0,
                rx,
                || false,
                || Ok(()),
            ))
        }
    }

    fn probe_blocks_forever() -> ProbeFn {
        Arc::new(|_url| Box::pin(std::future::pending::<Result<(), ProbeError>>()))
    }

    #[tokio::test]
    async fn fails_fast_when_sidecar_dies_during_startup() {
        let runtime = LocalRuntime::new(Arc::new(DeadLauncher), probe_blocks_forever(), "/health");
        let status = runtime.start("x", &[], 37999).await.unwrap();
        match &status {
            RuntimeStatus::Failed { reason } => {
                assert!(reason.contains("exited"), "unexpected reason: {reason}")
            }
            other => panic!("expected fast-fail Failed, got {other:?}"),
        }
    }

    fn probe_third_attempt_ok() -> ProbeFn {
        use std::sync::atomic::{AtomicU32, Ordering};
        let counter = Arc::new(AtomicU32::new(0));
        Arc::new(move |_url| {
            let counter = counter.clone();
            Box::pin(async move {
                let attempt = counter.fetch_add(1, Ordering::SeqCst);
                if attempt >= 2 {
                    Ok(())
                } else {
                    Err(ProbeError::ExhaustedAttempts { attempts: 1 })
                }
            })
        })
    }

    #[tokio::test]
    async fn restart_succeeds_on_third_attempt() {
        let mut launcher = MockSidecarLauncher::default();
        for _ in 0..3 {
            launcher.expect_spawn(SpawnSpec {
                command: "mistralrs-server".into(),
                args: vec![],
                stdout_lines: vec![],
            });
        }
        let runtime = LocalRuntime::new(Arc::new(launcher), probe_third_attempt_ok(), "/health");
        let status = runtime
            .start_with_retry("mistralrs-server", &[], 37100, 3)
            .await
            .unwrap();
        assert!(matches!(status, RuntimeStatus::Ready { .. }));
    }

    #[tokio::test]
    async fn stop_resets_status_to_off() {
        let mut launcher = MockSidecarLauncher::default();
        launcher.expect_spawn(SpawnSpec {
            command: "mistralrs-server".into(),
            args: vec![],
            stdout_lines: vec![],
        });
        let runtime = LocalRuntime::new(Arc::new(launcher), probe_always_ok(), "/health");
        let _ = runtime.start("mistralrs-server", &[], 37002).await.unwrap();
        runtime.stop().await.unwrap();
        assert!(matches!(runtime.status().await, RuntimeStatus::Off));
    }

    #[tokio::test]
    async fn probe_receives_the_configured_health_path() {
        use std::sync::Mutex as StdMutex;
        let seen: Arc<StdMutex<Option<String>>> = Arc::new(StdMutex::new(None));
        let seen_clone = seen.clone();
        let probe: ProbeFn = Arc::new(move |url: &str| {
            *seen_clone.lock().unwrap() = Some(url.to_string());
            Box::pin(async { Ok(()) })
        });
        let mut launcher = MockSidecarLauncher::default();
        launcher.expect_spawn(SpawnSpec {
            command: "x".into(),
            args: vec![],
            stdout_lines: vec![],
        });
        let runtime = LocalRuntime::new(Arc::new(launcher), probe, "/healthz");
        let _ = runtime.start("x", &[], 41000).await.unwrap();
        assert_eq!(
            seen.lock().unwrap().as_deref(),
            Some("http://127.0.0.1:41000/healthz")
        );
    }
}
