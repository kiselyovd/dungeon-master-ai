use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Context;
use app_domain::srd::embedder::{
    DEFAULT_EMBEDDING_MODEL, embedding_dim, parse_embedding_model,
};
use app_llm::{AnthropicProvider, LlmProvider, MockProvider};
use app_server::{AppState, config::Settings, db::init_db, db::srd_chunks_clear, router};
use sqlx::Row;
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

    // Resolve the embedding model (env override -> default, fall back if unknown).
    let requested_model_name = std::env::var("DMAI_EMBEDDING_MODEL")
        .unwrap_or_else(|_| DEFAULT_EMBEDDING_MODEL.to_string());

    // Resolve to (name, model). On parse failure we fall back to the default
    // and record the *default* name on AgentConfig - not the rejected one -
    // so context_builder later re-parses successfully.
    let (resolved_model_name, embedding_model) = match parse_embedding_model(&requested_model_name)
    {
        Ok(m) => (requested_model_name, m),
        Err(e) => {
            tracing::warn!(
                "invalid DMAI_EMBEDDING_MODEL='{requested_model_name}': {e}. Falling back to '{DEFAULT_EMBEDDING_MODEL}'."
            );
            let m = parse_embedding_model(DEFAULT_EMBEDDING_MODEL)
                .expect("default model must parse");
            (DEFAULT_EMBEDDING_MODEL.to_string(), m)
        }
    };

    let expected_dim = embedding_dim(&embedding_model);

    // Update AgentConfig stored on AppState to reflect the chosen model.
    {
        let mut cfg = state.agent_config();
        cfg.embedding_model = resolved_model_name.clone();
        state.set_agent_config(cfg);
    }

    // Cache invalidation: if any stored embedding has the wrong dim, clear
    // srd_chunks so the embed task below re-builds with the active model.
    if let Err(e) = invalidate_srd_cache_on_dim_mismatch(state.db(), expected_dim).await {
        tracing::warn!("srd cache invalidation check failed: {e}");
    }

    // Spawn background SRD embedding (downloads model on first run, fast after that).
    let state_clone = state.clone();
    let model_name_for_log = resolved_model_name.clone();
    tokio::task::spawn_blocking(move || {
        use app_domain::srd::embedder::embed_chunks;
        use app_domain::srd::loader::load_all_chunks;
        let chunks = load_all_chunks();
        match embed_chunks(chunks, embedding_model) {
            Ok(retriever) => {
                state_clone.set_srd_retriever(Arc::new(retriever));
                tracing::info!(model = %model_name_for_log, "SRD retriever ready");
            }
            Err(e) => {
                tracing::warn!("SRD embedding failed (RAG will be unavailable): {e}");
            }
        }
    });

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

/// If any stored `srd_chunks.embedding` blob has a different f32 dimension
/// than the active model, clear the cache so a fresh re-embed runs. Each
/// stored embedding is a little-endian f32 vector (4 bytes per element).
async fn invalidate_srd_cache_on_dim_mismatch(
    pool: &sqlx::SqlitePool,
    expected_dim: usize,
) -> Result<(), sqlx::Error> {
    let row = sqlx::query("SELECT embedding FROM srd_chunks WHERE embedding IS NOT NULL LIMIT 1")
        .fetch_optional(pool)
        .await?;
    if let Some(r) = row {
        let blob: Vec<u8> = r.try_get("embedding")?;
        let stored_dim = blob.len() / 4;
        if stored_dim != expected_dim {
            tracing::info!(
                stored_dim,
                expected_dim,
                "embedding model changed (dim mismatch); clearing srd_chunks cache"
            );
            srd_chunks_clear(pool).await?;
        }
    }
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
