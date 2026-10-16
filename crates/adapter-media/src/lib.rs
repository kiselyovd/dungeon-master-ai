//! Outbound media, model-distribution, and local-runtime adapters.

pub mod hf;
pub mod image;
pub mod local_runtime;
pub mod models;
pub mod video;

mod runtime_control_client;
pub use runtime_control_client::LoopbackRuntimeControl;
