use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SecretsError {
    #[error("secret operation {operation} failed with code {code}")]
    Operation {
        operation: &'static str,
        code: &'static str,
    },
}

#[async_trait]
pub trait SecretsStore: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<String>, SecretsError>;
    async fn set(&self, key: &str, value: &str) -> Result<(), SecretsError>;
    async fn delete(&self, key: &str) -> Result<(), SecretsError>;
}
