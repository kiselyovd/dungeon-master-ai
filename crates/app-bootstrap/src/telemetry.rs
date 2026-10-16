pub type TelemetryGuard = app_server::telemetry::TelemetryGuard;

pub fn init() -> anyhow::Result<TelemetryGuard> {
    Ok(app_server::telemetry::init_telemetry()?)
}
