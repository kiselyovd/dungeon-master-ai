//! Production `SidecarLauncher` impl backed by Tauri v2's `tauri-plugin-shell`.
//! `MockSidecarLauncher` (in `app_llm`) covers unit tests; this lives in
//! `src-tauri` because it depends on a live `tauri::AppHandle`.
//!
//! The launcher spawns the externalBin asset registered in `tauri.conf.json`
//! (e.g. `binaries/mistralrs-server-{target}.exe`), pipes stdout lines into an
//! mpsc channel that the returned `SidecarHandle` exposes, and packages a
//! `kill` closure that owns the underlying `CommandChild`.
//!
//! The plan in `2026-05-08-m4-local-mode-and-packaging.md` calls
//! `spawn(args, name)` and a `Box<dyn FnOnce>` constructor for `from_parts`;
//! the actual A.1 contract is `spawn(name, args)` and an `impl FnOnce` arg, so
//! this file matches the live trait, not the plan's prose.

use app_llm::sidecar_launcher::{SidecarError, SidecarHandle, SidecarLauncher};
use async_trait::async_trait;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::AppHandle;
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;
use tokio::sync::mpsc;

pub struct TauriSidecarLauncher {
    app: AppHandle,
}

impl TauriSidecarLauncher {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[async_trait]
impl SidecarLauncher for TauriSidecarLauncher {
    async fn spawn(&self, name: &str, args: &[&str]) -> Result<SidecarHandle, SidecarError> {
        let cmd = self
            .app
            .shell()
            .sidecar(name)
            .map_err(|e| SidecarError::Spawn {
                name: name.into(),
                source: io::Error::new(io::ErrorKind::NotFound, e.to_string()),
            })?;

        let (mut rx, child) = cmd.args(args).spawn().map_err(|e| SidecarError::Spawn {
            name: name.into(),
            source: io::Error::other(e.to_string()),
        })?;

        let pid = child.pid();
        let (line_tx, line_rx) = mpsc::channel(64);

        // tauri-plugin-shell's CommandChild has no `try_wait`, so liveness is
        // tracked off the event stream: the child is alive until a `Terminated`
        // event arrives (or the event channel closes).
        let alive = Arc::new(AtomicBool::new(true));
        let alive_for_events = alive.clone();
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Stdout(line_bytes) => {
                        let line = String::from_utf8_lossy(&line_bytes).to_string();
                        if line_tx.send(line).await.is_err() {
                            break;
                        }
                    }
                    CommandEvent::Terminated(_) => break,
                    _ => {}
                }
            }
            alive_for_events.store(false, Ordering::SeqCst);
        });

        let alive_for_liveness = alive.clone();
        let is_alive = move || alive_for_liveness.load(Ordering::SeqCst);

        let name_owned = name.to_string();
        let kill_fn = move || -> Result<(), SidecarError> {
            child.kill().map_err(|e| SidecarError::Spawn {
                name: name_owned,
                source: io::Error::other(e.to_string()),
            })
        };

        Ok(SidecarHandle::from_parts(pid, line_rx, is_alive, kill_fn))
    }
}
