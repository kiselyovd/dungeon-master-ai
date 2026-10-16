//! Outbound video generation adapters.

pub mod provider;
pub mod sidecar;

pub use provider::{
    VideoCapabilities, VideoError, VideoEvent, VideoPrompt, VideoProvider, VideoStream,
};
pub use sidecar::LocalVideoSidecarProvider;
