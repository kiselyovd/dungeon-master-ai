//! Outbound image generation adapters.
//!
//! - `provider` - trait + types shared by all providers.
//! - `cache` - deterministic cache key for prompt deduplication.
//! - `replicate` - Replicate v1 predictions API provider (cloud).
//! - `stub` - LocalImageSidecarProvider, the local HTTP loopback to the Python sidecar.

pub mod bundled;
pub mod cache;
pub mod provider;
pub mod replicate;
pub mod resilient;
pub mod retry;
pub mod stub;

pub use bundled::{
    bundled_assets, bundled_image_provider, BundledAsset, BundledAssetCategory,
    BundledImageProvider,
};
pub use provider::{ImageBytes, ImageError, ImagePrompt, ImageProvider, ImageSource};
pub use resilient::ResilientImageProvider;
pub use retry::{default_policy as image_default_retry_policy, RetryableImageProvider};
