//! Environment-backed bootstrap configuration.

pub use app_server::config::Settings;

/// A production build must receive its vault credential from the process
/// environment. Debug builds may run without persistent secrets and fall back
/// to the in-memory adapter, but never use a built-in passphrase.
pub fn vault_passphrase() -> anyhow::Result<Option<Vec<u8>>> {
    match std::env::var("DMAI_VAULT_PASSPHRASE") {
        Ok(value) if !value.is_empty() => Ok(Some(value.into_bytes())),
        Ok(_) | Err(std::env::VarError::NotPresent) if cfg!(debug_assertions) => Ok(None),
        Ok(_) | Err(std::env::VarError::NotPresent) => {
            anyhow::bail!("DMAI_VAULT_PASSPHRASE is required for a production backend")
        }
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn source_contains_no_default_vault_secret() {
        let source = include_str!("config.rs");
        let forbidden = ["default", "vault", "passphrase"].join("-");
        assert!(!source.contains(&forbidden));
    }
}
