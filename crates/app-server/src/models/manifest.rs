//! Static manifest of downloadable models. Sha256 values are placeholders
//! filled in at first-release time; download.rs treats empty sha256 as
//! "skip integrity check, just download" - acceptable until we cut a
//! shipping release. See spec section 3.6.

use serde::{Deserialize, Serialize};

/// Identifier for a downloadable model.
///
/// The `Custom` variant holds owned strings so users can register arbitrary
/// HF GGUF repos discovered via Discovery. This makes the enum `Clone + Eq +
/// Hash` (no longer `Copy`); all callers must pass `id.clone()` when they
/// previously relied on `Copy`.
///
/// `MANIFEST` only carries the static (non-`Custom`) variants; the download
/// manager synthesises a transient `ModelManifest` for `Custom` at call time
/// in `manifest_for(&id)`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelId {
    // chat (existing; Qwen3.5 family is uniformly VL+thinking)
    Qwen3_5_0_8b,
    Qwen3_5_2b,
    Qwen3_5_4b,
    Qwen3_5_9b,

    // image (existing)
    SdxlTurbo,

    // image (NEW for M7-DM)
    SdxlLightningBase,
    SdxlLightning4StepLora,
    NunchakuFluxDevInt4,
    NunchakuFluxTurboAlpha8stepLora,
    ZImageTurboSvdq,
    Qwen3_4bTextEncoder,
    T5xxlEncoder,

    // video (NEW for M7-DM, opt-in)
    LtxVideo09_6Distilled,

    // custom HF GGUF discovered at runtime (NEW)
    Custom {
        hf_repo: String,
        gguf_filename: String,
        mmproj_filename: Option<String>,
    },
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
    /// Dependency graph for multi-file models. Walker dedupes shared deps
    /// (T5xxlEncoder is required by both FLUX-Quality and LTX-Video).
    pub requires: &'static [ModelId],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelKind {
    GgufFile,
    DiffusersFolder,
    /// VL GGUF that ships a separate mmproj GGUF.
    GgufWithMmproj { mmproj_filename: &'static str },
    /// Standalone .safetensors (e.g. LoRA, Turbo-Alpha).
    SafetensorsSingleFile,
    /// SVDQuant W4A4 INT4 single-file (e.g. Nunchaku FLUX).
    NunchakuSvdquant,
    /// SVDQ INT4 model directory (e.g. Z-Image-Turbo).
    SvdqInt4Folder,
    /// LTX-Video distilled checkpoint.
    LtxVideoSafetensors,
}

