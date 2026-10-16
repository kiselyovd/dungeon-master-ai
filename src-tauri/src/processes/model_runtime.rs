use std::net::TcpListener;
use std::time::Duration;

pub fn free_loopback_port() -> std::io::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

pub fn with_runtime_port(mut args: Vec<String>, port: u16) -> Vec<String> {
    if let Some(index) = args.iter().position(|arg| arg == "--port") {
        if let Some(value) = args.get_mut(index + 1) {
            *value = port.to_string();
            return args;
        }
    }
    args.extend(["--port".to_owned(), port.to_string()]);
    args
}

pub async fn probe(port: u16) -> bool {
    probe_url(&format!("http://127.0.0.1:{port}/health")).await
}

pub(super) async fn probe_url(url: &str) -> bool {
    let client = reqwest::Client::new();
    for attempt in 1..=180 {
        if client
            .get(url)
            .timeout(Duration::from_secs(2))
            .send()
            .await
            .is_ok_and(|response| response.status().is_success())
        {
            log::info!("process_role=runtime lifecycle=healthy probe_attempt={attempt}");
            return true;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::with_runtime_port;

    #[test]
    fn replaces_placeholder_port_without_changing_other_arguments() {
        assert_eq!(
            with_runtime_port(vec!["serve".into(), "--port".into(), "0".into()], 41000),
            vec!["serve", "--port", "41000"]
        );
    }
}
