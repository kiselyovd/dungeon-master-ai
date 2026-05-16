//! Static catalog of providers + curated models per modality. Lookups feed
//! the `GET /providers/catalog` route, the `GET /providers/:id/caps` route, and
//! the frontend ModelSelector "Curated" section. Discovered/Custom entries
//! live in `discoveredCatalogs` on the settings slice (filled at runtime via
//! `POST /providers/discover`), not here.

use app_llm::Capabilities;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderMode {
    Local,
    Cloud,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderModality {
    Chat,
    Image,
    Video,
}

#[derive(Debug, Clone, Serialize)]
pub struct CuratedModelEntry {
    pub model_id: &'static str,
    pub display_name: &'static str,
    pub capabilities: Capabilities,
    pub default: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderCatalogEntry {
    pub id: &'static str,
    pub display_name: &'static str,
    pub mode: ProviderMode,
    pub modality: ProviderModality,
    pub curated_models: &'static [CuratedModelEntry],
    pub requires_api_key: bool,
    pub requires_base_url: bool,
    pub supports_discovery: bool,
    pub license: &'static str,
}

const fn caps_all_true() -> Capabilities {
    Capabilities {
        vision_input: true,
        reasoning: true,
        tool_calls: true,
        streaming: true,
    }
}

const fn caps_text_with_tools() -> Capabilities {
    Capabilities {
        vision_input: false,
        reasoning: false,
        tool_calls: true,
        streaming: true,
    }
}

pub const CHAT_CATALOG: &[ProviderCatalogEntry] = &[
    ProviderCatalogEntry {
        id: "local-mistralrs",
        display_name: "Local: mistralrs",
        mode: ProviderMode::Local,
        modality: ProviderModality::Chat,
        curated_models: &[
            CuratedModelEntry {
                model_id: "qwen3.5-0.8b",
                display_name: "Qwen3.5-0.8B (Q4_K_M, VL+thinking)",
                capabilities: caps_all_true(),
                default: false,
            },
            CuratedModelEntry {
                model_id: "qwen3.5-2b",
                display_name: "Qwen3.5-2B (Q4_K_M, VL+thinking)",
                capabilities: caps_all_true(),
                default: false,
            },
            CuratedModelEntry {
                model_id: "qwen3.5-4b",
                display_name: "Qwen3.5-4B (Q4_K_M, VL+thinking)",
                capabilities: caps_all_true(),
                default: true,
            },
            CuratedModelEntry {
                model_id: "qwen3.5-9b",
                display_name: "Qwen3.5-9B (Q4_K_M, VL+thinking)",
                capabilities: caps_all_true(),
                default: false,
            },
        ],
        requires_api_key: false,
        requires_base_url: false,
        supports_discovery: true,
        license: "Apache 2.0 (Qwen)",
    },
    ProviderCatalogEntry {
        id: "anthropic",
        display_name: "Anthropic Claude",
        mode: ProviderMode::Cloud,
        modality: ProviderModality::Chat,
        curated_models: &[
            CuratedModelEntry {
                model_id: "claude-opus-4-7",
                display_name: "Claude Opus 4.7",
                capabilities: caps_all_true(),
                default: false,
            },
            CuratedModelEntry {
                model_id: "claude-sonnet-4-6",
                display_name: "Claude Sonnet 4.6",
                capabilities: caps_all_true(),
                default: false,
            },
            CuratedModelEntry {
                model_id: "claude-haiku-4-5-20251001",
                display_name: "Claude Haiku 4.5",
                capabilities: caps_all_true(),
                default: true,
            },
        ],
        requires_api_key: true,
        requires_base_url: false,
        supports_discovery: false,
        license: "Anthropic ToS",
    },
    ProviderCatalogEntry {
        id: "openai-compat",
        display_name: "OpenAI-compatible (custom)",
        mode: ProviderMode::Cloud,
        modality: ProviderModality::Chat,
        curated_models: &[CuratedModelEntry {
            model_id: "custom",
            display_name: "Custom (configure model id below)",
            capabilities: caps_text_with_tools(),
            default: true,
        }],
        requires_api_key: true,
        requires_base_url: true,
        supports_discovery: true,
        license: "varies",
    },
];

// IMAGE_CATALOG + VIDEO_CATALOG filled in Task B.5.
pub const IMAGE_CATALOG: &[ProviderCatalogEntry] = &[];
pub const VIDEO_CATALOG: &[ProviderCatalogEntry] = &[];

pub fn find_chat_entry(id: &str) -> Option<&'static ProviderCatalogEntry> {
    CHAT_CATALOG.iter().find(|e| e.id == id)
}

pub fn find_entry_any_modality(id: &str) -> Option<&'static ProviderCatalogEntry> {
    CHAT_CATALOG
        .iter()
        .chain(IMAGE_CATALOG.iter())
        .chain(VIDEO_CATALOG.iter())
        .find(|e| e.id == id)
}

pub fn default_chat_model(id: &str) -> Option<&'static str> {
    let entry = find_chat_entry(id)?;
    entry
        .curated_models
        .iter()
        .find(|m| m.default)
        .map(|m| m.model_id)
        .or_else(|| entry.curated_models.first().map(|m| m.model_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_catalog_has_three_providers_in_m7_dm() {
        let ids: Vec<_> = CHAT_CATALOG.iter().map(|e| e.id).collect();
        assert!(ids.contains(&"local-mistralrs"));
        assert!(ids.contains(&"anthropic"));
        assert!(ids.contains(&"openai-compat"));
        assert_eq!(CHAT_CATALOG.len(), 3);
    }

    #[test]
    fn anthropic_default_model_is_haiku_4_5() {
        assert_eq!(
            default_chat_model("anthropic"),
            Some("claude-haiku-4-5-20251001")
        );
    }

    #[test]
    fn local_mistralrs_default_is_qwen3_5_4b() {
        assert_eq!(default_chat_model("local-mistralrs"), Some("qwen3.5-4b"));
    }

    #[test]
    fn find_chat_entry_unknown_returns_none() {
        assert!(find_chat_entry("not-a-provider").is_none());
    }
}
