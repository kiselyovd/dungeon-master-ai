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
//!
//! Dev fallback: when the `dmai-image-sidecar` PyInstaller bundle (~5 GB,
//! torch/diffusers) is absent and `DMAI_IMAGE_SIDECAR_DEV` points at a repo
//! root with a populated `.venv`, the launcher runs `python sidecar/app.py`
//! from source instead - so Local Mode image work does not need a bundle
//! rebuild per change. Production builds leave the var unset and require the
//! staged binary.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use app_llm::sidecar_launcher::{SidecarError, SidecarHandle, SidecarLauncher};
use async_trait::async_trait;
use tokio::sync::mpsc;

/// Resolved dev paths for running the Python image sidecar from source.
/// Present only in dev builds; `None` in production.
struct PythonSidecarDev {
    /// Path to the repo virtualenv Python interpreter.
    python: PathBuf,
    /// Path to `sidecar/app.py`.
    app_py: PathBuf,
}

/// Detect a dev Python sidecar setup from `DMAI_IMAGE_SIDECAR_DEV`, which a
/// developer sets to the repo root before `tauri dev`. Returns `None` when the
/// var is unset (production) or the venv interpreter / `app.py` do not exist.
fn detect_python_sidecar_dev() -> Option<PythonSidecarDev> {
    let root = PathBuf::from(std::env::var("DMAI_IMAGE_SIDECAR_DEV").ok()?);
    let python = if cfg!(windows) {
        root.join(".venv").join("Scripts").join("python.exe")
    } else {
        root.join(".venv").join("bin").join("python")
    };
    let app_py = root.join("sidecar").join("app.py");
    if python.is_file() && app_py.is_file() {
        Some(PythonSidecarDev { python, app_py })
    } else {
        tracing::warn!(
            root = %root.display(),
            python_ok = python.is_file(),
            app_py_ok = app_py.is_file(),
            "DMAI_IMAGE_SIDECAR_DEV is set but the venv python or sidecar/app.py is missing; dev fallback disabled"
        );
        None
    }
}

/// Spawns sidecar child processes by resolving their binary relative to a base
/// directory (the `dmai-server` executable directory in production), with a
/// dev fallback that runs the Python image sidecar straight from the repo venv.
pub struct ProcessSidecarLauncher {
    base_dir: PathBuf,
    /// Dev-only: when the `dmai-image-sidecar` binary is absent, run the Python
    /// sidecar from source instead. `None` in production.
    python_sidecar_dev: Option<PythonSidecarDev>,
}

impl ProcessSidecarLauncher {
    /// Construct a launcher rooted at an explicit directory, with no dev
    /// fallback. Used by tests.
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            base_dir,
            python_sidecar_dev: None,
        }
    }

    /// Production constructor: resolve sidecars next to the running executable.
    /// `DMAI_SIDECAR_DIR` overrides the directory (dev convenience). Falls back
    /// to "." when the current exe path cannot be determined. The Python
    /// sidecar dev fallback is enabled when `DMAI_IMAGE_SIDECAR_DEV` points at
    /// a repo root with a populated `.venv`.
    pub fn from_current_exe() -> Self {
        let base_dir = if let Ok(dir) = std::env::var("DMAI_SIDECAR_DIR") {
            PathBuf::from(dir)
        } else {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(Path::to_path_buf))
                .unwrap_or_else(|| PathBuf::from("."))
        };
        Self {
            base_dir,
            python_sidecar_dev: detect_python_sidecar_dev(),
        }
    }
}

/// Resolve a sidecar binary path inside `base_dir`. Tries the bundled name
/// (`<name><ext>` - Tauri strips the target triple when it bundles
/// externalBin sidecars) first, then falls back to the lexicographically-first
/// `<name>-*<ext>` match (the un-bundled `<name>-<target-triple><ext>` layout
/// used under `src-tauri/binaries/`). Sorting makes the choice deterministic
/// when the directory holds more than one matching triple.
/// A real, runnable binary: a regular file with non-zero length. `build.rs`
/// drops a 0-byte placeholder for unbuilt sidecars so tauri's externalBin check
/// passes; treating that as runnable makes `Command::spawn` fail with a cryptic
/// "%1 is not a valid Win32 application" (os error 193) instead of falling back
/// to the dev path / reporting a clear "not built" error. (Audit finding via
/// live runtime test.)
fn is_real_binary(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| m.is_file() && m.len() > 0)
        .unwrap_or(false)
}

fn resolve_binary(base_dir: &Path, name: &str) -> Option<PathBuf> {
    let ext = std::env::consts::EXE_SUFFIX; // ".exe" on Windows, "" elsewhere
    let exact = base_dir.join(format!("{name}{ext}"));
    if is_real_binary(&exact) {
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
            file_name.starts_with(&prefix)
                && file_name.ends_with(ext)
                && is_real_binary(&entry.path())
        })
        .map(|entry| entry.path())
        .collect();
    candidates.sort();
    candidates.into_iter().next()
}

