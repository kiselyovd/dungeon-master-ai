//! Static manifest of downloadable models. Sha256 values are placeholders
//! filled in at first-release time; download.rs treats empty sha256 as
//! "skip integrity check, just download" - acceptable until we cut a
//! shipping release. See spec section 3.6.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelId {
    Qwen3_5_0_8b,
    Qwen3_5_2b,
    Qwen3_5_4b,
    Qwen3_5_9b,
    SdxlTurbo,
}

#[derive(Debug, Clone)]
pub struct ModelManifest {
    pub id: ModelId,
    pub display_name: &'static str,
    pub size_bytes_estimate: u64,
    pub vram_bytes_estimate: u64,
    pub sha256: &'static str,
    pub hf_repo: &'static str,
    pub hf_filename: &'static str,
    pub kind: ModelKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelKind {
    GgufFile,
    DiffusersFolder,
}

pub const MANIFEST: &[ModelManifest] = &[
    ModelManifest {
        id: ModelId::Qwen3_5_0_8b,
        display_name: "Qwen3.5-0.8B Q4_K_M",
        size_bytes_estimate: 600 * 1024 * 1024,
        vram_bytes_estimate: 900 * 1024 * 1024,
        sha256: "",
        hf_repo: "Qwen/Qwen3.5-0.8B-Instruct-GGUF",
        hf_filename: "qwen3.5-0.8b-instruct-q4_k_m.gguf",
        kind: ModelKind::GgufFile,
    },
    ModelManifest {
        id: ModelId::Qwen3_5_2b,
        display_name: "Qwen3.5-2B Q4_K_M",
        size_bytes_estimate: 1_500 * 1024 * 1024,
        vram_bytes_estimate: 2_000 * 1024 * 1024,
        sha256: "",
        hf_repo: "Qwen/Qwen3.5-2B-Instruct-GGUF",
        hf_filename: "qwen3.5-2b-instruct-q4_k_m.gguf",
        kind: ModelKind::GgufFile,
    },
    ModelManifest {
        id: ModelId::Qwen3_5_4b,
        display_name: "Qwen3.5-4B Q4_K_M",
        size_bytes_estimate: 3_000 * 1024 * 1024,
        vram_bytes_estimate: 2_500 * 1024 * 1024,
        sha256: "",
        hf_repo: "Qwen/Qwen3.5-4B-Instruct-GGUF",
        hf_filename: "qwen3.5-4b-instruct-q4_k_m.gguf",
        kind: ModelKind::GgufFile,
    },
    ModelManifest {
        id: ModelId::Qwen3_5_9b,
        display_name: "Qwen3.5-9B Q4_K_M",
        size_bytes_estimate: 6_500 * 1024 * 1024,
        vram_bytes_estimate: 5_500 * 1024 * 1024,
        sha256: "",
        hf_repo: "Qwen/Qwen3.5-9B-Instruct-GGUF",
        hf_filename: "qwen3.5-9b-instruct-q4_k_m.gguf",
        kind: ModelKind::GgufFile,
    },
    ModelManifest {
        id: ModelId::SdxlTurbo,
        display_name: "SDXL-Turbo (fp16)",
        size_bytes_estimate: 7_000 * 1024 * 1024,
        vram_bytes_estimate: 6_500 * 1024 * 1024,
        sha256: "",
        hf_repo: "stabilityai/sdxl-turbo",
        hf_filename: "*",
        kind: ModelKind::DiffusersFolder,
    },
];

pub fn lookup(id: ModelId) -> Option<&'static ModelManifest> {
    MANIFEST.iter().find(|m| m.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_has_four_qwen_sizes_plus_sdxl() {
        let ids: Vec<_> = MANIFEST.iter().map(|m| m.id).collect();
        assert!(ids.contains(&ModelId::Qwen3_5_0_8b));
        assert!(ids.contains(&ModelId::Qwen3_5_2b));
        assert!(ids.contains(&ModelId::Qwen3_5_4b));
        assert!(ids.contains(&ModelId::Qwen3_5_9b));
        assert!(ids.contains(&ModelId::SdxlTurbo));
        assert_eq!(MANIFEST.len(), 5);
    }

    #[test]
    fn lookup_by_id_works() {
        let m = lookup(ModelId::Qwen3_5_4b).unwrap();
        assert_eq!(m.hf_filename, "qwen3.5-4b-instruct-q4_k_m.gguf");
    }
}
