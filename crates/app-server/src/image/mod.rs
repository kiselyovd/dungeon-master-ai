//! Temporary compatibility facade for outbound image adapters.

pub use adapter_media::image::*;

pub mod cache {
    pub use adapter_media::image::cache::*;
}
pub mod provider {
    pub use app_application::ports::media::{ImageBytes, ImageError, ImagePrompt, ImageProvider};
}
pub mod replicate {
    pub use adapter_media::image::replicate::*;
}
pub mod retry {
    pub use adapter_media::image::retry::*;
}
pub mod stub {
    pub use adapter_media::image::stub::*;
}
