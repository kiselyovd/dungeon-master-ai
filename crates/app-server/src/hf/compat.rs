//! Compatibility whitelist: which arches and quant patterns the local
//! mistralrs sidecar can actually serve.

pub const SUPPORTED_ARCH: &[&str] = &[
    "qwen2", "qwen3", "llama2", "llama3", "llama", "mistral", "mixtral", "phi3", "gemma2", "gemma",
];

const SUPPORTED_QUANT_SUFFIXES: &[&str] = &[
    "-q4_k_m.gguf",
    "-q5_k_m.gguf",
    "-q8_0.gguf",
    "-f16.gguf",
    ".safetensors",
];

pub fn is_compat_arch(arch: &str) -> bool {
    let a = arch.to_lowercase();
    SUPPORTED_ARCH.iter().any(|s| *s == a)
}

pub fn is_compat_quant(filename: &str) -> bool {
    let f = filename.to_lowercase();
    SUPPORTED_QUANT_SUFFIXES.iter().any(|sfx| f.ends_with(sfx))
}

pub fn detect_arch(tags: &[String]) -> Option<String> {
    for t in tags {
        let lower = t.to_lowercase();
        if SUPPORTED_ARCH.iter().any(|s| lower == *s) {
            return Some(lower);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_arch_basic() {
        assert!(is_compat_arch("qwen3"));
        assert!(is_compat_arch("Qwen3"));
        assert!(!is_compat_arch("falcon"));
    }

    #[test]
    fn supported_quant_basic() {
        assert!(is_compat_quant("model-q4_k_m.gguf"));
        assert!(is_compat_quant("WEIRD-CASE-Q4_K_M.GGUF"));
        assert!(!is_compat_quant("model-q2_k.gguf"));
        assert!(!is_compat_quant("README.md"));
    }

    #[test]
    fn detect_arch_from_tags() {
        let tags = vec![
            "text-generation".into(),
            "qwen3".into(),
            "license:apache-2.0".into(),
        ];
        assert_eq!(detect_arch(&tags), Some("qwen3".into()));
    }
}
