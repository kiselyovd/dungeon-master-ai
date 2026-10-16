pub use super::model_runtime::free_loopback_port;

pub fn args(port: u16, weights_dir: &str) -> Vec<String> {
    vec![
        "--port".into(),
        port.to_string(),
        "--weights-dir".into(),
        weights_dir.into(),
    ]
}

pub async fn probe(port: u16) -> bool {
    super::model_runtime::probe_url(&format!("http://127.0.0.1:{port}/healthz")).await
}

#[cfg(test)]
mod tests {
    #[test]
    fn fixed_media_executable_receives_only_typed_arguments() {
        assert_eq!(
            super::args(42000, "models"),
            vec!["--port", "42000", "--weights-dir", "models"]
        );
    }
}
