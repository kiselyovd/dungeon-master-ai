//! Shared manifest schema between the system-shipped catalog and the
//! user-added manifest (HF search additions). Both `SystemEntry` and
//! `UserEntry` are serialized as snake_case JSON for direct consumption by
//! the React frontend in `src/state/local_llm/manifest.ts`.

use serde::{Deserialize, Serialize};

/// One curated local-LLM model that ships with the app.
///
/// `size_gb` is an estimate displayed in the UI; the authoritative size lives
/// in the download manager and may differ slightly once the actual file is
/// fetched. `license` is a SPDX-ish short string (e.g. `apache-2.0`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemEntry {
    pub id: String,
    pub hf_repo: String,
    pub hf_filename: String,
    pub arch: String,
    pub quant: String,
    pub size_gb: f32,
    pub license: String,
    pub display_name: String,
}

/// A user-added entry layered on top of the system catalog. Embeds the same
/// fields as `SystemEntry` (flattened in JSON) plus provenance metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserEntry {
    #[serde(flatten)]
    pub system: SystemEntry,
    pub added_at: String,
    pub source: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_entry_round_trips_snake_case() {
        let s = SystemEntry {
            id: "qwen3.5-4b".into(),
            hf_repo: "Qwen/Qwen3.5-4B-Instruct-GGUF".into(),
            hf_filename: "qwen3.5-4b-instruct-q4_k_m.gguf".into(),
            arch: "qwen3".into(),
            quant: "gguf-q4_k_m".into(),
            size_gb: 3.0,
            license: "apache-2.0".into(),
            display_name: "Qwen3.5-4B".into(),
        };
        let json = serde_json::to_string(&s).expect("serialize");
        assert!(json.contains("\"hf_repo\":"));
        assert!(json.contains("\"display_name\":"));
        let round: SystemEntry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(round, s);
    }

    #[test]
    fn user_entry_flattens_system_fields() {
        let u = UserEntry {
            system: SystemEntry {
                id: "custom/foo".into(),
                hf_repo: "custom/foo".into(),
                hf_filename: "foo.gguf".into(),
                arch: "llama".into(),
                quant: "gguf-q4_k_m".into(),
                size_gb: 1.0,
                license: "mit".into(),
                display_name: "Foo".into(),
            },
            added_at: "2026-05-19T00:00:00Z".into(),
            source: "hf-search".into(),
        };
        let json = serde_json::to_string(&u).expect("serialize");
        // Flattened: hf_repo lives at the top level alongside source.
        assert!(json.contains("\"hf_repo\":"));
        assert!(json.contains("\"source\":\"hf-search\""));
        assert!(!json.contains("\"system\":"));
    }
}
