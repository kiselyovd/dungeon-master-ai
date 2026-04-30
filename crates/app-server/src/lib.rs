//! HTTP API for the dungeon-master-ai backend.

pub mod config;
pub mod db;
pub mod error;
pub mod routes;
pub mod state;

use axum::Router;
use axum::routing::{get, post};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

pub use state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(routes::health::health))
        .route("/chat", post(routes::chat::chat))
        .route("/providers", get(routes::settings::get_providers))
        .route("/settings", post(routes::settings::post_settings))
        .route("/combat/start", post(routes::combat::post_combat_start))
        .route("/combat/action", post(routes::combat::post_combat_action))
        .route("/combat/end", post(routes::combat::post_combat_end))
        .with_state(state)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers(Any)
                .allow_methods(Any),
        )
        .layer(TraceLayer::new_for_http())
}

pub async fn router_with_mock_llm() -> Router {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("in-memory db");
    db::init_db(&pool).await.expect("migrate");
    let state = AppState::new(Arc::new(app_llm::MockProvider::new(vec![])), "mock".into(), pool);
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

        pub async fn start_with(
            llm: Arc<dyn app_llm::LlmProvider>,
            db: sqlx::SqlitePool,
        ) -> Self {
            let state = AppState::new(llm, "mock".into(), db);
            let app = router(state);
            let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
            let addr = listener.local_addr().expect("addr");
            let handle = tokio::spawn(async move {
                axum::serve(listener, app).await.expect("serve");
            });
            Self {
                addr,
                _handle: handle,
            }
        }

        pub fn url(&self, path: &str) -> String {
            format!("http://{}{}", self.addr, path)
        }
    }
}
