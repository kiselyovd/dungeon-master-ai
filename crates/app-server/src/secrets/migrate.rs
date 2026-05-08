//! One-shot migration: copy plaintext `secrets.json` keys into a `SecretsRepo`,
//! then drop a `.secrets_migrated_v1` sentinel so subsequent boots skip the
//! migration. The original `secrets.json` is renamed `.json.bak` (not deleted)
//! so users can recover if migration to the vault was incomplete.

use crate::secrets::repo::{SecretsError, SecretsRepo};
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct MigrationResult {
    pub migrated_keys: Vec<String>,
}

pub async fn migrate_secrets_json(
    base_dir: &Path,
    dest: Arc<dyn SecretsRepo>,
) -> Result<MigrationResult, SecretsError> {
    let sentinel = base_dir.join(".secrets_migrated_v1");
    if sentinel.exists() {
        return Ok(MigrationResult::default());
    }
    if !base_dir.exists() {
        std::fs::create_dir_all(base_dir)?;
    }
    let json_path = base_dir.join("secrets.json");
    let mut migrated = Vec::new();
    if json_path.exists() {
        let raw = std::fs::read_to_string(&json_path)?;
        let map: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&raw)
            .map_err(|e| SecretsError::Vault(e.to_string()))?;
        for (k, v) in map {
            if let Some(val) = v.as_str() {
                dest.set(&k, val).await?;
                migrated.push(k);
            }
        }
        let bak = json_path.with_extension("json.bak");
        std::fs::rename(&json_path, &bak)?;
    }
    std::fs::write(&sentinel, b"")?;
    tracing::info!(migrated_keys = ?migrated, "secrets migration complete");
    Ok(MigrationResult {
        migrated_keys: migrated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::repo::InMemorySecretsRepo;
    use tempfile::TempDir;

    #[tokio::test]
    async fn migrates_keys_then_marks_sentinel() {
        let tmp = TempDir::new().unwrap();
        let secrets_json = tmp.path().join("secrets.json");
        std::fs::write(
            &secrets_json,
            r#"{"anthropic_api_key":"sk-foo","replicate_api_key":"rk-bar"}"#,
        )
        .unwrap();
        let dest = Arc::new(InMemorySecretsRepo::default());
        let result = migrate_secrets_json(tmp.path(), dest.clone())
            .await
            .unwrap();
        let mut keys = result.migrated_keys.clone();
        keys.sort();
        assert_eq!(keys, vec!["anthropic_api_key", "replicate_api_key"]);
        assert!(tmp.path().join(".secrets_migrated_v1").exists());
        assert!(!secrets_json.exists());
        assert!(tmp.path().join("secrets.json.bak").exists());
        assert_eq!(
            dest.get("anthropic_api_key").await.unwrap(),
            Some("sk-foo".into())
        );
    }

    #[tokio::test]
    async fn idempotent_when_sentinel_exists() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".secrets_migrated_v1"), "").unwrap();
        let dest = Arc::new(InMemorySecretsRepo::default());
        let result = migrate_secrets_json(tmp.path(), dest).await.unwrap();
        assert!(result.migrated_keys.is_empty());
    }

    #[tokio::test]
    async fn no_op_when_secrets_json_missing() {
        let tmp = TempDir::new().unwrap();
        let dest = Arc::new(InMemorySecretsRepo::default());
        let result = migrate_secrets_json(tmp.path(), dest).await.unwrap();
        assert!(result.migrated_keys.is_empty());
        assert!(tmp.path().join(".secrets_migrated_v1").exists());
    }
}
