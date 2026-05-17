//! M7-DM video generation module: trait + LTX-Video sidecar provider + SSE route.

pub mod provider;
pub mod sidecar;

pub use provider::{
    VideoCapabilities, VideoError, VideoEvent, VideoPrompt, VideoProvider, VideoStream,
};
pub use sidecar::LocalVideoSidecarProvider;
