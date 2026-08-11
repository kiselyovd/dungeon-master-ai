//! Temporary compatibility facade for outbound video adapters.

pub use adapter_media::video::*;

pub mod provider {
    pub use app_application::ports::media::{
        VideoCapabilities, VideoError, VideoEvent, VideoPrompt, VideoProvider, VideoStream,
    };
}
pub mod sidecar {
    pub use adapter_media::video::sidecar::*;
}
