//! Temporary compatibility facade for outbound local-runtime adapters.

pub use adapter_media::local_runtime::*;

pub mod health {
    pub use adapter_media::local_runtime::health::*;
}
pub mod port {
    pub use adapter_media::local_runtime::port::*;
}
pub mod registry {
    pub use adapter_media::local_runtime::registry::*;
}
pub mod runtime {
    pub use adapter_media::local_runtime::runtime::*;
}
