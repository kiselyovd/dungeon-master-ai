//! Retry wrapper for LlmProvider.
//!
//! Wraps any `LlmProvider` and re-invokes `stream_chat` with exponential
//! backoff on retryable errors. Default policy: 3 retries (4 total attempts)
//! with 250ms base, 2.0 factor, jitter enabled.
//!
//! Streaming semantics: only the initial connection is retried. If the stream
//! drops mid-flight after the first chunk is yielded, the error is surfaced
//! to the caller. Replaying would duplicate output the user has already seen.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use backon::{ExponentialBuilder, Retryable};

use crate::provider::{Capabilities, ChatRequest, ChunkStream, LlmError, LlmProvider};

/// Decide whether a given `LlmError` should trigger a retry.
///
/// Retryable: rate-limit (HTTP 429), network errors, transient provider errors
/// surfaced as `LlmError::Provider` with no clear schema or validation marker
/// (treated as retryable on the conservative side - cloud providers sometimes
/// wrap 5xx in generic strings).
///
/// NOT retryable: auth failures, invalid response (schema or validation), and
/// any other terminal condition.
pub fn is_retryable(err: &LlmError) -> bool {
    match err {
        LlmError::RateLimit => true,
        LlmError::Network(_) => true,
        // Transient provider strings (5xx wrapped). Conservative match - we
        // explicitly exclude InvalidResponse / AuthFailure which would lead to
        // wasted retries.
        LlmError::Provider(_) => true,
        LlmError::AuthFailure => false,
        LlmError::InvalidResponse(_) => false,
    }
}

/// Default exponential-backoff policy used by `RetryableProvider::new`.
///
/// - Base delay: 250ms
/// - Factor: 2.0
/// - Jitter: enabled (backon's default uniform jitter)
/// - Max retries: 3 (total attempts = 4)
///
/// Resulting backoff sequence (approximate, before jitter): 250ms, 500ms, 1000ms.
pub fn default_policy() -> ExponentialBuilder {
    ExponentialBuilder::default()
        .with_min_delay(Duration::from_millis(250))
        .with_factor(2.0)
        .with_jitter()
        .with_max_times(3)
}

pub struct RetryableProvider {
    inner: Arc<dyn LlmProvider>,
    policy: ExponentialBuilder,
}

impl RetryableProvider {
    pub fn new(inner: Arc<dyn LlmProvider>) -> Self {
        Self {
            inner,
            policy: default_policy(),
        }
    }

    pub fn with_policy(inner: Arc<dyn LlmProvider>, policy: ExponentialBuilder) -> Self {
        Self { inner, policy }
    }
}

#[async_trait]
impl LlmProvider for RetryableProvider {
    async fn stream_chat(&self, req: ChatRequest) -> Result<ChunkStream, LlmError> {
        let inner = self.inner.clone();
        let op = move || {
            let inner = inner.clone();
            let req = req.clone();
            async move { inner.stream_chat(req).await }
        };
        op.retry(self.policy)
            .when(|e: &LlmError| is_retryable(e))
            .await
    }

    fn name(&self) -> &'static str {
        self.inner.name()
    }

    fn capabilities_for_model(&self, model_id: &str) -> Capabilities {
        self.inner.capabilities_for_model(model_id)
    }

    fn active_model(&self) -> &str {
        self.inner.active_model()
    }
}
