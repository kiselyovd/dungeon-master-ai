use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};

use app_application::models::local_models::{RuntimeStartRequest, RuntimeStatus};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use tauri::AppHandle;
use tokio::sync::oneshot;
use uuid::Uuid;

use super::RuntimeProcesses;

const NONCE_LIMIT: usize = 4096;

#[derive(Clone)]
struct ControlState {
    app: AppHandle,
    processes: RuntimeProcesses,
    token: Arc<str>,
    replay: ReplayProtector,
}

#[derive(Clone, Default)]
struct ReplayProtector {
    inner: Arc<Mutex<(HashSet<String>, VecDeque<String>)>>,
}

impl ReplayProtector {
    fn accept(&self, nonce: &str) -> bool {
        if nonce.len() < 16 || nonce.len() > 128 {
            return false;
        }
        let mut guard = self.inner.lock().expect("control replay lock poisoned");
        if guard.0.contains(nonce) {
            return false;
        }
        guard.0.insert(nonce.to_owned());
        guard.1.push_back(nonce.to_owned());
        if guard.1.len() > NONCE_LIMIT {
            if let Some(expired) = guard.1.pop_front() {
                guard.0.remove(&expired);
            }
        }
        true
    }
}

pub struct ControlServer {
    endpoint: String,
    token: String,
    shutdown: Mutex<Option<oneshot::Sender<()>>>,
}

impl ControlServer {
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn shutdown(&self) {
        if let Some(sender) = self
            .shutdown
            .lock()
            .expect("control shutdown lock poisoned")
            .take()
        {
            let _ = sender.send(());
        }
    }
}

pub async fn start(
    app: AppHandle,
    processes: RuntimeProcesses,
) -> Result<ControlServer, Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let state = ControlState {
        app,
        processes,
        token: Arc::from(token.clone()),
        replay: ReplayProtector::default(),
    };
    let router = Router::new()
        .route("/v1/runtime/start", post(start_runtime))
        .route("/v1/runtime/stop", post(stop_runtime))
        .route("/v1/runtime/status", get(runtime_status))
        .with_state(state);
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    tauri::async_runtime::spawn(async move {
        let result = axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await;
        if let Err(error) = result {
            log::error!("process_role=runtime-control lifecycle=failed error={error}");
        }
    });
    log::info!(
        "process_role=runtime-control lifecycle=ready port={}",
        address.port()
    );
    Ok(ControlServer {
        endpoint: format!("http://127.0.0.1:{}", address.port()),
        token,
        shutdown: Mutex::new(Some(shutdown_tx)),
    })
}

fn authorize(state: &ControlState, headers: &HeaderMap) -> Result<(), StatusCode> {
    if !bearer_matches(&state.token, headers) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let nonce = headers
        .get("x-dmai-nonce")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if !state.replay.accept(nonce) {
        return Err(StatusCode::CONFLICT);
    }
    Ok(())
}

fn bearer_matches(token: &str, headers: &HeaderMap) -> bool {
    let expected = format!("Bearer {token}");
    let Some(actual) = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    constant_time_eq(actual.as_bytes(), expected.as_bytes())
}

fn constant_time_eq(actual: &[u8], expected: &[u8]) -> bool {
    if actual.len() != expected.len() {
        return false;
    }
    actual
        .iter()
        .zip(expected)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

async fn start_runtime(
    State(state): State<ControlState>,
    headers: HeaderMap,
    Json(request): Json<RuntimeStartRequest>,
) -> Result<Json<RuntimeStatus>, StatusCode> {
    authorize(&state, &headers)?;
    Ok(Json(
        state
            .processes
            .start_in_background(state.app.clone(), request),
    ))
}

async fn stop_runtime(
    State(state): State<ControlState>,
    headers: HeaderMap,
) -> Result<Json<RuntimeStatus>, StatusCode> {
    authorize(&state, &headers)?;
    Ok(Json(state.processes.stop().await))
}

async fn runtime_status(
    State(state): State<ControlState>,
    headers: HeaderMap,
) -> Result<Json<RuntimeStatus>, StatusCode> {
    authorize(&state, &headers)?;
    Ok(Json(state.processes.status()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_token_and_replayed_nonce() {
        let replay = ReplayProtector::default();
        assert!(replay.accept("nonce-0000000001"));
        assert!(!replay.accept("nonce-0000000001"));
        assert!(!replay.accept("short"));
        let mut request_headers = HeaderMap::new();
        request_headers.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer wrong".parse().unwrap(),
        );
        assert!(!bearer_matches("expected", &request_headers));
        request_headers.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer expected".parse().unwrap(),
        );
        assert!(bearer_matches("expected", &request_headers));
    }
}
