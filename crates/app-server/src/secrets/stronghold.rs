//! Production `SecretsRepo` backed by `iota_stronghold` 2.x snapshot files.
//!
//! Encrypted at rest via the standard Stronghold layered protection:
//! the snapshot file is sealed by a 32-byte key derived from the
//! supplied passphrase via Blake2b256, with KeyProvider's NCKey
//! holding the unlocked key in continuously-rotated locked memory
//! whenever a write needs to fire.
//!
//! Architecturally the dmai-server sidecar is normally re-populated
//! from the frontend's Stronghold-encrypted secrets via /settings POST,
//! so this repo's role is to make the backend resilient to frontend-less
//! restarts: a cached secret survives a restart without the frontend
//! having to re-deliver it. New backend-originated secrets (none exist
//! today) would also drop in here.
//!
//! Lifetime: the underlying `Stronghold` and `KeyProvider` hold no
//! `Send + Sync + 'static` futures internally - operations are sync but
//! perform disk IO during commit, so async callers wrap each call in
//! `tokio::task::spawn_blocking` to keep the Tokio runtime responsive.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use iota_stronghold::{KeyProvider, SnapshotPath, Stronghold};
use tokio::sync::Mutex;

use super::repo::{SecretsError, SecretsRepo};

const CLIENT_PATH: &[u8] = b"dmai-server-secrets";

/// Encrypted-at-rest secrets repository.
pub struct StrongholdSecretsRepo {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    stronghold: Stronghold,
    snapshot_path: SnapshotPath,
    passphrase: Vec<u8>,
}

impl StrongholdSecretsRepo {
    /// Open or create a Stronghold snapshot at `snapshot_path`, sealed by
    /// `passphrase`. Returns an error if the snapshot file exists but the
    /// passphrase fails to decrypt it.
    pub fn open(snapshot_path: PathBuf, passphrase: Vec<u8>) -> Result<Self, SecretsError> {
        let stronghold = Stronghold::default();
        let snapshot_path = SnapshotPath::from_path(snapshot_path);

        // The stronghold session is empty; if a snapshot file already
        // exists on disk, decrypt it and pull our client into the
        // session. Otherwise create a fresh client. Either way the
        // session has the named client loaded after this returns.
        if snapshot_path.exists() {
            let key_provider = build_key_provider(&passphrase)?;
            stronghold
                .load_client_from_snapshot(CLIENT_PATH, &key_provider, &snapshot_path)
                .map_err(|e| SecretsError::Vault(format!("load_client_from_snapshot: {e}")))?;
        } else {
            stronghold
                .create_client(CLIENT_PATH)
                .map_err(|e| SecretsError::Vault(format!("create_client: {e}")))?;
        }

        Ok(Self {
            inner: Arc::new(Mutex::new(Inner {
                stronghold,
                snapshot_path,
                passphrase,
            })),
        })
    }
}

fn build_key_provider(passphrase: &[u8]) -> Result<KeyProvider, SecretsError> {
    KeyProvider::with_passphrase_hashed_blake2b(passphrase.to_vec())
        .map_err(|e| SecretsError::Vault(format!("key provider: {e}")))
}

#[async_trait]
impl SecretsRepo for StrongholdSecretsRepo {
    async fn get(&self, key: &str) -> Result<Option<String>, SecretsError> {
        let inner = self.inner.clone();
        let key = key.to_owned();
        tokio::task::spawn_blocking(move || {
            let inner = inner.blocking_lock();
            let client = inner
                .stronghold
                .get_client(CLIENT_PATH)
                .map_err(|e| SecretsError::Vault(format!("get_client: {e}")))?;
            let store = client.store();
            let bytes = store
                .get(key.as_bytes())
                .map_err(|e| SecretsError::Vault(format!("store get: {e}")))?;
            match bytes {
                Some(bytes) => Ok(Some(
                    String::from_utf8(bytes)
                        .map_err(|e| SecretsError::Vault(format!("utf8: {e}")))?,
                )),
                None => Ok(None),
            }
        })
        .await
        .map_err(|e| SecretsError::Vault(format!("join: {e}")))?
    }

