//! Test-only stub sidecar. Binds `127.0.0.1:<--port>` and answers HTTP 200 on
//! the health endpoints a real sidecar would serve (`/health`, `/healthz`,
//! `/v1/models`). Used by `tests/local_runtime_probe.rs` to exercise the
//! `ProcessSidecarLauncher` + health probe against a real child process
//! without needing a multi-gigabyte mistralrs-server build.

use std::net::{Ipv4Addr, SocketAddrV4};

#[tokio::main]
async fn main() {
    let mut port: u16 = 0;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--port" {
            port = args
                .next()
                .and_then(|p| p.parse().ok())
                .expect("--port requires a numeric argument");
        }
    }

    let app = axum::Router::new()
        .route("/health", axum::routing::get(|| async { "ok" }))
        .route("/healthz", axum::routing::get(|| async { "ok" }))
        .route("/v1/models", axum::routing::get(|| async { "{}" }));

    let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("stub sidecar bind");
    let bound_port = listener
        .local_addr()
        .expect("stub sidecar local_addr")
        .port();
    println!("STUB_SIDECAR_LISTENING port={bound_port}");
    axum::serve(listener, app)
        .await
        .expect("stub sidecar serve");
}
