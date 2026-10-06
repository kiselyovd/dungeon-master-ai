//! Production composition root for the backend sidecar.

mod authority;
pub mod config;
pub mod paths;
pub mod telemetry;
pub mod wiring;

use std::net::SocketAddr;

use anyhow::Context;
use tokio::net::TcpListener;
use tracing::info;

/// Build every concrete adapter, bind the loopback HTTP listener, and serve.
/// The stdout readiness line is a stable machine contract consumed by Tauri.
pub async fn run() -> anyhow::Result<()> {
    let _telemetry = telemetry::init().context("init telemetry")?;
    let settings = config::Settings::from_env();
    let authority = authority::retain_for_process()?;
    let state = wiring::build_state(&settings).await?;
    if authority {
        adapter_sqlite::SqliteStore::new(state.db().clone())
            .recover_after_exclusive_authority(state.process_id())
            .await
            .context("reconcile interrupted agent operations")?;
    }
    let app = adapter_http::router(app_server::http_services::bundle(state));
    let listener = TcpListener::bind(&settings.bind_addr)
        .await
        .with_context(|| format!("bind {}", settings.bind_addr))?;
    let addr: SocketAddr = listener.local_addr().context("local_addr")?;

    println!(
        "APP_SERVER_LISTENING port={} host={}",
        addr.port(),
        addr.ip()
    );
    info!(process_role = "backend", ?addr, "app-server listening");
    axum::serve(listener, app).await.context("serve")?;
    Ok(())
}
