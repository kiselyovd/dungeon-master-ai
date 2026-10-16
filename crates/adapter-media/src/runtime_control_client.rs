use app_application::models::local_models::{RuntimeStartRequest, RuntimeStatus};
use app_application::ports::runtime::{RuntimeControl, RuntimeError};
use async_trait::async_trait;
use reqwest::header::{HeaderValue, AUTHORIZATION};
use std::time::Duration;
use uuid::Uuid;

/// Authenticated client for the Tauri-owned loopback process controller.
/// The bearer token is retained only in memory and is never exposed through
/// Debug, tracing, or error values.
pub struct LoopbackRuntimeControl {
    client: reqwest::Client,
    endpoint: String,
    authorization: HeaderValue,
}

impl LoopbackRuntimeControl {
    pub fn new(endpoint: impl Into<String>, token: impl AsRef<str>) -> Result<Self, RuntimeError> {
        let authorization =
            HeaderValue::from_str(&format!("Bearer {}", token.as_ref())).map_err(|_| {
                RuntimeError::Operation {
                    operation: "configure",
                    code: "control_credential_invalid",
                }
            })?;
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .map_err(|_| RuntimeError::Operation {
                    operation: "configure",
                    code: "control_client_invalid",
                })?,
            endpoint: endpoint.into().trim_end_matches('/').to_owned(),
            authorization,
        })
    }

    pub fn from_env() -> Result<Option<Self>, RuntimeError> {
        let Ok(endpoint) = std::env::var("DMAI_RUNTIME_CONTROL_URL") else {
            return Ok(None);
        };
        let token =
            std::env::var("DMAI_RUNTIME_CONTROL_TOKEN").map_err(|_| RuntimeError::Operation {
                operation: "configure",
                code: "control_credential_missing",
            })?;
        Self::new(endpoint, token).map(Some)
    }

    async fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&RuntimeStartRequest>,
        operation: &'static str,
    ) -> Result<RuntimeStatus, RuntimeError> {
        let mut request = self
            .client
            .request(method, format!("{}{path}", self.endpoint))
            .header(AUTHORIZATION, self.authorization.clone())
            .header("x-dmai-nonce", Uuid::new_v4().to_string());
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request.send().await.map_err(|_| RuntimeError::Operation {
            operation,
            code: "control_unavailable",
        })?;
        if !response.status().is_success() {
            return Err(RuntimeError::Operation {
                operation,
                code: "control_rejected",
            });
        }
        response.json().await.map_err(|_| RuntimeError::Operation {
            operation,
            code: "control_response_invalid",
        })
    }
}

#[async_trait]
impl RuntimeControl for LoopbackRuntimeControl {
    async fn start(&self, request: RuntimeStartRequest) -> Result<RuntimeStatus, RuntimeError> {
        self.request(
            reqwest::Method::POST,
            "/v1/runtime/start",
            Some(&request),
            "start",
        )
        .await
    }

    async fn stop(&self) -> Result<RuntimeStatus, RuntimeError> {
        self.request(reqwest::Method::POST, "/v1/runtime/stop", None, "stop")
            .await
    }

    async fn status(&self) -> Result<RuntimeStatus, RuntimeError> {
        self.request(reqwest::Method::GET, "/v1/runtime/status", None, "status")
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use wiremock::matchers::{header_exists, method, path};
    use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

    #[derive(Clone)]
    struct CaptureNonces(Arc<Mutex<Vec<String>>>);

    impl Respond for CaptureNonces {
        fn respond(&self, request: &Request) -> ResponseTemplate {
            let nonce = request
                .headers
                .get("x-dmai-nonce")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned)
                .unwrap_or_default();
            self.0.lock().unwrap().push(nonce);
            ResponseTemplate::new(200).set_body_json(RuntimeStatus::default())
        }
    }

    #[tokio::test]
    async fn sends_auth_and_a_fresh_nonce_for_each_request() {
        let server = MockServer::start().await;
        let nonces = Arc::new(Mutex::new(Vec::new()));
        Mock::given(method("GET"))
            .and(path("/v1/runtime/status"))
            .and(header_exists("authorization"))
            .and(header_exists("x-dmai-nonce"))
            .respond_with(CaptureNonces(nonces.clone()))
            .expect(2)
            .mount(&server)
            .await;
        let control = LoopbackRuntimeControl::new(server.uri(), "secret").unwrap();
        control.status().await.unwrap();
        control.status().await.unwrap();
        let values = nonces.lock().unwrap();
        assert_eq!(values.len(), 2);
        assert!(!values[0].is_empty());
        assert_ne!(values[0], values[1]);
    }
}
