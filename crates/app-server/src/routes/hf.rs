//! Compatibility exports for HF inbound routes.

pub use adapter_http::routes::local_control::{
    add_manifest, delete_manifest, delete_token, get_token_status, license_check, post_token,
    search,
};
