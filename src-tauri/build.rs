use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../crates/app-server/src");
    println!("cargo:rerun-if-changed=../crates/app-llm/src");
    println!("cargo:rerun-if-changed=../crates/app-domain/src");
    println!("cargo:rerun-if-env-changed=DMAI_SKIP_SIDECAR_BUILD");

    if env::var_os("DMAI_SKIP_SIDECAR_BUILD").is_none() {
        build_sidecar();
    }

    // The mistralrs-server and dmai-image-sidecar sidecars are built by their
    // CI workflows (or the matching build scripts) and staged into
    // src-tauri/binaries/ before `tauri build`. For local `cargo test` /
    // `cargo check` / `tauri dev` without a real binary we drop in an empty
    // placeholder so the tauri-build externalBin resource check is satisfied.
    // A release build with no real binary emits a loud warning instead of
    // silently shipping a broken Local Mode.
    ensure_sidecar_placeholder(
        "mistralrs-server",
        "Build: scripts/build_mistralrs.sh (or prebuild-sidecars workflow).",
    );
    ensure_sidecar_placeholder(
        "dmai-image-sidecar",
        "Build: sidecar/scripts/build.sh (or prebuild-python-sidecar workflow).",
    );

    // Tauri's build script validates externalBin existence, so the sidecar
    // must already be in place before we hand off control to it.
    tauri_build::build();
}

fn ensure_sidecar_placeholder(bin_basename: &str, build_hint: &str) {
    let target_triple = env::var("TARGET").expect("TARGET env var");
    // Extension keyed on the compilation TARGET, not the build host, so a
    // cross-compiled build still produces the correct `.exe` name.
    let ext = if target_triple.contains("windows") { ".exe" } else { "" };
    let bin_name = format!("{bin_basename}-{target_triple}{ext}");
    let dst_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("binaries");
    fs::create_dir_all(&dst_dir).expect("mkdir binaries/");
    let dst = dst_dir.join(&bin_name);

    // A real binary has been staged (CI workflow or a build script): nothing
    // to do, never overwrite it with a placeholder.
    let real_binary_present = dst.metadata().map(|m| m.len() > 0).unwrap_or(false);
    if real_binary_present {
        return;
    }

    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    if profile == "release" {
        // Do not ship a silent lie: a release build with no real sidecar
        // binary must be impossible to miss.
        println!("cargo:warning=====================================================================");
        println!("cargo:warning=RELEASE BUILD with NO real {bin_basename} binary. The matching");
        println!("cargo:warning=Local Mode feature will be non-functional in this build.");
        println!("cargo:warning={build_hint}");
        println!("cargo:warning=====================================================================");
    }

    // Lay down an empty placeholder so tauri-build's externalBin existence
    // check passes. In a release build the warning above already fired.
    if !dst.exists() {
        fs::File::create(&dst).expect("create placeholder sidecar binary");
        if profile != "release" {
            println!(
                "cargo:warning={bin_basename} binary missing - created empty placeholder (dev build)."
            );
        }
    }
}

fn build_sidecar() {
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let workspace_root = workspace_root();
    // Build into a sibling target dir so we don't deadlock against the
    // outer cargo's lock on workspace_root/target.
    let sidecar_target = workspace_root.join("target").join("sidecar-build");

    let mut cmd = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    cmd.args(["build", "-p", "app-server"]);
    if profile == "release" {
        cmd.arg("--release");
    }
    cmd.env("CARGO_TARGET_DIR", &sidecar_target);
    let status = cmd
        .status()
        .expect("failed to run cargo build for app-server");
    assert!(status.success(), "app-server build failed");

    let bin_name = if cfg!(windows) {
        "app-server.exe"
    } else {
        "app-server"
    };
    let src = sidecar_target.join(&profile).join(bin_name);
    assert!(src.exists(), "expected built binary at {src:?}");

    let target_triple = env::var("TARGET").expect("TARGET env var");
    let dst_name = if cfg!(windows) {
        format!("dmai-server-{target_triple}.exe")
    } else {
        format!("dmai-server-{target_triple}")
    };
    let dst_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("binaries");
    fs::create_dir_all(&dst_dir).expect("mkdir binaries/");
    let dst = dst_dir.join(&dst_name);
    fs::copy(&src, &dst).expect("copy app-server binary");
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .unwrap()
        .to_path_buf()
}
