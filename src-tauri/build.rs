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

    // Tauri's build script validates externalBin existence, so the sidecar
    // must already be in place before we hand off control to it.
    tauri_build::build();
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
