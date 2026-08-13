use tauri::{Manager, WindowEvent};

mod commands;
mod processes;

use commands::backend::{backend_port, BackendState};
use processes::control::ControlServer;
use processes::RuntimeProcesses;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(BackendState::default())
        .manage(RuntimeProcesses::default())
        .setup(|app| {
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
            processes::backend::spawn_backend(
                app.handle().clone(),
                control.endpoint(),
                control.token(),
                control.executor(),
            )?;
            app.manage(control);
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(
                event,
                WindowEvent::CloseRequested { .. } | WindowEvent::Destroyed
            ) {
                let processes = window
                    .app_handle()
                    .state::<RuntimeProcesses>()
                    .inner()
                    .clone();
                tauri::async_runtime::block_on(processes.stop());
                processes::backend::stop(window.app_handle());
                window.app_handle().state::<ControlServer>().shutdown();
            }
        })
        .invoke_handler(tauri::generate_handler![backend_port])
        .run(tauri::generate_context!())
        .expect("error while running the Tauri application");
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
