//! One-shot migration: copy plaintext `secrets.json` keys into a `SecretsStore`,
//! then drop a `.secrets_migrated_v1` sentinel so subsequent boots skip the
//! migration. The original `secrets.json` is renamed `.json.bak` (not deleted)
//! so users can recover if migration to the vault was incomplete.

use app_application::ports::secrets::{SecretsError, SecretsStore};
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct MigrationResult {
    pub migrated_keys: Vec<String>,
}

pub async fn migrate_secrets_json(
    base_dir: &Path,
    dest: Arc<dyn SecretsStore>,
) -> Result<MigrationResult, SecretsError> {
    let sentinel = base_dir.join(".secrets_migrated_v1");
    if sentinel.exists() {
        return Ok(MigrationResult::default());
    }
    if !base_dir.exists() {
        std::fs::create_dir_all(base_dir).map_err(|_| migration_error("create_directory"))?;
    }
    let json_path = base_dir.join("secrets.json");
    let mut migrated = Vec::new();
    let mut originals = Vec::new();
    if json_path.exists() {
        let raw =
            std::fs::read_to_string(&json_path).map_err(|_| migration_error("read_legacy_file"))?;
        let map: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(&raw).map_err(|_| migration_error("decode_legacy_file"))?;
        for (k, v) in map {
            if let Some(val) = v.as_str() {
                let original = dest.get(&k).await?;
                if let Err(error) = dest.set(&k, val).await {
                    restore_originals(dest.as_ref(), &originals).await;
                    return Err(error);
                }
                originals.push((k.clone(), original));
                migrated.push(k);
            }
        }
        let bak = json_path.with_extension("json.bak");
        if std::fs::rename(&json_path, &bak).is_err() {
            restore_originals(dest.as_ref(), &originals).await;
            return Err(migration_error("backup_legacy_file"));
        }
    }
    std::fs::write(&sentinel, b"").map_err(|_| migration_error("write_sentinel"))?;
    tracing::info!(migrated_keys = ?migrated, "secrets migration complete");
    Ok(MigrationResult {
        migrated_keys: migrated,
    })
}

async fn restore_originals(dest: &dyn SecretsStore, originals: &[(String, Option<String>)]) {
    for (key, value) in originals.iter().rev() {
        let result = match value {
            Some(value) => dest.set(key, value).await,
            None => dest.delete(key).await,
        };
        if result.is_err() {
            tracing::error!(key_category = %key, "secret migration rollback failed");
        }
    }
}

fn migration_error(code: &'static str) -> SecretsError {
    SecretsError::Operation {
        operation: "migrate",
        code,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InMemorySecretsStore;
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
        let dest = Arc::new(InMemorySecretsStore::default());
        let result = migrate_secrets_json(tmp.path(), dest.clone())
            .await
            .unwrap();
        let mut keys = result.migrated_keys.clone();
        keys.sort();
        // The migration is key-name agnostic: it copies whatever the legacy
        // file holds. A migrated `anthropic_api_key` (from a pre-D.5 install)
        // is later purged on the first /settings/v2 cloud save (see
        // routes/settings/mod.rs); migrating it here is correct and harmless.
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
        let dest = Arc::new(InMemorySecretsStore::default());
        let result = migrate_secrets_json(tmp.path(), dest).await.unwrap();
        assert!(result.migrated_keys.is_empty());
    }

    #[tokio::test]
    async fn no_op_when_secrets_json_missing() {
        let tmp = TempDir::new().unwrap();
        let dest = Arc::new(InMemorySecretsStore::default());
        let result = migrate_secrets_json(tmp.path(), dest).await.unwrap();
        assert!(result.migrated_keys.is_empty());
        assert!(tmp.path().join(".secrets_migrated_v1").exists());
    }
}
