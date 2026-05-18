//! Per-provider hardcoded "Recommended" model lists.
//!
//! The discovery sources for openai-compat and local-mistralrs currently
//! return every model the endpoint surfaces, all marked `DiscoveredApi` or
//! `DiscoveredHfHub`. The frontend ModelSelector splits its UI by source -
//! `Curated` rows go under "Recommended", `Discovered*` rows go under
//! "Discovered". With nothing curated, the Recommended section stayed empty
//! for both providers, leaving users to scroll through a long generic list.
//!
//! This module ships a small const-list of well-known starter models per
//! provider. They are prepended (deduped by `model_id`) to the discovery
//! result so the Recommended section always has a sensible default pick.
//!
//! No network calls, no per-user state. Adding a model is a Rust-side
//! const-list edit.

use app_llm::Capabilities;

use super::types::{ModelSource, ResolvedModelEntry};

const fn caps_vrt() -> Capabilities {
    // vision + reasoning + tools
    Capabilities {
        vision_input: true,
        reasoning: true,
        tool_calls: true,
        streaming: true,
    }
}

const fn caps_vt() -> Capabilities {
    // vision + tools (no reasoning)
    Capabilities {
        vision_input: true,
        reasoning: false,
        tool_calls: true,
        streaming: true,
    }
}

const fn caps_t() -> Capabilities {
    // tools only
    Capabilities {
        vision_input: false,
        reasoning: false,
        tool_calls: true,
        streaming: true,
    }
}

/// Recommended entries to surface for a given provider id. Empty Vec when
/// the provider has no curated picks (e.g. Replicate, which is a generic
/// hub search).
pub fn recommended_for(provider_id: &str) -> Vec<ResolvedModelEntry> {
    match provider_id {
        "openai" | "openai-compat" => openai_compat_recommended(),
        "local-mistralrs" => local_mistralrs_recommended(),
        _ => Vec::new(),
    }
}

fn openai_compat_recommended() -> Vec<ResolvedModelEntry> {
    vec![
        ResolvedModelEntry {
            model_id: "gpt-5".to_string(),
            display_name: "GPT-5 (recommended)".to_string(),
            capabilities: caps_vrt(),
            source: ModelSource::Curated,
            context_length: Some(400_000),
            price_per_million_input: Some(1.25),
            price_per_million_output: Some(10.0),
        },
        ResolvedModelEntry {
            model_id: "gpt-4o".to_string(),
            display_name: "GPT-4o".to_string(),
            capabilities: caps_vt(),
            source: ModelSource::Curated,
            context_length: Some(128_000),
            price_per_million_input: Some(2.5),
            price_per_million_output: Some(10.0),
        },
        ResolvedModelEntry {
            model_id: "o3-mini".to_string(),
            display_name: "o3-mini (reasoning)".to_string(),
            capabilities: caps_vrt(),
            source: ModelSource::Curated,
            context_length: Some(200_000),
            price_per_million_input: Some(1.1),
            price_per_million_output: Some(4.4),
        },
        ResolvedModelEntry {
            model_id: "meta-llama/llama-3.1-70b-instruct".to_string(),
            display_name: "Llama 3.1 70B Instruct (popular self-hosted)".to_string(),
            capabilities: caps_t(),
            source: ModelSource::Curated,
            context_length: Some(128_000),
            price_per_million_input: None,
            price_per_million_output: None,
        },
    ]
}

fn local_mistralrs_recommended() -> Vec<ResolvedModelEntry> {
    // Pulled from MANIFEST. Qwen3.5 family is uniformly VL+thinking per
    // manifest.rs:21. model_id strings use the snake_case ModelId form (the
    // enum's serde representation) so the frontend can feed them directly
    // back into chat.providers.local-mistralrs.model_id.
    vec![
        ResolvedModelEntry {
            model_id: "qwen3_5_4b".to_string(),
            display_name: "Qwen3.5-4B Q4_K_M (recommended)".to_string(),
            capabilities: caps_vrt(),
            source: ModelSource::Curated,
            context_length: Some(32_000),
            price_per_million_input: None,
            price_per_million_output: None,
        },
        ResolvedModelEntry {
            model_id: "qwen3_5_2b".to_string(),
            display_name: "Qwen3.5-2B Q4_K_M (low VRAM)".to_string(),
            capabilities: caps_vrt(),
            source: ModelSource::Curated,
            context_length: Some(32_000),
            price_per_million_input: None,
            price_per_million_output: None,
        },
        ResolvedModelEntry {
            model_id: "qwen3_5_9b".to_string(),
            display_name: "Qwen3.5-9B Q4_K_M (higher quality)".to_string(),
            capabilities: caps_vrt(),
            source: ModelSource::Curated,
            context_length: Some(32_000),
            price_per_million_input: None,
            price_per_million_output: None,
        },
    ]
}

