//! Image generation module.
//!
//! - `provider` - trait + types shared by all providers.
//! - `cache` - deterministic cache key for prompt deduplication.
//! - `replicate` - Replicate v1 predictions API provider (cloud).
//! - `stub` - LocalSdxlSidecarProvider, an M3-time stub for the M4 Python sidecar.

pub mod cache;
pub mod provider;
pub mod replicate;
pub mod stub;
