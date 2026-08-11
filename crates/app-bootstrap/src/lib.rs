//! Production composition root for the backend sidecar.

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
    let app = wiring::build_router(&settings).await?;
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
