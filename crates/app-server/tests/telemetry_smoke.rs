//! Sub-task #4: init_telemetry creates the log directory, returns a guard,
//! and writes a JSON line on `tracing::info!`.

use app_server::telemetry::init_telemetry_at;
use tempfile::TempDir;

#[test]
fn creates_log_file_and_writes_json_event() {
    let dir = TempDir::new().expect("tempdir");
    let guard = init_telemetry_at(dir.path()).expect("init telemetry");

    tracing::info!(target: "dmai_smoke", marker = "alpha", "telemetry smoke event");

    // Drop guard to flush.
    drop(guard);

    let entries: Vec<_> = std::fs::read_dir(dir.path())
        .expect("read log dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("dmai.log"))
                .unwrap_or(false)
        })
        .collect();
    assert!(
        !entries.is_empty(),
        "expected at least one dmai.log.* file in {:?}",
        dir.path()
    );

    let contents = std::fs::read_to_string(&entries[0]).expect("read log");
    assert!(
        contents.contains("telemetry smoke event"),
        "log should contain the event message; got: {}",
        contents
    );
    assert!(
        contents.contains("\"marker\":\"alpha\"") || contents.contains("\"marker\": \"alpha\""),
        "log should be JSON with the marker field; got: {}",
        contents
    );
}
