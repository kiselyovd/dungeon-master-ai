//! Batch B integration test: a real spawned child process driven through the
//! production `ProcessSidecarLauncher` + real health probe, asserting the
//! `LocalRuntime` transitions to `Ready`.
//!
//! Uses the `stub_sidecar` bin (a tiny axum server) as a stand-in for
//! mistralrs-server so the test runs hermetically in CI without a model.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use app_server::local_runtime::port::discover_free_port;
use app_server::local_runtime::{
    probe_real, LocalRuntime, ProbeConfig, ProcessSidecarLauncher, RuntimeStatus,
};

#[tokio::test]
async fn process_launcher_spawns_stub_and_probe_reaches_ready() {
    // CARGO_BIN_EXE_stub_sidecar is set by Cargo for this package's
    // integration tests and points at the freshly built stub binary.
    let stub_path = PathBuf::from(env!("CARGO_BIN_EXE_stub_sidecar"));
    let bin_dir = stub_path.parent().expect("stub binary has a parent dir");

    let launcher = Arc::new(ProcessSidecarLauncher::new(bin_dir.to_path_buf()));
    let probe = probe_real(ProbeConfig {
        max_attempts: 10,
        initial_delay: Duration::from_millis(50),
    });
    let runtime = LocalRuntime::new(launcher, probe, "/health");

    let port = discover_free_port().expect("free port");
    let port_str = port.to_string();
    let status = runtime
        .start("stub_sidecar", &["--port", &port_str], port)
        .await
        .expect("start stub sidecar");

    // Stop unconditionally before the assertions so a failed assertion does
    // not orphan the spawned stub_sidecar child process.
    let stop_result = runtime.stop().await;
    let final_status = runtime.status().await;

    assert!(
        matches!(status, RuntimeStatus::Ready { port: p } if p == port),
        "expected Ready, got {status:?}"
    );
    stop_result.expect("stop stub sidecar");
    assert!(matches!(final_status, RuntimeStatus::Off));
}
