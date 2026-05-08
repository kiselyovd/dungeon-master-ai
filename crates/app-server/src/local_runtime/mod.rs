//! Lifecycle management for sidecar processes (mistralrs-server, Python SDXL).
pub mod health;
pub mod port;

pub use health::{probe_until_ready, ProbeConfig, ProbeError};
