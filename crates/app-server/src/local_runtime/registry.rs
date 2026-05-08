//! Two-runtime registry: LLM (mistralrs sidecar) + image (Python SDXL sidecar).
//! Phase D wires status/start/stop endpoints; Phase G adds GPU-coordination
//! mutex on top.

use std::sync::Arc;

use crate::local_runtime::runtime::{LocalRuntime, RuntimeStatus};

pub struct RuntimeRegistry {
    pub llm: Arc<LocalRuntime>,
    pub image: Arc<LocalRuntime>,
}

#[derive(Debug, serde::Serialize)]
pub struct RegistrySnapshot {
    pub llm: RuntimeStatus,
    pub image: RuntimeStatus,
}

impl RuntimeRegistry {
    pub fn new(llm: Arc<LocalRuntime>, image: Arc<LocalRuntime>) -> Self {
        Self { llm, image }
    }

    pub async fn status(&self) -> RegistrySnapshot {
        RegistrySnapshot {
            llm: self.llm.status().await,
            image: self.image.status().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local_runtime::runtime::probe_always_fail;
    use app_llm::NullSidecarLauncher;

    #[tokio::test]
    async fn snapshot_starts_off_for_both() {
        let llm = Arc::new(LocalRuntime::new(
            Arc::new(NullSidecarLauncher),
            probe_always_fail(),
        ));
        let image = Arc::new(LocalRuntime::new(
            Arc::new(NullSidecarLauncher),
            probe_always_fail(),
        ));
        let reg = RuntimeRegistry::new(llm, image);
        let snap = reg.status().await;
        assert!(matches!(snap.llm, RuntimeStatus::Off));
        assert!(matches!(snap.image, RuntimeStatus::Off));
    }
}
