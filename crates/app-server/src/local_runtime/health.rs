//! HTTP health probe with bounded retry and exponential backoff.
//!
//! Used by `LocalRuntime` after `Command::spawn` succeeds: we cannot trust
//! that a child process is actually serving HTTP just because the syscall
//! returned. The probe pings the sidecar's `/health` until it answers 2xx
//! or the budget is exhausted.

use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct ProbeConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
}

#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error("health probe exhausted after {attempts} attempts")]
    ExhaustedAttempts { attempts: u32 },
}

pub async fn probe_until_ready(url: &str, cfg: ProbeConfig) -> Result<(), ProbeError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("client build");
    let mut delay = cfg.initial_delay;
    for _ in 0..cfg.max_attempts {
        match client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => return Ok(()),
            _ => {
                tokio::time::sleep(delay).await;
                delay *= 2;
            }
        }
    }
    Err(ProbeError::ExhaustedAttempts {
        attempts: cfg.max_attempts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn ready_after_health_responds_200() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let url = format!("{}/health", server.uri());
        let result = probe_until_ready(
            &url,
            ProbeConfig {
                max_attempts: 3,
                initial_delay: Duration::from_millis(10),
            },
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn fails_after_all_retries() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;

        let url = format!("{}/health", server.uri());
        let result = probe_until_ready(
            &url,
            ProbeConfig {
                max_attempts: 3,
                initial_delay: Duration::from_millis(5),
            },
        )
        .await;
        assert!(matches!(
            result,
            Err(ProbeError::ExhaustedAttempts { attempts: 3 })
        ));
    }
}
