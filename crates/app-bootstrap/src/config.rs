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

/// Privileged desktop APIs must never become the public co-op listener.
pub fn verify_local_listener(address: std::net::SocketAddr) -> anyhow::Result<()> {
    if !address.ip().is_loopback() {
        anyhow::bail!("privileged desktop API requires a loopback listener");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn privileged_listener_accepts_only_ipv4_or_ipv6_loopback() {
        for address in ["127.0.0.1:0", "127.0.0.2:8144", "[::1]:0"] {
            super::verify_local_listener(address.parse().unwrap()).unwrap();
        }
        for address in [
            "0.0.0.0:0",
            "192.168.1.10:8144",
            "10.20.0.4:8144",
            "[::]:0",
            "[fd00::1]:8144",
        ] {
            assert!(super::verify_local_listener(address.parse().unwrap()).is_err());
        }
    }

    #[test]
    fn source_contains_no_default_vault_secret() {
        let source = include_str!("config.rs");
        let forbidden = ["default", "vault", "passphrase"].join("-");
        assert!(!source.contains(&forbidden));
    }
}