/// Decide which program to run, and with which args, for a sidecar spawn.
/// Resolves the staged/bundled binary first; if it is absent and `name` is the
/// Python image sidecar, falls back to the dev venv interpreter running
/// `sidecar/app.py` directly. Pure (does not spawn) so the decision is
/// unit-testable.
fn resolve_spawn_target(
    base_dir: &Path,
    python_sidecar_dev: Option<&PythonSidecarDev>,
    name: &str,
    args: &[&str],
) -> Result<(PathBuf, Vec<String>), SidecarError> {
    if let Some(path) = resolve_binary(base_dir, name) {
        return Ok((path, args.iter().map(|s| s.to_string()).collect()));
    }
    // Dev fallback: only the Python image sidecar can run from source.
    if name == "dmai-image-sidecar" {
        if let Some(dev) = python_sidecar_dev {
            let mut full = Vec::with_capacity(args.len() + 1);
            full.push(dev.app_py.to_string_lossy().into_owned());
            full.extend(args.iter().map(|s| s.to_string()));
            return Ok((dev.python.clone(), full));
        }
    }
    Err(SidecarError::Spawn {
        name: name.into(),
        source: std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "sidecar binary `{name}` not found in {}",
                base_dir.display()
            ),
        ),
    })
}

#[async_trait]
impl SidecarLauncher for ProcessSidecarLauncher {
    async fn spawn(&self, name: &str, args: &[&str]) -> Result<SidecarHandle, SidecarError> {
        let (program, full_args) =
            resolve_spawn_target(&self.base_dir, self.python_sidecar_dev.as_ref(), name, args)?;

        let mut child = Command::new(&program)
            .args(&full_args)
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

    #[test]
    fn resolve_binary_ignores_zero_byte_placeholder() {
        // build.rs lays down a 0-byte placeholder for unbuilt sidecars; it must
        // not be treated as runnable (otherwise spawn fails with os error 193).
        let dir = tempfile::tempdir().unwrap();
        let ext = std::env::consts::EXE_SUFFIX;
        std::fs::write(dir.path().join(format!("dmai-image-sidecar{ext}")), b"").unwrap();
        assert_eq!(resolve_binary(dir.path(), "dmai-image-sidecar"), None);
    }

    #[test]
    fn resolve_spawn_target_falls_back_to_dev_when_binary_is_placeholder() {
        // With only a 0-byte placeholder present, the image sidecar must fall
        // back to the dev venv python running sidecar/app.py.
        let dir = tempfile::tempdir().unwrap();
        let ext = std::env::consts::EXE_SUFFIX;
        std::fs::write(dir.path().join(format!("dmai-image-sidecar{ext}")), b"").unwrap();
        let dev = PythonSidecarDev {
            python: PathBuf::from("/venv/python"),
            app_py: PathBuf::from("/repo/sidecar/app.py"),
        };
        let (program, args) = resolve_spawn_target(
            dir.path(),
            Some(&dev),
            "dmai-image-sidecar",
            &["--port", "1"],
        )
        .unwrap();
        assert_eq!(program, PathBuf::from("/venv/python"));
        assert_eq!(args[0], "/repo/sidecar/app.py");
    }

    #[tokio::test]
    async fn spawn_missing_binary_reports_spawn_error() {
        let dir = tempfile::tempdir().unwrap();
        let launcher = ProcessSidecarLauncher::new(dir.path().to_path_buf());
        let result = launcher.spawn("does-not-exist", &[]).await;
        assert!(matches!(result, Err(SidecarError::Spawn { .. })));
    }

    #[test]
    fn resolve_spawn_target_uses_binary_when_present() {
        let dir = tempfile::tempdir().unwrap();
        let ext = std::env::consts::EXE_SUFFIX;
        let bin = dir.path().join(format!("dmai-image-sidecar{ext}"));
        std::fs::write(&bin, b"x").unwrap();
        let (program, args) =
            resolve_spawn_target(dir.path(), None, "dmai-image-sidecar", &["--port", "1"]).unwrap();
        assert_eq!(program, bin);
        assert_eq!(args, vec!["--port".to_string(), "1".to_string()]);
    }

    #[test]
    fn resolve_spawn_target_falls_back_to_python_venv_for_image_sidecar() {
        let dir = tempfile::tempdir().unwrap(); // empty - no staged binary
        let dev = PythonSidecarDev {
            python: PathBuf::from("/venv/bin/python"),
            app_py: PathBuf::from("/repo/sidecar/app.py"),
        };
        let (program, args) = resolve_spawn_target(
            dir.path(),
            Some(&dev),
            "dmai-image-sidecar",
            &["--port", "1", "--weights-dir", "/w"],
        )
        .unwrap();
        assert_eq!(program, PathBuf::from("/venv/bin/python"));
        assert_eq!(
            args,
            vec![
                "/repo/sidecar/app.py".to_string(),
                "--port".to_string(),
                "1".to_string(),
                "--weights-dir".to_string(),
                "/w".to_string(),
            ]
        );
    }

    #[test]
    fn resolve_spawn_target_errors_for_image_sidecar_without_dev_config() {
        let dir = tempfile::tempdir().unwrap();
        let result = resolve_spawn_target(dir.path(), None, "dmai-image-sidecar", &[]);
        assert!(matches!(result, Err(SidecarError::Spawn { .. })));
    }

    #[test]
    fn resolve_spawn_target_does_not_fall_back_for_mistralrs() {
        // The venv fallback is image-sidecar-only: a missing mistralrs-server
        // binary still errors even when a dev config is present.
        let dir = tempfile::tempdir().unwrap();
        let dev = PythonSidecarDev {
            python: PathBuf::from("/venv/bin/python"),
            app_py: PathBuf::from("/repo/sidecar/app.py"),
        };
        let result = resolve_spawn_target(dir.path(), Some(&dev), "mistralrs-server", &[]);
        assert!(matches!(result, Err(SidecarError::Spawn { .. })));
    }
}
