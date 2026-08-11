//! Compatibility exports for local-LLM inbound routes.

pub use adapter_http::routes::local_control::{
    cancel_or_delete, download_events, get_manifest, set_active_model, start_download,
};
