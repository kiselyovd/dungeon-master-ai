//! HTTP API for the dungeon-master-ai backend.

pub mod agent;
pub mod config;
mod control_services;
pub mod db;
pub mod error;
pub mod hf;
pub mod http_services;
pub mod image;
pub mod license;
pub mod local_runtime;
pub mod models;
pub mod paths;
pub mod providers;
pub mod routes;
pub mod secrets;
pub mod state;
pub mod telemetry;
pub mod testing;
pub mod video;

use axum::Router;
use std::sync::Arc;

pub use state::AppState;

pub fn router(state: AppState) -> Router {
    adapter_http::router(http_services::bundle(state))
}

pub async fn router_with_mock_llm() -> Router {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("in-memory db");
    db::init_db(&pool).await.expect("migrate");
    let state = AppState::new(
        Arc::new(app_llm::MockProvider::new(vec![])),
        "mock".into(),
        pool,
    );
    router(state)
}

#[cfg(any(test, feature = "test-support"))]
pub mod test_support {
    use super::*;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use tokio::net::TcpListener;

    pub struct TestServer {
        pub addr: SocketAddr,
        pub state: AppState,
        _handle: tokio::task::JoinHandle<()>,
    }

    impl TestServer {
        pub async fn start() -> Self {
            let pool = sqlx::SqlitePool::connect("sqlite::memory:")
                .await
                .expect("in-memory db");
            crate::db::init_db(&pool).await.expect("migrate");
            Self::start_with(Arc::new(app_llm::MockProvider::new(vec![])), pool).await
        }

        pub async fn start_with(llm: Arc<dyn app_llm::LlmProvider>, db: sqlx::SqlitePool) -> Self {
            let state = AppState::new(llm, "mock".into(), db);
            let app = router(state.clone());
            let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
            let addr = listener.local_addr().expect("addr");
            let handle = tokio::spawn(async move {
                axum::serve(listener, app).await.expect("serve");
            });
            Self {
                addr,
                state,
                _handle: handle,
            }
        }

        pub fn url(&self, path: &str) -> String {
            format!("http://{}{}", self.addr, path)
        }
    }
}
