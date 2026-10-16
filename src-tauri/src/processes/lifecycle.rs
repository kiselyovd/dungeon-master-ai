use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager, WebviewWindow};

#[derive(Default)]
pub struct AppLifecycle {
    exiting: AtomicBool,
    stopped: AtomicBool,
}
impl AppLifecycle {
    fn approve_exit(&self) -> bool {
        !self.exiting.swap(true, Ordering::SeqCst)
    }
    fn begin_shutdown(&self) -> bool {
        !self.stopped.swap(true, Ordering::SeqCst)
    }
}

pub fn request_exit(app: &AppHandle) {
    if app.state::<AppLifecycle>().approve_exit() {
        log::info!("application lifecycle=exit-requested");
        app.exit(0);
    }
}

pub fn shutdown(app: &AppHandle) {
    let Some(lifecycle) = app.try_state::<AppLifecycle>() else {
        return;
    };
    if !lifecycle.begin_shutdown() {
        return;
    }
    // Stop accepting runtime requests before disposing their children.
    if let Some(control) = app.try_state::<super::control::ControlServer>() {
        control.shutdown();
    }
    if let Some(rooms) = app.try_state::<crate::commands::rooms::NativeRoomState>() {
        rooms.shutdown_streams();
    }
    if let Some(runtime) = app.try_state::<super::RuntimeProcesses>() {
        tauri::async_runtime::block_on(runtime.shutdown());
    }
    if app
        .try_state::<crate::commands::backend::BackendState>()
        .is_some()
    {
        super::backend::stop(app);
    }
    log::info!("application lifecycle=stopped");
}

#[tauri::command]
pub fn desktop_confirm_exit(window: WebviewWindow) -> Result<(), &'static str> {
    if window.label() != "main" {
        return Err("desktop_window_scope_denied");
    }
    crate::commands::desktop::capture(&window.as_ref().window(), true);
    request_exit(window.app_handle());
    Ok(())
}

pub fn closes_application(label: &str) -> bool {
    label == "main"
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn auxiliary_disposal_is_not_application_exit() {
        for label in ["hero", "journal", "map", "aux-qa-1", ""] {
            assert!(!closes_application(label));
        }
        assert!(closes_application("main"));
    }
    #[test]
    fn repeated_exit_and_shutdown_are_exactly_once() {
        let lifecycle = AppLifecycle::default();
        assert!(lifecycle.approve_exit());
        assert!(!lifecycle.approve_exit());
        assert!(lifecycle.begin_shutdown());
        assert!(!lifecycle.begin_shutdown());
    }
    #[test]
    fn concurrent_shutdown_has_one_owner() {
        let lifecycle = std::sync::Arc::new(AppLifecycle::default());
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let state = lifecycle.clone();
                std::thread::spawn(move || state.begin_shutdown())
            })
            .collect();
        assert_eq!(
            handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .filter(|owner| *owner)
                .count(),
            1
        );
    }
}
