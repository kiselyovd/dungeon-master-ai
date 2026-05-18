//! Image generation module.
//!
//! - `provider` - trait + types shared by all providers.
//! - `cache` - deterministic cache key for prompt deduplication.
//! - `replicate` - Replicate v1 predictions API provider (cloud).
//! - `stub` - LocalImageSidecarProvider, the local HTTP loopback to the Python sidecar.

pub mod cache;
pub mod provider;
pub mod replicate;
pub mod retry;
pub mod stub;

pub use retry::{default_policy as image_default_retry_policy, RetryableImageProvider};
