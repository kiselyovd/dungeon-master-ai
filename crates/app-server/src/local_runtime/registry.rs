//! Two-runtime registry: LLM (mistralrs sidecar) + image (Python SDXL sidecar).
//! Phase D wires status/start/stop endpoints; Phase G layers a GPU-coordination
//! mutex (`acquire_gpu_for_image` / `release_gpu_to_llm`) on top so the
//! Auto-Swap VRAM strategy can hand the 10 GB on a single card between LLM
//! and image gen workloads without out-of-memory crashes.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use crate::local_runtime::runtime::{LocalRuntime, RuntimeStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuOwner {
    None = 0,
    Llm = 1,
    Image = 2,
}

impl From<u8> for GpuOwner {
    fn from(value: u8) -> Self {
        match value {
            1 => GpuOwner::Llm,
            2 => GpuOwner::Image,
            _ => GpuOwner::None,
        }
    }
}

pub struct RuntimeRegistry {
    pub llm: Arc<LocalRuntime>,
    pub image: Arc<LocalRuntime>,
    gpu_owner: AtomicU8,
}

#[derive(Debug, serde::Serialize)]
pub struct RegistrySnapshot {
    pub llm: RuntimeStatus,
    pub image: RuntimeStatus,
}

impl RuntimeRegistry {
    pub fn new(llm: Arc<LocalRuntime>, image: Arc<LocalRuntime>) -> Self {
        Self {
            llm,
            image,
            gpu_owner: AtomicU8::new(GpuOwner::None as u8),
        }
    }

    pub async fn status(&self) -> RegistrySnapshot {
        RegistrySnapshot {
            llm: self.llm.status().await,
            image: self.image.status().await,
        }
    }

    pub fn gpu_owner(&self) -> GpuOwner {
        GpuOwner::from(self.gpu_owner.load(Ordering::SeqCst))
    }

    pub fn mark_llm_owns_gpu(&self) {
        self.gpu_owner.store(GpuOwner::Llm as u8, Ordering::SeqCst);
    }

    /// Hand the GPU to the image runtime. If the LLM owns it today, stop the
    /// LLM sidecar so its VRAM frees before the image pipeline loads its
    /// weights. Caller is responsible for handing the GPU back via
    /// `release_gpu_to_llm` once image generation completes.
    ///
    /// `mistralrs-server` does not expose a clean `/v1/admin/unload` endpoint
    /// today (open question #2 in the M4 spec); when it does, this method can
    /// switch to that for a faster swap.
    pub async fn acquire_gpu_for_image(&self) -> Result<(), String> {
        if self.gpu_owner() == GpuOwner::Llm {
            self.llm.stop().await.map_err(|e| e.to_string())?;
        }
        self.gpu_owner
            .store(GpuOwner::Image as u8, Ordering::SeqCst);
        Ok(())
    }

    /// Restart the LLM sidecar with the supplied gguf path + port and mark
    /// it as the GPU owner.
    pub async fn release_gpu_to_llm(&self, model_path: &str, port: u16) -> Result<(), String> {
        let port_str = port.to_string();
        let args: &[&str] = &["--port", &port_str, "--gguf-file", model_path];
        self.llm
            .start_with_retry("mistralrs-server", args, port, 3)
            .await
            .map_err(|e| e.to_string())?;
        self.gpu_owner.store(GpuOwner::Llm as u8, Ordering::SeqCst);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local_runtime::runtime::{probe_always_fail, probe_always_ok};
    use app_llm::sidecar_launcher::{MockSidecarLauncher, SpawnSpec};
    use app_llm::NullSidecarLauncher;

    fn null_runtime() -> Arc<LocalRuntime> {
        Arc::new(LocalRuntime::new(
            Arc::new(NullSidecarLauncher),
            probe_always_fail(),
        ))
    }

    #[tokio::test]
    async fn snapshot_starts_off_for_both() {
        let reg = RuntimeRegistry::new(null_runtime(), null_runtime());
        let snap = reg.status().await;
        assert!(matches!(snap.llm, RuntimeStatus::Off));
        assert!(matches!(snap.image, RuntimeStatus::Off));
    }

    #[tokio::test]
    async fn gpu_owner_starts_none() {
        let reg = RuntimeRegistry::new(null_runtime(), null_runtime());
        assert_eq!(reg.gpu_owner(), GpuOwner::None);
    }

    #[tokio::test]
    async fn acquire_for_image_marks_image_owner() {
        let reg = RuntimeRegistry::new(null_runtime(), null_runtime());
        reg.acquire_gpu_for_image().await.unwrap();
        assert_eq!(reg.gpu_owner(), GpuOwner::Image);
    }

    #[tokio::test]
    async fn acquire_for_image_stops_llm_when_it_owns_gpu() {
        let mut launcher = MockSidecarLauncher::default();
        launcher.expect_spawn(SpawnSpec {
            command: "mistralrs-server".into(),
            args: vec![],
            stdout_lines: vec![],
        });
        let llm = Arc::new(LocalRuntime::new(Arc::new(launcher), probe_always_ok()));
        let _ = llm
            .start("mistralrs-server", &[], 37500)
            .await
            .expect("start");
        let reg = RuntimeRegistry::new(llm.clone(), null_runtime());
        reg.mark_llm_owns_gpu();
        reg.acquire_gpu_for_image().await.unwrap();
        assert_eq!(reg.gpu_owner(), GpuOwner::Image);
        assert!(matches!(llm.status().await, RuntimeStatus::Off));
    }
}
