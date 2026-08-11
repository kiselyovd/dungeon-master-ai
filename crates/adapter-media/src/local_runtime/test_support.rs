use std::sync::Mutex;

use app_application::ports::runtime::{SidecarError, SidecarHandle, SidecarLauncher, SpawnSpec};
use async_trait::async_trait;
use tokio::sync::mpsc;

#[derive(Default)]
pub struct MockSidecarLauncher {
    expectations: Mutex<Vec<SpawnSpec>>,
}

impl MockSidecarLauncher {
    pub fn expect_spawn(&mut self, spec: SpawnSpec) {
        self.expectations
            .lock()
            .expect("expectations lock")
            .push(spec);
    }
}

#[async_trait]
impl SidecarLauncher for MockSidecarLauncher {
    async fn spawn(&self, name: &str, _args: &[&str]) -> Result<SidecarHandle, SidecarError> {
        let spec = self
            .expectations
            .lock()
            .expect("expectations lock")
            .pop()
            .ok_or_else(|| SidecarError::MockUnconfigured(name.into()))?;
        let (tx, rx) = mpsc::channel(spec.stdout_lines.len().max(1));
        for line in spec.stdout_lines {
            let _ = tx.try_send(line);
        }
        Ok(SidecarHandle::from_parts(0, rx, || true, || Ok(())))
    }
}

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
