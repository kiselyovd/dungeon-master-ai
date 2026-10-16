//! Compatibility facade and test adapters for the application runtime port.

use std::sync::Mutex;

use async_trait::async_trait;
use tokio::sync::mpsc;

pub use app_application::ports::runtime::{
    SidecarError, SidecarHandle, SidecarLauncher, SpawnSpec,
};

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
        let spec = {
            let mut queue = self.expectations.lock().unwrap();
            queue
                .pop()
                .ok_or_else(|| SidecarError::MockUnconfigured(name.into()))?
        };
        let capacity = spec.stdout_lines.len().max(1);
        let (tx, rx) = mpsc::channel(capacity);
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
