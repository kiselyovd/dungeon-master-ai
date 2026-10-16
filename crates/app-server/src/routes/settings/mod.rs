//! Compatibility exports for settings inbound routes.

pub mod v2;

pub use crate::control_services::settings::{post_settings_v2, validate_settings_v2};
pub use adapter_http::routes::settings::{
    get_providers, post_settings_v2 as post_settings_v2_http,
};
pub use v2::*;
