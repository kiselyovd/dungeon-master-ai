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

/// Build the CLI args to launch `mistralrs-server` (v0.8.x) as a GGUF chat
/// server.
///
/// mistralrs uses a subcommand CLI: `--port` is a top-level option and the
/// model is selected via the `gguf` subcommand, which takes the containing
/// directory (`-m`/`--quantized-model-id`) and the filename
/// (`-f`/`--quantized-filename`) SEPARATELY. The previous form
/// `--port <p> --gguf-file <path>` does not parse on 0.8 (there is no
/// `--gguf-file` flag and no subcommand), so the LLM sidecar exited on argv
/// parse and the runtime never came up. (Audit blocker 4, found by running the
/// freshly built binary.)
pub fn mistralrs_gguf_args(port: u16, model_dir: &str, filename: &str) -> Vec<String> {
    vec![
        "--port".into(),
        port.to_string(),
        "gguf".into(),
        "-m".into(),
        model_dir.into(),
        "-f".into(),
        filename.into(),
    ]
}

/// Build the CLI args to launch `mistralrs-server` (master / 0.8.3+) as an
/// auto-loader chat server for a non-GGUF model (safetensors) with in-situ
/// quantization.
///
/// Used for architectures the GGUF loader does not support but the normal model
/// path does - notably Gemma 4 (`google/gemma-4-E2B-it`), whose `gemma4` arch
/// is absent from `GGUFArchitecture` but present in the safetensors loader.
/// `--isq` is a TOP-LEVEL option and must precede the `run` subcommand; `-m`
/// is the HF hub repo id (mistralrs downloads + caches it on first start).
pub fn mistralrs_run_args(port: u16, model_id: &str, isq: &str) -> Vec<String> {
    vec![
        "--port".into(),
        port.to_string(),
        "--isq".into(),
        isq.into(),
        "run".into(),
        "-m".into(),
        model_id.into(),
    ]
}

#[cfg(test)]
mod arg_tests {
    use super::{mistralrs_gguf_args, mistralrs_run_args};

    #[test]
    fn gguf_args_match_mistralrs_080_cli() {
        let args = mistralrs_gguf_args(51234, "/models", "Qwen3.5-4B-Q4_K_M.gguf");
        assert_eq!(
            args,
            vec![
                "--port",
                "51234",
                "gguf",
                "-m",
                "/models",
                "-f",
                "Qwen3.5-4B-Q4_K_M.gguf",
            ]
        );
        // The port must precede the subcommand (top-level option), and there is
        // no `--gguf-file` flag.
        let port_idx = args.iter().position(|a| a == "--port").unwrap();
        let sub_idx = args.iter().position(|a| a == "gguf").unwrap();
        assert!(
            port_idx < sub_idx,
            "--port must come before the gguf subcommand"
        );
        assert!(!args.iter().any(|a| a == "--gguf-file"));
    }

    #[test]
    fn run_args_use_top_level_isq_before_subcommand() {
        let args = mistralrs_run_args(40000, "google/gemma-4-E2B-it", "Q4K");
        assert_eq!(
            args,
            vec![
                "--port",
                "40000",
                "--isq",
                "Q4K",
                "run",
                "-m",
                "google/gemma-4-E2B-it",
            ]
        );
        // --isq is a top-level option: it must come before `run`.
        let isq_idx = args.iter().position(|a| a == "--isq").unwrap();
        let sub_idx = args.iter().position(|a| a == "run").unwrap();
        assert!(isq_idx < sub_idx, "--isq must precede the run subcommand");
    }
}
