//! Per-modality provider registry. Replaces the M5/M6 single-`Arc<dyn LlmProvider>`
//! AppState slot with a struct that carries chat / image / video providers,
//! each independently swappable when settings change.
//!
//! Video slot is added in Task F.3.

pub mod catalog;
pub mod discovery;
pub mod registry;

pub use catalog::{
    default_chat_model, find_chat_entry, find_entry_any_modality, CuratedModelEntry,
    ProviderCatalogEntry, ProviderModality, ProviderMode, CHAT_CATALOG, IMAGE_CATALOG,
    VIDEO_CATALOG,
};
pub use registry::ProviderRegistry;