    async fn set(&self, key: &str, value: &str) -> Result<(), SecretsError> {
        let inner = self.inner.clone();
        let key = key.to_owned();
        let value = value.to_owned();
        tokio::task::spawn_blocking(move || {
            let inner = inner.blocking_lock();
            let client = inner
                .stronghold
                .get_client(CLIENT_PATH)
                .map_err(|e| SecretsError::Vault(format!("get_client: {e}")))?;
            client
                .store()
                .insert(key.into_bytes(), value.into_bytes(), None)
                .map_err(|e| SecretsError::Vault(format!("store insert: {e}")))?;
            commit(&inner)?;
            Ok(())
        })
        .await
        .map_err(|e| SecretsError::Vault(format!("join: {e}")))?
    }

    async fn delete(&self, key: &str) -> Result<(), SecretsError> {
        let inner = self.inner.clone();
        let key = key.to_owned();
        tokio::task::spawn_blocking(move || {
            let inner = inner.blocking_lock();
            let client = inner
                .stronghold
                .get_client(CLIENT_PATH)
                .map_err(|e| SecretsError::Vault(format!("get_client: {e}")))?;
            client
                .store()
                .delete(key.as_bytes())
                .map_err(|e| SecretsError::Vault(format!("store delete: {e}")))?;
            commit(&inner)?;
            Ok(())
        })
        .await
        .map_err(|e| SecretsError::Vault(format!("join: {e}")))?
    }
}

fn commit(inner: &Inner) -> Result<(), SecretsError> {
    let key_provider = build_key_provider(&inner.passphrase)?;
    inner
        .stronghold
        .commit_with_keyprovider(&inner.snapshot_path, &key_provider)
        .map_err(|e| SecretsError::Vault(format!("commit: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn open_then_round_trip_in_memory() {
        let dir = tempdir().unwrap();
        let snapshot = dir.path().join("vault.hold");
        let repo = StrongholdSecretsRepo::open(snapshot, b"test-passphrase".to_vec()).unwrap();

        repo.set("anthropic", "sk-ant-real").await.unwrap();
        assert_eq!(
            repo.get("anthropic").await.unwrap(),
            Some("sk-ant-real".to_string())
        );

        repo.delete("anthropic").await.unwrap();
        assert_eq!(repo.get("anthropic").await.unwrap(), None);
    }

    #[tokio::test]
    async fn missing_key_returns_none() {
        let dir = tempdir().unwrap();
        let snapshot = dir.path().join("vault.hold");
        let repo = StrongholdSecretsRepo::open(snapshot, b"pw".to_vec()).unwrap();
        assert_eq!(repo.get("absent").await.unwrap(), None);
    }

    #[tokio::test]
    async fn snapshot_round_trips_across_open_calls() {
        let dir = tempdir().unwrap();
        let snapshot = dir.path().join("vault.hold");
        let pw = b"shared-passphrase".to_vec();

        // First session writes a value.
        {
            let repo = StrongholdSecretsRepo::open(snapshot.clone(), pw.clone()).unwrap();
            repo.set("openai", "sk-openai-test").await.unwrap();
        }

        // Drop the first repo entirely, then reopen and read back.
        let repo = StrongholdSecretsRepo::open(snapshot, pw).unwrap();
        assert_eq!(
            repo.get("openai").await.unwrap(),
            Some("sk-openai-test".to_string())
        );
    }

    #[tokio::test]
    async fn wrong_passphrase_on_existing_snapshot_errors() {
        let dir = tempdir().unwrap();
        let snapshot = dir.path().join("vault.hold");

        // Seal a snapshot with one passphrase.
        {
            let repo =
                StrongholdSecretsRepo::open(snapshot.clone(), b"correct-pw".to_vec()).unwrap();
            repo.set("k", "v").await.unwrap();
        }

        // Try to reopen with the wrong one - must surface a Vault error.
        let err = StrongholdSecretsRepo::open(snapshot, b"wrong-pw".to_vec());
        assert!(err.is_err(), "wrong passphrase should not silently succeed");
    }
}
