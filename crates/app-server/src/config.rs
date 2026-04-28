use std::env;

pub struct Settings {
    pub bind_addr: String,
    pub anthropic_api_key: Option<String>,
    pub default_model: String,
}

impl Settings {
    pub fn from_env() -> Self {
        Self {
            bind_addr: env::var("DMAI_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:0".into()),
            anthropic_api_key: env::var("ANTHROPIC_API_KEY")
                .ok()
                .filter(|k| !k.trim().is_empty()),
            default_model: env::var("DMAI_DEFAULT_MODEL")
                .unwrap_or_else(|_| "claude-haiku-4-5-20251001".into()),
        }
    }
}
