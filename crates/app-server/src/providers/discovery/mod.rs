//! M7-DM Dynamic Model Discovery. POST /providers/discover dispatches to
//! per-provider DiscoverySource implementations and returns a normalised
//! DiscoveryResult. Frontend caches per (provider_id, key_hash) for 7 days.

pub mod anthropic_curated;
pub mod capability_infer;
pub mod hf_hub_search;
pub mod openai_v1_models;
pub mod recommended;
pub mod replicate_search;
pub mod types;

pub use anthropic_curated::AnthropicCurated;
pub use capability_infer::infer_capabilities;
pub use hf_hub_search::HfHubSearch;
pub use openai_v1_models::OpenAIV1Models;
pub use recommended::{merge_recommended, recommended_for};
pub use replicate_search::ReplicateSearch;
pub use types::{
    DiscoverParams, DiscoveryError, DiscoveryResult, DiscoverySource, ModelSource,
    ResolvedModelEntry,
};
