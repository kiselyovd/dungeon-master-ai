//! Telemetry init: structured JSON logs to `app_data_dir/logs/dmai.log.YYYY-MM-DD`
//! plus stderr passthrough. Used by `main.rs`; tests use `init_telemetry_at`
//! with a tempdir override.
//!
//! Out of scope (deferred): retention policy, Prometheus exposition, metrics
//! aggregation, external error tracking.

use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::paths;

/// Held by `main()` for the full app lifetime; on `Drop` the non-blocking
/// appender flushes its buffered writes.
pub struct TelemetryGuard {
    _worker_guard: WorkerGuard,
}

/// Production init: writes to `app_data_dir/logs/`.
pub fn init_telemetry() -> std::io::Result<TelemetryGuard> {
    let log_dir = paths::app_data_dir().join("logs");
    init_telemetry_at(&log_dir)
}

/// Test/override init: writes to the given directory.
///
/// Splits on a `try_init` so multiple test binaries in the same process can
/// tolerate a second invocation (re-init becomes a no-op when a subscriber is
/// already set). The worker guard is still returned and still flushes on drop.
pub fn init_telemetry_at(log_dir: &Path) -> std::io::Result<TelemetryGuard> {
    std::fs::create_dir_all(log_dir)?;

    let appender = tracing_appender::rolling::daily(log_dir, "dmai.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(appender);

    let file_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_writer(non_blocking)
        .with_target(true)
        .with_thread_ids(true)
        .with_current_span(true)
        .with_span_list(false);

    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_target(false);

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,dmai=debug,sqlx=warn"));

    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(stderr_layer)
        .try_init();

    Ok(TelemetryGuard {
        _worker_guard: guard,
    })
}
