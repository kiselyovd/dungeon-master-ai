//! Local SDXL sidecar provider stub.
//!
//! M4 implements this against the Python FastAPI sidecar.
//! The stub exists so the M3 type system compiles with the full ImageProvider trait
//! and the orchestrator can hand out one of two ImageProvider impls without a runtime
//! variant check.

use async_trait::async_trait;

use crate::image::provider::{ImageBytes, ImageError, ImagePrompt, ImageProvider};

pub struct LocalSdxlSidecarProvider;

#[async_trait]
impl ImageProvider for LocalSdxlSidecarProvider {
    async fn generate(&self, _prompt: ImagePrompt) -> Result<ImageBytes, ImageError> {
        unimplemented!("LocalSdxlSidecarProvider is not implemented until M4")
    }

    fn estimated_seconds(&self) -> u32 {
        8
    }

    fn cost_per_image(&self) -> f32 {
        0.0
    }
}
