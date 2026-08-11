//! Outbound two-runtime registry for model and media sidecars.
//! Phase D wires status/start/stop endpoints; Phase G layers a GPU-coordination
//! mutex (`acquire_gpu_for_image` / `release_gpu_to_llm`) on top so the
//! Auto-Swap VRAM strategy can hand the 10 GB on a single card between LLM
//! and image gen workloads without out-of-memory crashes.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use tokio::sync::{Mutex, OwnedMutexGuard};
use tracing::warn;

use crate::local_runtime::runtime::{LocalRuntime, RuntimeStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuOwner {
    None = 0,
    Llm = 1,
    Image = 2,
    /// M7-DM: video generation (LTX-Video local sidecar) holding the GPU.
    /// Same single-owner contract as Image: caller must `release_gpu_to_llm`
    /// after the clip completes.
    Video = 3,
}

impl From<u8> for GpuOwner {
    fn from(value: u8) -> Self {
        match value {
            1 => GpuOwner::Llm,
            2 => GpuOwner::Image,
            3 => GpuOwner::Video,
            _ => GpuOwner::None,
        }
    }
}

pub struct RuntimeRegistry {
    pub llm: Arc<LocalRuntime>,
    pub image: Arc<LocalRuntime>,
    gpu_owner: AtomicU8,
    gpu_gate: Arc<Mutex<()>>,
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
            gpu_gate: Arc::new(Mutex::new(())),
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
    pub async fn acquire_gpu_for_image(&self) -> Result<OwnedMutexGuard<()>, String> {
        let lease = self.gpu_gate.clone().lock_owned().await;
        if self.gpu_owner() == GpuOwner::Llm {
            self.llm.stop().await.map_err(|e| e.to_string())?;
        }
        self.gpu_owner
            .store(GpuOwner::Image as u8, Ordering::SeqCst);
        Ok(lease)
    }

    /// Same contract as `acquire_gpu_for_image` but marks Video ownership;
    /// LTX-Video generation (~20-30s) goes through this path. Image and Video
    /// share the same single-owner mutex so they serialise naturally.
    pub async fn acquire_gpu_for_video(&self) -> Result<OwnedMutexGuard<()>, String> {
        let lease = self.gpu_gate.clone().lock_owned().await;
        if self.gpu_owner() == GpuOwner::Llm {
            self.llm.stop().await.map_err(|e| e.to_string())?;
        }
        self.gpu_owner
            .store(GpuOwner::Video as u8, Ordering::SeqCst);
        Ok(lease)
    }

    /// Restart the LLM sidecar with `args` on `port` and mark it as the GPU
    /// owner. The caller reuses the pre-swap port so the chat provider's
    /// base_url stays valid across the swap. Blocks until the LLM probes
    /// healthy (so the agent loop's next round can call it immediately).
    pub async fn release_gpu_to_llm(&self, args: &[&str], port: u16) -> Result<(), String> {
        self.llm
            .start_with_retry("mistralrs-server", args, port, 3)
            .await
            .map_err(|e| e.to_string())?;
        self.gpu_owner.store(GpuOwner::Llm as u8, Ordering::SeqCst);
        Ok(())
    }
}

/// Per-turn Auto-Swap coordinator. On a single 10 GB card the local LLM (~3 GB)
/// and an SDXL pipeline (~6.5 GB + CUDA overhead) do not both fit while a
/// generation runs, so each `generate_image` tool call frees the LLM's VRAM
/// first and restarts it on the SAME port afterwards (keeping the chat
/// provider's base_url valid). Built only for the local-runtime + Auto-Swap
/// case; `None` everywhere else means image gen runs without touching the LLM.
pub struct ImageGpuSwap {
    registry: Arc<RuntimeRegistry>,
    llm_args: Vec<String>,
    llm_port: u16,
    lease: Mutex<Option<OwnedMutexGuard<()>>>,
}

impl ImageGpuSwap {
    pub fn new(registry: Arc<RuntimeRegistry>, llm_args: Vec<String>, llm_port: u16) -> Self {
        Self {
            registry,
            llm_args,
            llm_port,
            lease: Mutex::new(None),
        }
    }

    /// Stop the LLM so its VRAM is free before the image pipeline loads.
    pub async fn acquire(&self) {
        let mut slot = self.lease.lock().await;
        if slot.is_some() {
            return;
        }
        match self.registry.acquire_gpu_for_image().await {
            Ok(lease) => *slot = Some(lease),
            Err(error) => {
                warn!(code = "gpu_acquire_failed", %error, "auto-swap acquire failed");
            }
        }
    }

    /// Restart the LLM on its original port after the image completes. Blocks
    /// until it is healthy again so the next agent round can use it.
    pub async fn release(&self) {
        let refs: Vec<&str> = self.llm_args.iter().map(String::as_str).collect();
        if let Err(e) = self.registry.release_gpu_to_llm(&refs, self.llm_port).await {
            warn!(code = "gpu_release_failed", error = %e, "auto-swap release failed");
        }
        self.lease.lock().await.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local_runtime::runtime::{probe_always_fail, probe_always_ok};
    use app_application::ports::runtime::SpawnSpec;

    use crate::local_runtime::test_support::{MockSidecarLauncher, NullSidecarLauncher};

    fn null_runtime() -> Arc<LocalRuntime> {
        Arc::new(LocalRuntime::new(
            Arc::new(NullSidecarLauncher),
            probe_always_fail(),
            "/health",
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
        let llm = Arc::new(LocalRuntime::new(
            Arc::new(launcher),
            probe_always_ok(),
            "/health",
        ));
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

    #[tokio::test]
    async fn image_and_video_gpu_leases_are_serialized() {
        let registry = Arc::new(RuntimeRegistry::new(null_runtime(), null_runtime()));
        let image_lease = registry.acquire_gpu_for_image().await.unwrap();

        let contender = registry.clone();
        let video = tokio::spawn(async move {
            let lease = contender.acquire_gpu_for_video().await.unwrap();
            (contender.gpu_owner(), lease)
        });

        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        assert!(!video.is_finished(), "video must wait for the image lease");
        drop(image_lease);

        let (owner, _video_lease) = video.await.unwrap();
        assert_eq!(owner, GpuOwner::Video);
    }
}
