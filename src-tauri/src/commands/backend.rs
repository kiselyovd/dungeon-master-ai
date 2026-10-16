use std::sync::Mutex;

use tauri::State;
use tauri_plugin_shell::process::CommandChild;

#[derive(Default)]
pub struct BackendState {
    pub(crate) port: Mutex<Option<u16>>,
    pub(crate) child: Mutex<Option<CommandChild>>,
}

#[tauri::command]
pub fn backend_port(state: State<'_, BackendState>) -> Option<u16> {
    *state.port.lock().expect("backend port lock poisoned")
}
