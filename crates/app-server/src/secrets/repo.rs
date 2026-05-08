//! Async-trait secrets store. `InMemorySecretsRepo` is for tests + dev mode;
//! `StrongholdSecretsRepo` (Phase E.2) is the production-backed implementation.

use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Debug, thiserror::Error)]
pub enum SecretsError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("vault: {0}")]
    Vault(String),
}

#[async_trait]
pub trait SecretsRepo: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<String>, SecretsError>;
    async fn set(&self, key: &str, value: &str) -> Result<(), SecretsError>;
    async fn delete(&self, key: &str) -> Result<(), SecretsError>;
}

#[derive(Default)]
pub struct InMemorySecretsRepo {
    store: RwLock<HashMap<String, String>>,
}

#[async_trait]
impl SecretsRepo for InMemorySecretsRepo {
    async fn get(&self, key: &str) -> Result<Option<String>, SecretsError> {
        Ok(self.store.read().await.get(key).cloned())
    }

    async fn set(&self, key: &str, value: &str) -> Result<(), SecretsError> {
        self.store
            .write()
            .await
            .insert(key.into(), value.into());
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), SecretsError> {
        self.store.write().await.remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn in_memory_repo_round_trips() {
        let repo = InMemorySecretsRepo::default();
        repo.set("anthropic", "sk-test").await.unwrap();
        assert_eq!(
            repo.get("anthropic").await.unwrap(),
            Some("sk-test".into())
        );
        repo.delete("anthropic").await.unwrap();
        assert_eq!(repo.get("anthropic").await.unwrap(), None);
    }

    #[tokio::test]
    async fn missing_key_returns_none() {
        let repo = InMemorySecretsRepo::default();
        assert_eq!(repo.get("absent").await.unwrap(), None);
    }
}
