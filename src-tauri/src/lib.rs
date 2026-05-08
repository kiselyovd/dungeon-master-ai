use std::sync::Mutex;

use tauri::async_runtime::spawn;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};

pub mod sidecar_launcher;

#[derive(Default)]
struct BackendState {
    port: Mutex<Option<u16>>,
    child: Mutex<Option<CommandChild>>,
}

#[tauri::command]
fn backend_port(state: State<'_, BackendState>) -> Option<u16> {
    *state.port.lock().expect("port lock")
}

fn spawn_backend(app: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let sidecar = app
        .shell()
        .sidecar("dmai-server")
        .map_err(|e| format!("sidecar lookup: {e}"))?;

    let (mut rx, child) = sidecar
        .spawn()
        .map_err(|e| format!("spawn dmai-server: {e}"))?;

    let state: State<BackendState> = app.state();
    *state.child.lock().expect("child lock") = Some(child);

    let app_for_task = app.clone();
    spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line_bytes) => {
                    let line = String::from_utf8_lossy(&line_bytes);
                    if let Some(port) = parse_listening_port(&line) {
                        let st: State<BackendState> = app_for_task.state();
                        *st.port.lock().expect("port lock") = Some(port);
                        let _ = app_for_task.emit("backend-ready", port);
                        log::info!("backend listening on port {port}");
                    } else {
                        log::info!("[backend] {line}");
                    }
                }
                CommandEvent::Stderr(line_bytes) => {
                    log::warn!("[backend stderr] {}", String::from_utf8_lossy(&line_bytes));
                }
                CommandEvent::Terminated(status) => {
                    log::error!("backend terminated: {status:?}");
                }
                CommandEvent::Error(err) => log::error!("backend error: {err}"),
                _ => {}
            }
        }
    });

    Ok(())
}

fn parse_listening_port(line: &str) -> Option<u16> {
    line.strip_prefix("APP_SERVER_LISTENING port=")?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(BackendState::default())
        .setup(|app| {
            let handle = app.handle().clone();
            spawn_backend(handle)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![backend_port])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::parse_listening_port;

    #[test]
    fn parses_port_from_listening_line() {
        let line = "APP_SERVER_LISTENING port=51234 host=127.0.0.1";
        assert_eq!(parse_listening_port(line), Some(51234));
    }

    #[test]
    fn ignores_unrelated_lines() {
        assert_eq!(parse_listening_port("hello world"), None);
        assert_eq!(parse_listening_port("APP_SERVER_LISTENING port=abc"), None);
    }
}
