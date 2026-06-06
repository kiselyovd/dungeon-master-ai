use std::env;

pub struct Settings {
    pub bind_addr: String,
    pub default_model: String,
}

impl Settings {
    pub fn from_env() -> Self {
        Self {
            bind_addr: env::var("DMAI_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:0".into()),
            // No provider is configured at boot; the active model is set by the
            // first /settings/v2 POST. Native Anthropic env bootstrap was
            // removed in M11 Batch D.5.
            default_model: env::var("DMAI_DEFAULT_MODEL").unwrap_or_default(),
        }
    }
}
