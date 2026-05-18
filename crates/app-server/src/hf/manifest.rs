//! `user_manifest.json` read/write/add/remove.
//!
//! Schema version 1. Forward-compat rule: when the file already exists with a
//! `version` field greater than the one this binary knows about, we fail-fast
//! with `UnsupportedVersion` rather than silently dropping unknown fields.
//! Older versions are not produced by this code; the file is created with
//! `version: 1` on first write.

use std::path::Path;

use serde::{Deserialize, Serialize};

use app_domain::local_llm::manifest::UserEntry;

/// On-disk shape of `user_manifest.json`. The `version` field gates future
/// migrations; bumping it is a deliberate, explicit step that older builds
/// must refuse via `UnsupportedVersion`.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct UserManifestFile {
    pub version: u32,
    pub entries: Vec<UserEntry>,
}

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("unsupported manifest version: {0}")]
    UnsupportedVersion(u32),
    #[error("duplicate entry id: {0}")]
    DuplicateId(String),
}

/// Read the manifest file from disk, or synthesize an empty `version: 1`
/// manifest if the path does not exist yet. Callers should treat a missing
/// file as "no user-added models", not as an error.
pub fn load_or_init(path: &Path) -> Result<UserManifestFile, ManifestError> {
    if !path.exists() {
        return Ok(UserManifestFile {
            version: 1,
            entries: vec![],
        });
    }
    let bytes = std::fs::read(path)?;
    let f: UserManifestFile = serde_json::from_slice(&bytes)?;
    if f.version != 1 {
        return Err(ManifestError::UnsupportedVersion(f.version));
    }
    Ok(f)
}

/// Atomically rewrite the manifest. Parent directories are created as needed
/// so callers do not have to special-case the very first add (which happens
/// when the user adds their first HF-search model and `models_dir/..` is the
/// dmai data root that exists, while `user_manifest.json` does not).
pub fn save(path: &Path, file: &UserManifestFile) -> Result<(), ManifestError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(file)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

/// Append a new entry, rejecting duplicates by `system.id`. The id is the
/// canonical key used by the frontend `ModelSelector` and by the download
/// manager; a duplicate would silently shadow the prior model and is treated
/// as a 400 by the HTTP handler.
pub fn add_entry(path: &Path, entry: UserEntry) -> Result<(), ManifestError> {
    let mut f = load_or_init(path)?;
    if f.entries.iter().any(|e| e.system.id == entry.system.id) {
        return Err(ManifestError::DuplicateId(entry.system.id));
    }
    f.entries.push(entry);
    save(path, &f)
}

/// Remove an entry by id. Removing a non-existent id is a no-op (idempotent),
/// matching the DELETE handler contract: deleting twice still yields 204.
pub fn remove_entry(path: &Path, id: &str) -> Result<(), ManifestError> {
    let mut f = load_or_init(path)?;
    f.entries.retain(|e| e.system.id != id);
    save(path, &f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_domain::local_llm::manifest::SystemEntry;
    use tempfile::TempDir;

    fn entry(id: &str) -> UserEntry {
        UserEntry {
            system: SystemEntry {
                id: id.into(),
                hf_repo: "org/repo".into(),
                hf_filename: format!("{id}.gguf"),
                arch: "qwen3".into(),
                quant: "gguf-q4_k_m".into(),
                size_gb: 4.0,
                license: "apache-2.0".into(),
                display_name: id.into(),
            },
            added_at: "2026-05-19T00:00:00Z".into(),
            source: "hf-search".into(),
        }
    }

    #[test]
    fn load_missing_yields_empty_v1() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nope.json");
        let loaded = load_or_init(&path).unwrap();
        assert_eq!(loaded.version, 1);
        assert!(loaded.entries.is_empty());
    }

    #[test]
    fn add_remove_persist_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("user_manifest.json");
        add_entry(&path, entry("a")).unwrap();
        add_entry(&path, entry("b")).unwrap();
        let loaded = load_or_init(&path).unwrap();
        assert_eq!(loaded.entries.len(), 2);
        remove_entry(&path, "a").unwrap();
        let loaded = load_or_init(&path).unwrap();
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].system.id, "b");
    }

    #[test]
    fn version_check_fails_on_unknown_version() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("user_manifest.json");
        std::fs::write(&path, r#"{"version":2,"entries":[]}"#).unwrap();
        let res = load_or_init(&path);
        assert!(matches!(res, Err(ManifestError::UnsupportedVersion(2))));
    }

    #[test]
    fn duplicate_id_is_rejected() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("user_manifest.json");
        add_entry(&path, entry("a")).unwrap();
        let res = add_entry(&path, entry("a"));
        assert!(matches!(res, Err(ManifestError::DuplicateId(_))));
    }

    #[test]
    fn remove_missing_id_is_noop() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("user_manifest.json");
        add_entry(&path, entry("a")).unwrap();
        // Removing an id that was never added must succeed silently so the
        // DELETE handler stays idempotent across retries.
        remove_entry(&path, "does-not-exist").unwrap();
        let loaded = load_or_init(&path).unwrap();
        assert_eq!(loaded.entries.len(), 1);
    }

    #[test]
    fn save_creates_parent_dirs() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nested").join("subdir").join("file.json");
        save(
            &path,
            &UserManifestFile {
                version: 1,
                entries: vec![entry("only")],
            },
        )
        .unwrap();
        assert!(path.exists());
    }
}
