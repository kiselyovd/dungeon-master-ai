//! Lifecycle management for sidecar processes (mistralrs-server, Python SDXL).
pub mod health;
pub mod port;
pub mod process_launcher;
pub mod registry;
pub mod runtime;

pub use health::{probe_until_ready, ProbeConfig, ProbeError};
pub use process_launcher::ProcessSidecarLauncher;
pub use registry::{GpuOwner, RegistrySnapshot, RuntimeRegistry};
pub use runtime::{
    probe_always_fail, probe_always_ok, probe_real, LocalRuntime, ProbeFn, RuntimeStatus,
};

/// Build the CLI args to launch the `mistralrs` binary (mistralrs-cli, 0.8.3+)
/// as a GGUF chat server via its `serve` subcommand.
///
/// The new CLI replaces the deprecated `mistralrs-server`. `serve` is the
/// subcommand; the server bind (`--port`) and the model selector are flags
/// AFTER it. A GGUF model is `--format Gguf` plus the containing directory
/// (`-m`/`--model-id`, which accepts a local path) and the filename
/// (`-f`/`--quantized-file`). The deprecated binary mangled Gemma tool-call
/// output into raw text (`<|tool_call>` template tokens leaked); the new CLI's
/// v0.8.2 tool-calling fixes are why we migrated.
pub fn mistralrs_gguf_args(port: u16, model_dir: &str, filename: &str) -> Vec<String> {
    vec![
        "serve".into(),
        "--host".into(),
        "127.0.0.1".into(),
        "--port".into(),
        port.to_string(),
        "-m".into(),
        model_dir.into(),
        "--format".into(),
        "gguf".into(),
        "-f".into(),
        filename.into(),
    ]
}

/// Build the CLI args to launch the `mistralrs` binary (mistralrs-cli, 0.8.3+)
/// as an auto-loader chat server for a non-GGUF model (safetensors) with
/// in-situ quantization, via its `serve` subcommand.
///
/// Used for architectures the GGUF loader does not support but the normal model
/// path does - notably Gemma 4 (`google/gemma-4-E2B-it`), whose `gemma4` arch
/// is absent from `GGUFArchitecture` but present in the safetensors loader.
/// `--isq` and `-m` are model flags flattened into `serve`; the default format
/// is `Plain` (safetensors), so no `--format` is needed. `-m` is the HF hub
/// repo id (mistralrs downloads + caches it on first start).
pub fn mistralrs_run_args(port: u16, model_id: &str, isq: &str) -> Vec<String> {
    vec![
        "serve".into(),
        "--host".into(),
        "127.0.0.1".into(),
        "--port".into(),
        port.to_string(),
        "--isq".into(),
        isq.into(),
        "-m".into(),
        model_id.into(),
    ]
}

#[cfg(test)]
mod arg_tests {
    use super::{mistralrs_gguf_args, mistralrs_run_args};

    #[test]
    fn gguf_args_match_mistralrs_cli_serve() {
        let args = mistralrs_gguf_args(51234, "/models", "Qwen3.5-4B-Q4_K_M.gguf");
        assert_eq!(
            args,
            vec![
                "serve",
                "--host",
                "127.0.0.1",
                "--port",
                "51234",
                "-m",
                "/models",
                "--format",
                "gguf",
                "-f",
                "Qwen3.5-4B-Q4_K_M.gguf",
            ]
        );
        // `serve` is the leading subcommand; the GGUF format selector and file
        // flag come after it.
        assert_eq!(args.first().map(String::as_str), Some("serve"));
        assert!(args.iter().any(|a| a == "--format"));
        assert!(args.iter().any(|a| a == "gguf"));
    }

    #[test]
    fn run_args_use_serve_subcommand_with_isq() {
        let args = mistralrs_run_args(40000, "google/gemma-4-E2B-it", "q4k");
        assert_eq!(
            args,
            vec![
                "serve",
                "--host",
                "127.0.0.1",
                "--port",
                "40000",
                "--isq",
                "q4k",
                "-m",
                "google/gemma-4-E2B-it",
            ]
        );
        // `serve` leads; --isq is a model flag flattened into it (no `--format`
        // for the default Plain/safetensors path).
        assert_eq!(args.first().map(String::as_str), Some("serve"));
        assert!(!args.iter().any(|a| a == "--format"));
    }
}
