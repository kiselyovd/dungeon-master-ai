//! Lifecycle management for sidecar processes (mistralrs-server, Python SDXL).
pub mod health;
pub mod port;
pub mod runtime;

pub use health::{probe_until_ready, ProbeConfig, ProbeError};
pub use runtime::{probe_always_fail, probe_always_ok, probe_real, LocalRuntime, ProbeFn, RuntimeStatus};
