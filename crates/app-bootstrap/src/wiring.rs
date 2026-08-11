//! Concrete adapter construction for the backend process.

use std::sync::Arc;

use adapter_llm::embeddings::{embedding_dim, parse_embedding_model, DEFAULT_EMBEDDING_MODEL};
use anyhow::Context;
use app_llm::{LlmProvider, MockProvider};
use app_server::secrets::StrongholdSecretsRepo;
use app_server::{db::init_db, db::srd_chunks_clear, AppState};
use axum::Router;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::Row;

use crate::{config, paths};

pub async fn build_router(settings: &config::Settings) -> anyhow::Result<Router> {
    let state = build_state(settings).await?;
    Ok(adapter_http::router(app_server::http_services::bundle(
        state,
    )))
}

pub async fn build_state(settings: &config::Settings) -> anyhow::Result<AppState> {
    let llm: Arc<dyn LlmProvider> = Arc::new(MockProvider::new(vec![]));
    tracing::info!(
        process_role = "backend",
        "using mock provider until settings are applied"
    );

    let pool = match std::env::var("DATABASE_URL") {
        Ok(db_url) => sqlx::SqlitePool::connect(&db_url)
            .await
            .with_context(|| format!("connect sqlite {db_url}"))?,
        Err(_) => {
            let db_path = paths::default_db_path();
            let options = SqliteConnectOptions::new()
                .filename(&db_path)
                .create_if_missing(true);
            sqlx::SqlitePool::connect_with(options)
                .await
                .with_context(|| format!("connect sqlite {}", db_path.display()))?
        }
    };
    init_db(&pool).await.context("run migrations")?;
    let state = AppState::new(llm, settings.default_model.clone(), pool);

    if let Some(control) = adapter_media::LoopbackRuntimeControl::from_env()
        .map_err(|error| anyhow::anyhow!(error.to_string()))?
    {
        state.set_runtime_control(Arc::new(control));
        tracing::info!(
            process_role = "backend",
            "desktop runtime control connected"
        );
    }

    configure_secrets(&state)?;
    configure_embeddings(&state).await;
    Ok(state)
}

fn configure_secrets(state: &AppState) -> anyhow::Result<()> {
    let Some(snapshot_path) = paths::vault_path() else {
        return Ok(());
    };
    let Some(passphrase) = config::vault_passphrase()? else {
        tracing::warn!(
            process_role = "backend",
            "vault passphrase absent in debug build; secrets remain in memory"
        );
        return Ok(());
    };
    let repo = StrongholdSecretsRepo::open(snapshot_path.clone(), passphrase)
        .with_context(|| format!("open encrypted vault {}", snapshot_path.display()))?;
    state.set_secrets_repo(Arc::new(repo));
    tracing::info!(process_role = "backend", "encrypted secrets adapter ready");
    Ok(())
}

async fn configure_embeddings(state: &AppState) {
    let requested = std::env::var("DMAI_EMBEDDING_MODEL")
        .unwrap_or_else(|_| DEFAULT_EMBEDDING_MODEL.to_string());
    let (resolved, model) = match parse_embedding_model(&requested) {
        Ok(model) => (requested, model),
        Err(error) => {
            tracing::warn!(code = "embedding_model_invalid", %error, "using default embedding model");
            (
                DEFAULT_EMBEDDING_MODEL.to_string(),
                parse_embedding_model(DEFAULT_EMBEDDING_MODEL).expect("default embedding model"),
            )
        }
    };
    let expected_dim = embedding_dim(&model);
    let mut agent_config = state.agent_config();
    agent_config.embedding_model = resolved.clone();
    state.set_agent_config(agent_config);
    if let Err(error) = invalidate_srd_cache(state.db(), expected_dim).await {
        tracing::warn!(code = "srd_cache_check_failed", %error, "SRD cache check failed");
    }

    let state = state.clone();
    tokio::task::spawn_blocking(move || {
        use adapter_llm::embeddings::{embed_chunks, load_all_chunks};
        match embed_chunks(load_all_chunks(), model) {
            Ok(retriever) => {
                state.set_srd_retriever(Arc::new(retriever));
                tracing::info!(model = %resolved, "SRD retriever ready");
            }
            Err(error) => tracing::warn!(code = "srd_embedding_failed", %error, "RAG unavailable"),
        }
    });
}

async fn invalidate_srd_cache(
    pool: &sqlx::SqlitePool,
    expected_dim: usize,
) -> Result<(), sqlx::Error> {
    let row = sqlx::query("SELECT embedding FROM srd_chunks WHERE embedding IS NOT NULL LIMIT 1")
        .fetch_optional(pool)
        .await?;
    if let Some(row) = row {
        let blob: Vec<u8> = row.try_get("embedding")?;
        if blob.len() / 4 != expected_dim {
            srd_chunks_clear(pool).await?;
        }
    }
    Ok(())
}
