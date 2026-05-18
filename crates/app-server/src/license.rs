//! License classification for the catalog. Used by /settings/v2 to filter
//! non-OSS providers when behavior.license_restricted_mode is on.
//!
//! Strict whitelist: only well-known permissive OSS licenses pass. Anything
//! with NC (non-commercial), ToS-bound clouds, "varies" / "varies per model"
//! (which we cannot statically verify), or research licenses (OpenRAIL-M,
//! LTX) is treated as restricted.

/// Returns true if the catalog `license` field is a permissive OSS license
/// that is safe under license_restricted_mode. Anything else (proprietary,
/// non-commercial, research, ambiguous "varies") is restricted.
pub fn is_oss_license(license: &str) -> bool {
    let trimmed = license.trim();
    // Match "Apache 2.0", "Apache 2.0 (Qwen)", "Apache-2.0", etc.
    if trimmed.starts_with("Apache 2.0") || trimmed.starts_with("Apache-2.0") {
        return true;
    }
    if trimmed == "MIT" || trimmed.starts_with("MIT ") {
        return true;
    }
    if trimmed == "BSD-3-Clause" || trimmed.starts_with("BSD ") {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apache_variants_pass() {
        assert!(is_oss_license("Apache 2.0"));
        assert!(is_oss_license("Apache 2.0 (Qwen)"));
        assert!(is_oss_license("Apache-2.0"));
        assert!(is_oss_license("  Apache 2.0  "));
    }

    #[test]
    fn mit_and_bsd_pass() {
        assert!(is_oss_license("MIT"));
        assert!(is_oss_license("MIT License"));
        assert!(is_oss_license("BSD-3-Clause"));
    }

    #[test]
    fn proprietary_and_nc_fail() {
        assert!(!is_oss_license("Anthropic ToS"));
        assert!(!is_oss_license("varies"));
        assert!(!is_oss_license("varies per model"));
        assert!(!is_oss_license("SAI NC"));
        assert!(!is_oss_license("FLUX-dev NC"));
        assert!(!is_oss_license("OpenRAIL-M"));
        assert!(!is_oss_license("LTX (re-check before GA)"));
        assert!(!is_oss_license(""));
        assert!(!is_oss_license("Proprietary"));
    }
}
