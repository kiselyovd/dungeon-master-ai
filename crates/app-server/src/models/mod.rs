//! Temporary compatibility facade for model download adapters.

pub use adapter_media::models::*;

pub mod download {
    pub use adapter_media::models::download::*;
}
pub mod manager {
    pub use adapter_media::models::manager::*;
}
pub mod manifest {
    pub use adapter_media::models::manifest::*;
}
