use std::collections::{HashSet, VecDeque};
use std::net::TcpListener as StdTcpListener;
use std::sync::{Arc, Mutex};

use app_application::models::local_models::{RuntimeStartRequest, RuntimeStatus};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use tauri::AppHandle;
use tokio::runtime::Handle;
use tokio::sync::oneshot;
use uuid::Uuid;

use super::RuntimeProcesses;

const NONCE_LIMIT: usize = 4096;

#[derive(Clone)]
struct ControlState {
    app: AppHandle,
    processes: RuntimeProcesses,
    executor: Handle,
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
    executor: Handle,
    shutdown: Mutex<Option<oneshot::Sender<()>>>,
}

impl ControlServer {
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn executor(&self) -> &Handle {
        &self.executor
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
    let listener = StdTcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let address = listener.local_addr()?;
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let token_for_state = token.clone();
    let app_for_state = app.clone();
    let processes_for_state = processes.clone();
    let executor = spawn_control_executor(listener, shutdown_rx, move |executor| {
        let state = ControlState {
            app: app_for_state,
            processes: processes_for_state,
            executor: executor.clone(),
            token: Arc::from(token_for_state),
            replay: ReplayProtector::default(),
        };
        Router::new()
            .route("/v1/runtime/start", post(start_runtime))
            .route("/v1/runtime/stop", post(stop_runtime))
            .route("/v1/runtime/status", get(runtime_status))
            .with_state(state)
    })?;
    log::info!(
        "[FIX:runtime-control-executor] process_role=runtime-control lifecycle=ready port={}",
        address.port()
    );
    Ok(ControlServer {
        endpoint: format!("http://127.0.0.1:{}", address.port()),
        token,
        executor,
        shutdown: Mutex::new(Some(shutdown_tx)),
    })
}

fn spawn_control_executor<F>(
    listener: StdTcpListener,
    shutdown_rx: oneshot::Receiver<()>,
    router_factory: F,
) -> std::io::Result<Handle>
where
    F: FnOnce(Handle) -> Router + Send + 'static,
{
    let (executor_tx, executor_rx) = std::sync::mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("dmai-runtime-control".into())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    let _ = executor_tx.send(Err(error));
                    return;
                }
            };
            let executor = runtime.handle().clone();
            let router = router_factory(executor.clone());
            if executor_tx.send(Ok(executor)).is_err() {
                return;
            }
            log::info!(
                "[FIX:runtime-control-executor] process_role=runtime-control lifecycle=executor-started"
            );
            let result = runtime.block_on(async move {
                let listener = tokio::net::TcpListener::from_std(listener)?;
                axum::serve(listener, router)
                    .with_graceful_shutdown(async {
                        let _ = shutdown_rx.await;
                    })
                    .await
            });
            match result {
                Ok(()) => log::info!(
                    "[FIX:runtime-control-executor] process_role=runtime-control lifecycle=executor-stopped"
                ),
                Err(error) => log::error!(
                    "[FIX:runtime-control-executor] process_role=runtime-control lifecycle=failed error={error}"
                ),
            }
        })?;
    executor_rx.recv().map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "runtime control executor exited before startup",
        )
    })?
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
    log::info!(
        "[FIX:runtime-control-executor] operation=start phase=accepted model_id={} image={} video={}",
        request.model_id,
        request.enable_image,
        request.enable_video
    );
    let status =
        state
            .processes
            .start_in_background_on(&state.executor, state.app.clone(), request);
    log::info!("[FIX:runtime-control-executor] operation=start phase=acknowledged");
    Ok(Json(status))
}

async fn stop_runtime(
    State(state): State<ControlState>,
    headers: HeaderMap,
) -> Result<Json<RuntimeStatus>, StatusCode> {
    authorize(&state, &headers)?;
    log::info!("[FIX:runtime-control-executor] operation=stop phase=accepted");
    let status = state.processes.stop().await;
    log::info!("[FIX:runtime-control-executor] operation=stop phase=completed");
    Ok(Json(status))
}

async fn runtime_status(
    State(state): State<ControlState>,
    headers: HeaderMap,
) -> Result<Json<RuntimeStatus>, StatusCode> {
    authorize(&state, &headers)?;
    log::debug!("[FIX:runtime-control-executor] operation=status phase=accepted");
    let status = state.processes.status();
    log::debug!("[FIX:runtime-control-executor] operation=status phase=completed");
    Ok(Json(status))
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::{TcpListener as StdTcpListener, TcpStream};
    use std::time::Duration;

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

    #[test]
    fn control_executor_serves_requests_after_launcher_returns() {
        let listener = StdTcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let router = Router::new().route("/ping", get(|| async { "pong" }));
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        spawn_control_executor(listener, shutdown_rx, |_| router).unwrap();

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut response = String::new();
        while std::time::Instant::now() < deadline {
            if let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_secs(1)) {
                stream
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                stream
                    .write_all(
                        b"GET /ping HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                    )
                    .unwrap();
                let _ = stream.read_to_string(&mut response);
                if response.contains("200 OK") && response.ends_with("pong") {
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = shutdown_tx.send(());

        assert!(response.contains("200 OK"), "response was {response:?}");
        assert!(response.ends_with("pong"), "response was {response:?}");
    }
}
