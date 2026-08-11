use tauri::async_runtime::spawn;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

use crate::commands::backend::BackendState;

#[derive(Debug, PartialEq, Eq)]
enum BackendObservation<'a> {
    Stdout(&'a str),
    Terminated(String),
    Error(String),
}

#[derive(Debug, PartialEq, Eq)]
enum BackendEffect {
    Ready(u16),
    LogLine(String),
    Exited(String),
}

#[derive(Default)]
struct BackendLifecycle {
    ready_port: Option<u16>,
    has_exited: bool,
}

impl BackendLifecycle {
    fn observe(&mut self, observation: BackendObservation<'_>) -> Option<BackendEffect> {
        if self.has_exited {
            return None;
        }
        match observation {
            BackendObservation::Stdout(line) => match parse_listening_port(line) {
                Some(port) if self.ready_port != Some(port) => {
                    self.ready_port = Some(port);
                    Some(BackendEffect::Ready(port))
                }
                Some(_) => None,
                None => Some(BackendEffect::LogLine(line.trim_end().to_owned())),
            },
            BackendObservation::Terminated(reason) | BackendObservation::Error(reason) => {
                self.has_exited = true;
                self.ready_port = None;
                Some(BackendEffect::Exited(reason))
            }
        }
    }
}

fn clear_backend_state(app: &AppHandle) {
    let state: State<BackendState> = app.state();
    *state.port.lock().expect("backend port lock poisoned") = None;
    *state.child.lock().expect("backend child lock poisoned") = None;
}

pub fn stop(app: &AppHandle) {
    let state: State<BackendState> = app.state();
    *state.port.lock().expect("backend port lock poisoned") = None;
    let child = state
        .child
        .lock()
        .expect("backend child lock poisoned")
        .take();
    if let Some(child) = child {
        if let Err(error) = child.kill() {
            log::debug!("dmai-server stop raced with process exit: {error}");
        }
    }
}

pub fn spawn_backend(
    app: AppHandle,
    control_endpoint: &str,
    control_token: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    stop(&app);
    let sidecar = app
        .shell()
        .sidecar("dmai-server")
        .map_err(|error| format!("dmai-server sidecar lookup failed: {error}"))?
        .env("DMAI_RUNTIME_CONTROL_URL", control_endpoint)
        .env("DMAI_RUNTIME_CONTROL_TOKEN", control_token);
    let (mut events, child) = sidecar
        .spawn()
        .map_err(|error| format!("dmai-server spawn failed: {error}"))?;
    log::info!("process_role=backend pid={} lifecycle=spawned", child.pid());
    let state: State<BackendState> = app.state();
    *state.child.lock().expect("backend child lock poisoned") = Some(child);

    let app_for_events = app.clone();
    spawn(async move {
        let mut lifecycle = BackendLifecycle::default();
        while let Some(event) = events.recv().await {
            match event {
                CommandEvent::Stdout(bytes) => {
                    let line = String::from_utf8_lossy(&bytes);
                    match lifecycle.observe(BackendObservation::Stdout(&line)) {
                        Some(BackendEffect::Ready(port)) => {
                            let state: State<BackendState> = app_for_events.state();
                            *state.port.lock().expect("backend port lock poisoned") = Some(port);
                            let _ = app_for_events.emit("backend-ready", port);
                            log::info!("process_role=backend lifecycle=ready port={port}");
                        }
                        Some(BackendEffect::LogLine(line)) => log::info!("[dmai-server] {line}"),
                        Some(BackendEffect::Exited(_)) | None => {}
                    }
                }
                CommandEvent::Stderr(bytes) => {
                    log::warn!("process_role=backend stderr_bytes={}", bytes.len());
                }
                CommandEvent::Terminated(status) => {
                    let reason = format!("exit_code={:?}", status.code);
                    if let Some(BackendEffect::Exited(reason)) =
                        lifecycle.observe(BackendObservation::Terminated(reason))
                    {
                        clear_backend_state(&app_for_events);
                        let _ = app_for_events.emit("backend-exited", reason);
                    }
                    break;
                }
                CommandEvent::Error(error) => {
                    let reason = error.to_string();
                    if let Some(BackendEffect::Exited(reason)) =
                        lifecycle.observe(BackendObservation::Error(reason))
                    {
                        clear_backend_state(&app_for_events);
                        let _ = app_for_events.emit("backend-exited", reason);
                    }
                    break;
                }
                _ => {}
            }
        }
        clear_backend_state(&app_for_events);
    });
    Ok(())
}

fn parse_listening_port(line: &str) -> Option<u16> {
    line.trim()
        .strip_prefix("APP_SERVER_LISTENING port=")?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_port_from_listening_line() {
        assert_eq!(
            parse_listening_port("APP_SERVER_LISTENING port=51234 host=127.0.0.1"),
            Some(51234)
        );
    }

    #[test]
    fn lifecycle_emits_ready_and_exit_once() {
        let mut lifecycle = BackendLifecycle::default();
        let line = "APP_SERVER_LISTENING port=51234 host=127.0.0.1";
        assert_eq!(
            lifecycle.observe(BackendObservation::Stdout(line)),
            Some(BackendEffect::Ready(51234))
        );
        assert_eq!(lifecycle.observe(BackendObservation::Stdout(line)), None);
        assert_eq!(
            lifecycle.observe(BackendObservation::Terminated("code=1".into())),
            Some(BackendEffect::Exited("code=1".into()))
        );
        assert_eq!(lifecycle.observe(BackendObservation::Stdout(line)), None);
    }
}
