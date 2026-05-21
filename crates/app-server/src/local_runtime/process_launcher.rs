//! `std::process`-backed `SidecarLauncher` used by the `dmai-server` process.
//!
//! `dmai-server` runs as its own OS process (spawned by the Tauri shell), so it
//! has no `tauri::AppHandle` and cannot use `TauriSidecarLauncher`. This
//! launcher resolves sidecar binaries relative to `dmai-server`'s own
//! executable directory and spawns them with `std::process::Command`.
//!
//! Capability note (audit finding L5): because `dmai-server` spawns
//! `mistralrs-server` and `dmai-image-sidecar` itself via `std::process`,
//! those two sidecars do NOT need a `shell:allow-spawn` entry in
//! `src-tauri/capabilities/default.json`. Only `dmai-server` (spawned by the
//! Tauri shell) needs that entry, and it already has it. Local Mode requires
//! no capability change.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use app_llm::sidecar_launcher::{SidecarError, SidecarHandle, SidecarLauncher};
use async_trait::async_trait;
use tokio::sync::mpsc;

/// Spawns sidecar child processes by resolving their binary relative to a base
/// directory (the `dmai-server` executable directory in production).
pub struct ProcessSidecarLauncher {
    base_dir: PathBuf,
}

impl ProcessSidecarLauncher {
    /// Construct a launcher rooted at an explicit directory. Used by tests.
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Production constructor: resolve sidecars next to the running executable.
    /// `DMAI_SIDECAR_DIR` overrides the directory (dev convenience). Falls back
    /// to "." when the current exe path cannot be determined.
    pub fn from_current_exe() -> Self {
        if let Ok(dir) = std::env::var("DMAI_SIDECAR_DIR") {
            return Self::new(PathBuf::from(dir));
        }
        let base_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."));
        Self::new(base_dir)
    }
}

/// Resolve a sidecar binary path inside `base_dir`. Tries the bundled name
/// (`<name><ext>` - Tauri strips the target triple when it bundles
/// externalBin sidecars) first, then falls back to the lexicographically-first
/// `<name>-*<ext>` match (the un-bundled `<name>-<target-triple><ext>` layout
/// used under `src-tauri/binaries/`). Sorting makes the choice deterministic
/// when the directory holds more than one matching triple.
fn resolve_binary(base_dir: &Path, name: &str) -> Option<PathBuf> {
    let ext = std::env::consts::EXE_SUFFIX; // ".exe" on Windows, "" elsewhere
    let exact = base_dir.join(format!("{name}{ext}"));
    if exact.is_file() {
        return Some(exact);
    }
    // Fallback: <name>-<target-triple><ext>. Collect and sort so the choice is
    // deterministic if the directory holds more than one matching triple.
    let prefix = format!("{name}-");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(base_dir)
        .ok()?
        .flatten()
        .filter(|entry| {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            file_name.starts_with(&prefix) && file_name.ends_with(ext) && entry.path().is_file()
        })
        .map(|entry| entry.path())
        .collect();
    candidates.sort();
    candidates.into_iter().next()
}

#[async_trait]
impl SidecarLauncher for ProcessSidecarLauncher {
    async fn spawn(&self, name: &str, args: &[&str]) -> Result<SidecarHandle, SidecarError> {
        let path = resolve_binary(&self.base_dir, name).ok_or_else(|| SidecarError::Spawn {
            name: name.into(),
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "sidecar binary `{name}` not found in {}",
                    self.base_dir.display()
                ),
            ),
        })?;

        let mut child = Command::new(&path)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| SidecarError::Spawn {
                name: name.into(),
                source: e,
            })?;

        let pid = child.id();
        let stdout = child.stdout.take();
        let (tx, rx) = mpsc::channel(64);
        if let Some(stdout) = stdout {
            // Blocking line reader on a dedicated OS thread; `blocking_send`
            // bridges into the async channel without touching the runtime.
            std::thread::spawn(move || {
                for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                    if tx.blocking_send(line).is_err() {
                        break;
                    }
                }
            });
        }

        let kill = move || -> Result<(), SidecarError> {
            // Best-effort terminate, then reap so the child is not left as a
            // zombie. `kill` errors only when the process is already gone (the
            // desired end state); `wait` succeeds whether the child was killed
            // or had already exited.
            let _ = child.kill();
            let _ = child.wait();
            Ok(())
        };
        Ok(SidecarHandle::from_parts(pid, rx, kill))
    }
}

#[cfg(test)]
mod tests {
    // The real spawn path (spawn a live process, read stdout, kill+reap) is
    // covered by the integration test added in Task 3 (stub_sidecar).
    use super::*;

    #[test]
    fn resolve_binary_finds_exact_name() {
        let dir = tempfile::tempdir().unwrap();
        let ext = std::env::consts::EXE_SUFFIX;
        let bin = dir.path().join(format!("mistralrs-server{ext}"));
        std::fs::write(&bin, b"x").unwrap();
        assert_eq!(resolve_binary(dir.path(), "mistralrs-server"), Some(bin));
    }

    #[test]
    fn resolve_binary_falls_back_to_target_triple_suffix() {
        let dir = tempfile::tempdir().unwrap();
        let ext = std::env::consts::EXE_SUFFIX;
        let bin = dir
            .path()
            .join(format!("mistralrs-server-x86_64-unknown-linux-gnu{ext}"));
        std::fs::write(&bin, b"x").unwrap();
        assert_eq!(resolve_binary(dir.path(), "mistralrs-server"), Some(bin));
    }

    #[test]
    fn resolve_binary_returns_none_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(resolve_binary(dir.path(), "nonexistent"), None);
    }

    #[tokio::test]
    async fn spawn_missing_binary_reports_spawn_error() {
        let dir = tempfile::tempdir().unwrap();
        let launcher = ProcessSidecarLauncher::new(dir.path().to_path_buf());
        let result = launcher.spawn("does-not-exist", &[]).await;
        assert!(matches!(result, Err(SidecarError::Spawn { .. })));
    }
}