pub const MANIFEST: &[ModelManifest] = &[
    // --- chat (Qwen3.5 family, all VL+thinking) ---
    ModelManifest {
        id: ModelId::Qwen3_5_0_8b,
        display_name: "Qwen3.5-0.8B Q4_K_M",
        size_bytes_estimate: 600 * 1024 * 1024,
        vram_bytes_estimate: 900 * 1024 * 1024,
        sha256: "",
        hf_repo: "Qwen/Qwen3.5-0.8B-Instruct-GGUF",
        hf_filename: "qwen3.5-0.8b-instruct-q4_k_m.gguf",
        kind: ModelKind::GgufFile,
        requires: &[],
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
        requires: &[],
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
        requires: &[],
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
        requires: &[],
    },
    // --- image: Fast preset (existing) ---
    ModelManifest {
        id: ModelId::SdxlTurbo,
        display_name: "SDXL-Turbo (fp16)",
        size_bytes_estimate: 7_000 * 1024 * 1024,
        vram_bytes_estimate: 6_500 * 1024 * 1024,
        sha256: "",
        hf_repo: "stabilityai/sdxl-turbo",
        hf_filename: "*",
        kind: ModelKind::DiffusersFolder,
        requires: &[],
    },
    // --- image: Balanced preset (NEW default) ---
    ModelManifest {
        id: ModelId::SdxlLightningBase,
        display_name: "SDXL Base 1.0 (fp16)",
        size_bytes_estimate: 6_500 * 1024 * 1024,
        vram_bytes_estimate: 5_000 * 1024 * 1024,
        sha256: "",
        hf_repo: "stabilityai/stable-diffusion-xl-base-1.0",
        hf_filename: "*",
        kind: ModelKind::DiffusersFolder,
        requires: &[],
    },
    ModelManifest {
        id: ModelId::SdxlLightning4StepLora,
        display_name: "SDXL-Lightning 4-step LoRA",
        size_bytes_estimate: 200 * 1024 * 1024,
        vram_bytes_estimate: 250 * 1024 * 1024,
        sha256: "",
        hf_repo: "ByteDance/SDXL-Lightning",
        hf_filename: "sdxl_lightning_4step_lora.safetensors",
        kind: ModelKind::SafetensorsSingleFile,
        requires: &[ModelId::SdxlLightningBase],
    },
    // --- image: Quality preset (FLUX-dev NC) ---
    ModelManifest {
        id: ModelId::NunchakuFluxDevInt4,
        display_name: "Nunchaku FLUX.1-dev INT4 (SVDQuant W4A4)",
        size_bytes_estimate: 6_500 * 1024 * 1024,
        vram_bytes_estimate: 6_500 * 1024 * 1024,
        sha256: "",
        hf_repo: "mit-han-lab/nunchaku-flux.1-dev",
        hf_filename: "svdq-int4_r32-flux.1-dev.safetensors",
        kind: ModelKind::NunchakuSvdquant,
        requires: &[ModelId::T5xxlEncoder],
    },
    ModelManifest {
        id: ModelId::NunchakuFluxTurboAlpha8stepLora,
        display_name: "Nunchaku FLUX Turbo-Alpha 8-step LoRA",
        size_bytes_estimate: 150 * 1024 * 1024,
        vram_bytes_estimate: 200 * 1024 * 1024,
        sha256: "",
        hf_repo: "alimama-creative/FLUX.1-Turbo-Alpha",
        hf_filename: "diffusion_pytorch_model.safetensors",
        kind: ModelKind::SafetensorsSingleFile,
        requires: &[ModelId::NunchakuFluxDevInt4],
    },
    // --- image: Quality-OSS preset ---
    ModelManifest {
        id: ModelId::ZImageTurboSvdq,
        display_name: "Z-Image-Turbo 6B SVDQ-INT4",
        size_bytes_estimate: 5_500 * 1024 * 1024,
        vram_bytes_estimate: 5_500 * 1024 * 1024,
        sha256: "",
        hf_repo: "mit-han-lab/z-image-turbo-svdq-int4",
        hf_filename: "*",
        kind: ModelKind::SvdqInt4Folder,
        requires: &[ModelId::Qwen3_4bTextEncoder],
    },
    // --- shared text encoders ---
    ModelManifest {
        id: ModelId::Qwen3_4bTextEncoder,
        display_name: "Qwen3-4B (text encoder for Z-Image)",
        size_bytes_estimate: 3_000 * 1024 * 1024,
        vram_bytes_estimate: 3_000 * 1024 * 1024,
        sha256: "",
        hf_repo: "Qwen/Qwen3-4B",
        hf_filename: "*",
        kind: ModelKind::DiffusersFolder,
        requires: &[],
    },
    ModelManifest {
        id: ModelId::T5xxlEncoder,
        display_name: "T5-XXL Encoder (FLUX + LTX shared)",
        size_bytes_estimate: 9_500 * 1024 * 1024,
        vram_bytes_estimate: 9_500 * 1024 * 1024,
        sha256: "",
        hf_repo: "google/t5-v1_1-xxl",
        hf_filename: "*",
        kind: ModelKind::DiffusersFolder,
        requires: &[],
    },
    // --- video (opt-in) ---
    ModelManifest {
        id: ModelId::LtxVideo09_6Distilled,
        display_name: "LTX-Video 0.9.6 distilled",
        size_bytes_estimate: 6_000 * 1024 * 1024,
        vram_bytes_estimate: 8_000 * 1024 * 1024,
        sha256: "",
        hf_repo: "Lightricks/LTX-Video",
        hf_filename: "ltx-video-2b-v0.9.6-distilled.safetensors",
        kind: ModelKind::LtxVideoSafetensors,
        requires: &[ModelId::T5xxlEncoder],
    },
];

