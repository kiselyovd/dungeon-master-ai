pub mod migrate;
pub mod repo;
pub mod stronghold;

pub use migrate::{migrate_secrets_json, MigrationResult};
pub use repo::{InMemorySecretsRepo, SecretsError, SecretsRepo};
pub use stronghold::StrongholdSecretsRepo;

/// Vault key under which the Hugging Face access token is stored. Single
/// well-known constant so the HF handlers, the migration shim, and any
/// future surface (CLI, settings panel) all agree on the slot.
pub const HF_TOKEN_KEY: &str = "huggingface_token";

/// Convenience wrapper around `SecretsRepo::get` that flattens the
/// `Result<Option<_>>` to `Option<_>`. The HF route handlers never need to
/// distinguish "missing" from "vault read failed" - both surface to the user
/// as "no token", and `connected: false` is the right UX for both.
pub async fn get_hf_token(repo: &dyn SecretsRepo) -> Option<String> {
    repo.get(HF_TOKEN_KEY).await.ok().flatten()
}

/// Persist the HF token into the vault.
pub async fn set_hf_token(repo: &dyn SecretsRepo, token: &str) -> Result<(), SecretsError> {
    repo.set(HF_TOKEN_KEY, token).await
}

/// Wipe the HF token from the vault. Used by `DELETE /hf/token`.
pub async fn clear_hf_token(repo: &dyn SecretsRepo) -> Result<(), SecretsError> {
    repo.delete(HF_TOKEN_KEY).await
}

#[cfg(test)]
mod hf_token_helper_tests {
    use super::*;

    #[tokio::test]
    async fn hf_token_helpers_round_trip() {
        let repo = InMemorySecretsRepo::default();
        assert!(get_hf_token(&repo).await.is_none());
        set_hf_token(&repo, "hf_test_token").await.unwrap();
        assert_eq!(get_hf_token(&repo).await.as_deref(), Some("hf_test_token"));
        clear_hf_token(&repo).await.unwrap();
        assert!(get_hf_token(&repo).await.is_none());
    }
}
