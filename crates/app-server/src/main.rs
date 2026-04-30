use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Context;
use app_llm::{AnthropicProvider, LlmProvider, MockProvider};
use app_server::{AppState, config::Settings, db::init_db, router};
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

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data.db".into());
    let pool = sqlx::SqlitePool::connect(&db_url)
        .await
        .with_context(|| format!("connect sqlite {db_url}"))?;
    init_db(&pool).await.context("run migrations")?;

    let state = AppState::new(llm, settings.default_model.clone(), pool);

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
