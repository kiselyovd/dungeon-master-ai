use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Context;
use app_llm::{AnthropicProvider, LlmProvider, MockProvider};
use app_server::{AppState, config::Settings, router};
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let settings = Settings::from_env();

    let llm: Arc<dyn LlmProvider> = match settings.anthropic_api_key.clone() {
        Some(key) => Arc::new(AnthropicProvider::new(key)),
        None => {
            tracing::warn!(
                "ANTHROPIC_API_KEY not set; using MockProvider (chat will return canned data)"
            );
            Arc::new(MockProvider::new(vec![]))
        }
    };

    let state = AppState {
        llm,
        default_model: settings.default_model.clone(),
    };

    let listener = TcpListener::bind(&settings.bind_addr)
        .await
        .with_context(|| format!("bind {}", settings.bind_addr))?;
    let addr: SocketAddr = listener.local_addr().context("local_addr")?;

    println!(
        "APP_SERVER_LISTENING port={} host={}",
        addr.port(),
        addr.ip()
    );
    info!(?addr, "app-server listening");

    axum::serve(listener, router(state)).await.context("serve")?;
    Ok(())
}

fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .init();
}
