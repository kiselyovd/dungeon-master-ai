//! Public test helpers for integration tests in `crates/app-server/tests/`.
//! The module is `pub` (not `pub(crate)`) so external test binaries can use
//! `app_server::testing::new_test_state()` without duplicating fixture setup.

use std::sync::Arc;

use app_llm::{LlmProvider, MockProvider};
use sqlx::SqlitePool;

use crate::AppState;

/// Build a fresh `AppState` backed by an in-memory SQLite and a `MockProvider`
/// chat slot. Suitable for integration tests that exercise `/settings/v2`,
/// /agent/turn, /combat, etc. Tests that need a real provider should swap
/// after construction via `state.swap_registry(...)`.
pub async fn new_test_state() -> AppState {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("connect sqlite memory");
    let llm: Arc<dyn LlmProvider> = Arc::new(MockProvider::new(vec![]));
    AppState::new(llm, "mock".into(), pool)
}
