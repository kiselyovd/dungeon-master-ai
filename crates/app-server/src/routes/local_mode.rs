//! Compatibility exports for local-runtime inbound routes.

pub use adapter_http::routes::local_control::{
    delete_local_download as delete_download, download_progress, get_config, post_config,
    post_local_download as post_download, runtime_start, runtime_status, runtime_stop,
};
