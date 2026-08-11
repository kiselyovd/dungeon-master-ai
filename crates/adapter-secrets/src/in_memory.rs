use std::collections::HashMap;

use app_application::ports::secrets::{SecretsError, SecretsStore};
use async_trait::async_trait;
use tokio::sync::RwLock;

#[derive(Default)]
pub struct InMemorySecretsStore {
    values: RwLock<HashMap<String, String>>,
}

#[async_trait]
impl SecretsStore for InMemorySecretsStore {
    async fn get(&self, key: &str) -> Result<Option<String>, SecretsError> {
        Ok(self.values.read().await.get(key).cloned())
    }

    async fn set(&self, key: &str, value: &str) -> Result<(), SecretsError> {
        self.values
            .write()
            .await
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), SecretsError> {
        self.values.write().await.remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn round_trip_delete_and_missing_key_match_the_port_contract() {
        let store = InMemorySecretsStore::default();
        assert_eq!(store.get("missing").await.unwrap(), None);
        store
            .set("openai_compat_api_key", "test-value")
            .await
            .unwrap();
        assert_eq!(
            store.get("openai_compat_api_key").await.unwrap().as_deref(),
            Some("test-value")
        );
        store.delete("openai_compat_api_key").await.unwrap();
        assert_eq!(store.get("openai_compat_api_key").await.unwrap(), None);
    }
}
