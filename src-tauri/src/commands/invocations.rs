//! Native launch delivery never changes the active campaign or an operation.
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

#[derive(Default)]
pub struct InvocationBroker {
    pending_focus: Mutex<bool>,
}

/// argv[0] is the executable; arbitrary flags/files/URIs are not commands.
fn is_focus_invocation(args: &[String]) -> bool {
    args.len() == 1 && !args[0].is_empty() && args[0].len() <= 32_768
}

pub fn second_launch(app: &AppHandle, args: Vec<String>) {
    handle_launch(app, args, "second");
}

pub fn handle_launch(app: &AppHandle, args: Vec<String>, origin: &'static str) {
    let focus = is_focus_invocation(&args);
    let invitation = args.len() == 2 && crate::commands::invitations::queue(app, &args[1]).is_ok();
    if !focus && !invitation {
        log::warn!("native invocation result=rejected");
        return;
    }
    let broker = app.state::<InvocationBroker>();
    if let Ok(mut pending) = broker.pending_focus.lock() {
        *pending = true;
    } else {
        log::error!("native invocation result=broker_unavailable");
        return;
    }
    log::info!("native launch kind={origin} invocation=accepted");
    deliver_focus(app);
}

/// Called again on Ready because plugin callbacks can precede window creation.
pub fn deliver_focus(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let broker = app.state::<InvocationBroker>();
    let Ok(mut pending) = broker.pending_focus.lock() else {
        log::error!("native invocation result=broker_unavailable");
        return;
    };
    if !*pending {
        return;
    }
    if window
        .show()
        .and_then(|()| window.unminimize())
        .and_then(|()| window.set_focus())
        .is_err()
    {
        log::error!("native invocation result=focus_failed");
        return;
    }
    *pending = false;
    log::info!("native invocation result=focused window=main");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executable_only_launch_is_focus() {
        assert!(is_focus_invocation(&["C:\\QA\\app.exe".into()]));
    }

    #[test]
    fn unknown_arguments_and_unbounded_inputs_are_not_dispatched() {
        for args in [
            vec![],
            vec![String::new()],
            vec!["a".repeat(32_769)],
            vec!["app".into(), "--anything".into()],
            vec!["app".into(), "https://untrusted.example".into()],
        ] {
            assert!(!is_focus_invocation(&args));
        }
    }
}