/// Merge `recommended` in front of `discovered`, deduping by `model_id`.
/// Entries already present in `discovered` keep their discovered source
/// (so a user-facing list shows the recommended pick exactly once, in the
/// Recommended section).
pub fn merge_recommended(
    recommended: Vec<ResolvedModelEntry>,
    discovered: Vec<ResolvedModelEntry>,
) -> Vec<ResolvedModelEntry> {
    use std::collections::HashSet;
    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut out: Vec<ResolvedModelEntry> = Vec::with_capacity(recommended.len() + discovered.len());
    for entry in recommended {
        seen_ids.insert(entry.model_id.clone());
        out.push(entry);
    }
    for entry in discovered {
        if seen_ids.insert(entry.model_id.clone()) {
            out.push(entry);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_compat_has_4_curated_entries() {
        let entries = recommended_for("openai-compat");
        assert_eq!(entries.len(), 4);
        assert!(entries.iter().all(|e| matches!(e.source, ModelSource::Curated)));
        assert!(entries.iter().any(|e| e.model_id == "gpt-5"));
        assert!(entries.iter().any(|e| e.model_id == "o3-mini"));
    }

    #[test]
    fn openai_alias_resolves_to_same_list() {
        let a = recommended_for("openai");
        let b = recommended_for("openai-compat");
        assert_eq!(a.len(), b.len());
    }

    #[test]
    fn local_mistralrs_has_3_qwen_entries() {
        let entries = recommended_for("local-mistralrs");
        assert_eq!(entries.len(), 3);
        assert!(entries.iter().all(|e| matches!(e.source, ModelSource::Curated)));
        assert!(entries.iter().any(|e| e.model_id == "qwen3_5_4b"));
        assert!(entries.iter().any(|e| e.model_id == "qwen3_5_2b"));
        assert!(entries.iter().any(|e| e.model_id == "qwen3_5_9b"));
    }

    #[test]
    fn unknown_provider_has_no_recommendations() {
        assert!(recommended_for("replicate").is_empty());
        assert!(recommended_for("never-seen").is_empty());
    }

    #[test]
    fn merge_dedupes_discovered_entries_that_match_recommended_id() {
        let recommended = vec![ResolvedModelEntry {
            model_id: "gpt-5".to_string(),
            display_name: "GPT-5 (recommended)".to_string(),
            capabilities: caps_vrt(),
            source: ModelSource::Curated,
            context_length: Some(400_000),
            price_per_million_input: None,
            price_per_million_output: None,
        }];
        let discovered = vec![
            ResolvedModelEntry {
                model_id: "gpt-5".to_string(),
                display_name: "gpt-5 (openai)".to_string(),
                capabilities: caps_vrt(),
                source: ModelSource::DiscoveredApi,
                context_length: None,
                price_per_million_input: None,
                price_per_million_output: None,
            },
            ResolvedModelEntry {
                model_id: "gpt-4o".to_string(),
                display_name: "gpt-4o (openai)".to_string(),
                capabilities: caps_vt(),
                source: ModelSource::DiscoveredApi,
                context_length: None,
                price_per_million_input: None,
                price_per_million_output: None,
            },
        ];
        let merged = merge_recommended(recommended, discovered);
        assert_eq!(merged.len(), 2);
        // Recommended gpt-5 wins the slot.
        let gpt5 = merged.iter().find(|e| e.model_id == "gpt-5").unwrap();
        assert!(matches!(gpt5.source, ModelSource::Curated));
        // gpt-4o is in the discovered set, kept as DiscoveredApi.
        let gpt4o = merged.iter().find(|e| e.model_id == "gpt-4o").unwrap();
        assert!(matches!(gpt4o.source, ModelSource::DiscoveredApi));
    }

    #[test]
    fn merge_preserves_recommended_order_first_then_discovered_order() {
        let recommended = vec![ResolvedModelEntry {
            model_id: "rec-1".to_string(),
            display_name: "Rec1".into(),
            capabilities: caps_t(),
            source: ModelSource::Curated,
            context_length: None,
            price_per_million_input: None,
            price_per_million_output: None,
        }];
        let discovered = vec![
            ResolvedModelEntry {
                model_id: "disc-a".to_string(),
                display_name: "DiscA".into(),
                capabilities: caps_t(),
                source: ModelSource::DiscoveredApi,
                context_length: None,
                price_per_million_input: None,
                price_per_million_output: None,
            },
            ResolvedModelEntry {
                model_id: "disc-b".to_string(),
                display_name: "DiscB".into(),
                capabilities: caps_t(),
                source: ModelSource::DiscoveredApi,
                context_length: None,
                price_per_million_input: None,
                price_per_million_output: None,
            },
        ];
        let merged = merge_recommended(recommended, discovered);
        assert_eq!(
            merged.iter().map(|e| e.model_id.as_str()).collect::<Vec<_>>(),
            vec!["rec-1", "disc-a", "disc-b"],
        );
    }
}
