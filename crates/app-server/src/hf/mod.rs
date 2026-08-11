//! Temporary compatibility facade for Hugging Face adapters.

pub mod client {
    pub use adapter_media::hf::client::*;
}
pub mod compat {
    pub use adapter_media::hf::compat::*;
}
pub mod manifest {
    pub use adapter_media::hf::manifest::*;
}
pub mod types {
    pub use adapter_media::hf::types::*;
}
