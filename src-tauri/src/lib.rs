use tauri::{Manager, RunEvent, WindowEvent};

mod commands;
mod processes;

use commands::backend::{backend_port, BackendState};
use commands::desktop::{desktop_reset_geometry, DesktopState};
use commands::rooms::{
    cancel_room_stream, host_room, join_room, postpone_guest_room, recover_guest_room,
    recover_host_room, resume_room, room_operation, saved_guest_rooms, stop_host_room, stream_room,
    NativeRoomState,
};
use processes::control::ControlServer;
use processes::lifecycle::{desktop_confirm_exit, AppLifecycle};
use processes::RuntimeProcesses;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init();
    let process_job = processes::owned_job::OwnedProcessJob::new()
        .expect("desktop process ownership must be initialized before startup");
    let context = tauri::generate_context!();
    let mut startup_guard = Some(
        processes::startup_guard::StartupGuard::acquire(&context.config().identifier)
            .expect("desktop plugin startup must be serialized"),
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            commands::invocations::second_launch(app, args);
        }))
        // No configured CLI schemes: the bounded native parser owns cold/warm
        // dispatch, avoiding the plugin's raw-URL state and debug logging.
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(BackendState::default())
        .manage(NativeRoomState::default())
        .manage(RuntimeProcesses::default())
        .manage(DesktopState::default())
        .manage(AppLifecycle::default())
        .manage(commands::invocations::InvocationBroker::default())
        .manage(commands::invitations::InvitationInbox::default())
        .manage(commands::notifications::NotificationState::default())
        .manage(process_job)
        .setup(|app| {
            log::info!("native launch kind=first");
            commands::notifications::resume(app.handle());
            let args: Vec<String> = std::env::args().collect();
            if args.len() > 1 {
                commands::invocations::handle_launch(app.handle(), args, "cold");
            }
            commands::desktop::restore(app.handle());
            let salt_path = app
                .path()
                .app_local_data_dir()
                .map_err(|error| format!("resolve app_local_data_dir: {error}"))?
                .join("salt.txt");
            app.handle()
                .plugin(tauri_plugin_stronghold::Builder::with_argon2(&salt_path).build())?;

            let processes = app.state::<RuntimeProcesses>().inner().clone();
            let control = tauri::async_runtime::block_on(processes::control::start(
                app.handle().clone(),
                processes,
            ))?;
            app.manage(control);
            let control = app.state::<ControlServer>();
            if let Err(error) = processes::backend::spawn_backend(
                app.handle().clone(),
                control.endpoint(),
                control.token(),
                control.executor(),
            ) {
                processes::lifecycle::shutdown(app.handle());
                return Err(error);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(
                event,
                WindowEvent::Focused(true) | WindowEvent::ScaleFactorChanged { .. }
            ) {
                commands::desktop::recover_offscreen(window);
            }
            if matches!(
                event,
                WindowEvent::Moved(_)
                    | WindowEvent::Resized(_)
                    | WindowEvent::ScaleFactorChanged { .. }
            ) {
                commands::desktop::capture(window, false);
            }
            if matches!(event, WindowEvent::CloseRequested { .. }) {
                commands::desktop::capture(window, true);
            }
            if matches!(event, WindowEvent::Destroyed) {
                log::debug!("window={} lifecycle=destroyed", window.label());
                if processes::lifecycle::closes_application(window.label()) {
                    processes::lifecycle::request_exit(window.app_handle());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            backend_port,
            desktop_reset_geometry,
            desktop_confirm_exit,
            commands::invitations::desktop_parse_invitation,
            commands::invitations::desktop_take_invitation,
            commands::invitations::desktop_create_invitation,
            commands::invitations::desktop_register_invitation_protocol,
            commands::notifications::desktop_notification_preferences,
            commands::notifications::desktop_set_notifications,
            commands::notifications::desktop_notification_context,
            commands::notifications::desktop_offer_notification,
            commands::notifications::desktop_notification_qa_counts,
            commands::notifications::desktop_notification_qa_shortcut,
            commands::notifications::desktop_notification_qa_enroll,
            host_room,
            recover_host_room,
            stop_host_room,
            join_room,
            resume_room,
            recover_guest_room,
            saved_guest_rooms,
            postpone_guest_room,
            room_operation,
            stream_room,
            cancel_room_stream
        ])
        .build(context)
        .expect("error while building the Tauri application")
        .run(move |app, event| {
            if matches!(event, RunEvent::Ready) {
                // The single-instance message window and event loop now exist.
                drop(startup_guard.take());
                commands::invocations::deliver_focus(app);
            }
            if matches!(event, RunEvent::Exit) {
                commands::notifications::shutdown();
                processes::lifecycle::shutdown(app);
            }
        });
}

#[cfg(test)]
mod packaging_tests {
    use serde_json::Value;

    fn external_bins(source: &str) -> Vec<String> {
        serde_json::from_str::<Value>(source).unwrap()["bundle"]["externalBin"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_owned())
            .collect()
    }

    #[test]
    fn local_and_cloud_manifests_declare_exact_sidecar_sets() {
        assert_eq!(
            external_bins(include_str!("../tauri.conf.json")),
            vec![
                "binaries/dmai-server",
                "binaries/mistralrs-server",
                "binaries/dmai-image-sidecar",
            ]
        );
        assert_eq!(
            external_bins(include_str!("../tauri.cloud.conf.json")),
            vec!["binaries/dmai-server"]
        );
    }

    #[test]
    fn capability_allows_only_declared_fixed_sidecars() {
        let source = include_str!("../capabilities/default.json");
        for name in ["dmai-server", "mistralrs-server", "dmai-image-sidecar"] {
            assert!(source.contains(name));
        }
        assert!(!source.contains("shell:allow-execute"));
    }

    #[test]
    fn backend_sidecar_rebuild_tracks_every_in_process_workspace_layer() {
        let build_script = include_str!("../build.rs");
        for source in [
            "../crates/app-bootstrap/src",
            "../crates/app-server/src",
            "../crates/app-application/src",
            "../crates/adapter-llm/src",
        ] {
            assert!(
                build_script.contains(&format!("cargo:rerun-if-changed={source}")),
                "missing sidecar rebuild trigger for {source}"
            );
        }
    }
}
