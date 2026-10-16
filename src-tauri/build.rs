use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../crates/app-bootstrap/src");
    println!("cargo:rerun-if-changed=../crates/app-server/src");
    println!("cargo:rerun-if-changed=../crates/app-application/src");
    println!("cargo:rerun-if-changed=../crates/adapter-http/src");
    println!("cargo:rerun-if-changed=../crates/adapter-sqlite/src");
    println!("cargo:rerun-if-changed=../crates/adapter-llm/src");
    println!("cargo:rerun-if-changed=../crates/adapter-media/src");
    println!("cargo:rerun-if-changed=../crates/adapter-secrets/src");
    println!("cargo:rerun-if-changed=../crates/app-llm/src");
    println!("cargo:rerun-if-changed=../crates/app-domain/src");
    println!("cargo:rerun-if-env-changed=DMAI_SKIP_SIDECAR_BUILD");
    println!("cargo:rerun-if-env-changed=DMAI_CLOUD_ONLY");

    if env::var_os("DMAI_SKIP_SIDECAR_BUILD").is_none() {
        build_backend_sidecar();
    }

    if env::var_os("DMAI_CLOUD_ONLY").is_none() {
        ensure_local_sidecar(
            "mistralrs-server",
            "Build it with scripts/build_mistralrs.sh or scripts/build_mistralrs.ps1.",
        );
        ensure_local_sidecar(
            "dmai-image-sidecar",
            "Build it with sidecar/scripts/build.sh or the prebuild-python-sidecar workflow.",
        );
    }

    tauri_build::build();
}

fn ensure_local_sidecar(bin_basename: &str, build_hint: &str) {
    let target = target_triple();
    let extension = executable_extension(&target);
    let destination = binaries_dir().join(format!("{bin_basename}-{target}{extension}"));

    if is_non_empty_file(&destination) {
        return;
    }

    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    if profile == "release" {
        panic!(
            "release build refused: {bin_basename} is missing or empty at {}. {build_hint} \
             Use `bun run tauri:build:cloud` for a bundle that intentionally excludes local mode.",
            destination.display()
        );
    }

    fs::create_dir_all(binaries_dir()).expect("create src-tauri/binaries");
    if !destination.exists() {
        fs::File::create(&destination).expect("create debug sidecar placeholder");
    }
    println!(
        "cargo:warning={bin_basename} is missing; using an empty DEBUG placeholder at {}",
        destination.display()
    );
}

fn build_backend_sidecar() {
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let target = target_triple();
    let workspace_root = workspace_root();
    let sidecar_target_dir = workspace_root.join("target").join("sidecar-build");

    let mut command = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    command.args([
        "build",
        "-p",
        "app-bootstrap",
        "--bin",
        "app-server",
        "--target",
        &target,
    ]);
    if profile == "release" {
        command.arg("--release");
    }
    command.env("CARGO_TARGET_DIR", &sidecar_target_dir);

    let status = command
        .status()
        .expect("failed to invoke Cargo for the dmai-server sidecar");
    assert!(status.success(), "app-bootstrap sidecar build failed");

    let source = sidecar_target_dir
        .join(&target)
        .join(&profile)
        .join(format!("app-server{}", executable_extension(&target)));
    assert!(
        is_non_empty_file(&source),
        "expected a non-empty app-server binary at {}",
        source.display()
    );

    fs::create_dir_all(binaries_dir()).expect("create src-tauri/binaries");
    let destination = binaries_dir().join(format!(
        "dmai-server-{target}{}",
        executable_extension(&target)
    ));
    fs::copy(&source, &destination).unwrap_or_else(|error| {
        panic!(
            "copy dmai-server from {} to {} failed: {error}",
            source.display(),
            destination.display()
        )
    });
}

fn target_triple() -> String {
    env::var("TARGET").expect("TARGET environment variable is required by Cargo")
}

fn executable_extension(target: &str) -> &'static str {
    if target.contains("windows") {
        ".exe"
    } else {
        ""
    }
}

fn binaries_dir() -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR")).join("binaries")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri must live under the workspace root")
        .to_path_buf()
}

fn is_non_empty_file(path: &Path) -> bool {
    path.metadata()
        .map(|metadata| metadata.is_file() && metadata.len() > 0)
        .unwrap_or(false)
}
