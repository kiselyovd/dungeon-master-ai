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

#[tokio::test]
async fn process_launcher_probes_configured_healthz_path() {
    // The Python image sidecar serves /healthz, not /health. This exercises
    // the same real-spawned-process path as the test above but on the image
    // runtime's health endpoint, proving the per-runtime path reaches Ready.
    let stub_path = PathBuf::from(env!("CARGO_BIN_EXE_stub_sidecar"));
    let bin_dir = stub_path.parent().expect("stub binary has a parent dir");

    let launcher = Arc::new(ProcessSidecarLauncher::new(bin_dir.to_path_buf()));
    let probe = probe_real(ProbeConfig {
        max_attempts: 10,
        initial_delay: Duration::from_millis(50),
    });
    let runtime = LocalRuntime::new(launcher, probe, "/healthz");

    let port = discover_free_port().expect("free port");
    let port_str = port.to_string();
    let status = runtime
        .start("stub_sidecar", &["--port", &port_str], port)
        .await
        .expect("start stub sidecar");

    assert!(
        matches!(status, RuntimeStatus::Ready { port: p } if p == port),
        "expected Ready on /healthz, got {status:?}"
    );

    runtime.stop().await.expect("stop stub sidecar");
}

/// Manual verification (run with `cargo test -- --ignored` on a dev machine
/// that has `.venv` populated from `sidecar/requirements.txt`). Spawns the
/// REAL Python sidecar and asserts the `/healthz` probe drives it to Ready -
/// which also proves `--weights-dir` is accepted, since the sidecar
/// argparse-exits without it before it can serve any endpoint.
#[tokio::test]
#[ignore = "requires the Python sidecar venv; run manually with --ignored"]
async fn real_python_sidecar_reaches_ready_on_healthz() {
    // crates/app-server -> repo root is two levels up.
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root")
        .to_path_buf();
    let python = if cfg!(windows) {
        repo_root.join(".venv").join("Scripts").join("python.exe")
    } else {
        repo_root.join(".venv").join("bin").join("python")
    };
    let app_py = repo_root.join("sidecar").join("app.py");
    assert!(python.is_file(), "venv python not found at {python:?}");
    assert!(app_py.is_file(), "sidecar app.py not found at {app_py:?}");

    let weights = tempfile::tempdir().unwrap();
    let port = discover_free_port().expect("free port");
    let mut child = std::process::Command::new(&python)
        .arg(&app_py)
        .args(["--port", &port.to_string()])
        .arg("--weights-dir")
        .arg(weights.path())
        .spawn()
        .expect("spawn real python sidecar");

    let probe = probe_real(ProbeConfig {
        max_attempts: 40,
        initial_delay: Duration::from_millis(250),
    });
    let url = format!("http://127.0.0.1:{port}/healthz");
    let result = probe(&url).await;
    let _ = child.kill();
    let _ = child.wait();
    result.expect("real python sidecar should answer /healthz");
}