pub fn lookup(id: &ModelId) -> Option<&'static ModelManifest> {
    MANIFEST.iter().find(|m| &m.id == id)
}

/// Walk the dependency graph for a target model and return a topologically
/// ordered list (deps first, target last) with duplicates removed.
///
/// `Custom` variant returns a single-element vec; it has no static deps.
pub fn resolve_download_order(target: &ModelId) -> Vec<ModelId> {
    use std::collections::HashSet;
    let mut order = Vec::new();
    let mut seen: HashSet<ModelId> = HashSet::new();
    visit(target, &mut order, &mut seen);
    order
}

fn visit(id: &ModelId, order: &mut Vec<ModelId>, seen: &mut std::collections::HashSet<ModelId>) {
    if !seen.insert(id.clone()) {
        return;
    }
    if let Some(manifest) = lookup(id) {
        for dep in manifest.requires {
            visit(dep, order, seen);
        }
    }
    order.push(id.clone());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_has_qwen_image_video_entries() {
        let ids: Vec<&ModelId> = MANIFEST.iter().map(|m| &m.id).collect();
        assert!(ids.contains(&&ModelId::Qwen3_5_4b));
        assert!(ids.contains(&&ModelId::SdxlTurbo));
        assert!(ids.contains(&&ModelId::SdxlLightningBase));
        assert!(ids.contains(&&ModelId::SdxlLightning4StepLora));
        assert!(ids.contains(&&ModelId::NunchakuFluxDevInt4));
        assert!(ids.contains(&&ModelId::NunchakuFluxTurboAlpha8stepLora));
        assert!(ids.contains(&&ModelId::ZImageTurboSvdq));
        assert!(ids.contains(&&ModelId::Qwen3_4bTextEncoder));
        assert!(ids.contains(&&ModelId::T5xxlEncoder));
        assert!(ids.contains(&&ModelId::LtxVideo09_6Distilled));
        assert_eq!(MANIFEST.len(), 13);
    }

    #[test]
    fn lookup_by_id_works() {
        let m = lookup(&ModelId::Qwen3_5_4b).unwrap();
        assert_eq!(m.hf_filename, "qwen3.5-4b-instruct-q4_k_m.gguf");
    }

    #[test]
    fn model_id_custom_variant_holds_repo_and_filename() {
        let id = ModelId::Custom {
            hf_repo: "Qwen/Qwen2.5-VL-7B-Instruct-GGUF".into(),
            gguf_filename: "qwen2.5-vl-7b-instruct-q4_k_m.gguf".into(),
            mmproj_filename: Some("mmproj-qwen2.5-vl-7b-f16.gguf".into()),
        };
        match &id {
            ModelId::Custom { hf_repo, .. } => assert!(hf_repo.contains("Qwen")),
            _ => panic!("expected Custom"),
        }
    }

    #[test]
    fn resolve_download_order_empty_deps_is_just_self() {
        let order = resolve_download_order(&ModelId::Qwen3_5_4b);
        assert_eq!(order, vec![ModelId::Qwen3_5_4b]);
    }

    #[test]
    fn resolve_download_order_for_flux_includes_t5_first() {
        let order = resolve_download_order(&ModelId::NunchakuFluxDevInt4);
        let t5_idx = order
            .iter()
            .position(|m| matches!(m, ModelId::T5xxlEncoder))
            .unwrap();
        let flux_idx = order
            .iter()
            .position(|m| matches!(m, ModelId::NunchakuFluxDevInt4))
            .unwrap();
        assert!(t5_idx < flux_idx);
    }

    #[test]
    fn resolve_download_order_custom_is_just_self() {
        let id = ModelId::Custom {
            hf_repo: "x".into(),
            gguf_filename: "y.gguf".into(),
            mmproj_filename: None,
        };
        let order = resolve_download_order(&id);
        assert_eq!(order.len(), 1);
    }
}
