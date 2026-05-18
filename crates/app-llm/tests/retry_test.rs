//! Sub-task #3: retry semantics for LlmProvider wrapper.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use app_llm::{
    Capabilities, ChatChunk, ChatRequest, ChunkStream, LlmError, LlmProvider, RetryableProvider,
};
use futures::stream;

/// A mock provider that returns a configurable LlmError on the Nth call,
/// then returns a single-chunk success after that. Counts invocations.
struct ScriptedProvider {
    calls: AtomicUsize,
    fail_for: usize,
    error_factory: Box<dyn Fn() -> LlmError + Send + Sync>,
}

impl ScriptedProvider {
    fn new(fail_for: usize, error_factory: impl Fn() -> LlmError + Send + Sync + 'static) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            fail_for,
            error_factory: Box::new(error_factory),
        }
    }
    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl LlmProvider for ScriptedProvider {
    async fn stream_chat(&self, _req: ChatRequest) -> Result<ChunkStream, LlmError> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if n < self.fail_for {
            Err((self.error_factory)())
        } else {
            let chunks = vec![Ok(ChatChunk::TextDelta { text: "ok".into() })];
            Ok(Box::pin(stream::iter(chunks)))
        }
    }
    fn name(&self) -> &'static str {
        "scripted"
    }
    fn capabilities_for_model(&self, _: &str) -> Capabilities {
        Capabilities::default()
    }
    fn active_model(&self) -> &str {
        "scripted-1"
    }
}

fn req() -> ChatRequest {
    ChatRequest {
        messages: vec![app_llm::ChatMessage::user_text("hi")],
        model: "scripted-1".into(),
        max_tokens: None,
        temperature: None,
        tools: vec![],
        system_prompt: None,
        reasoning: None,
    }
}

#[tokio::test]
async fn no_retry_on_auth_failure() {
    let inner = Arc::new(ScriptedProvider::new(10, || LlmError::AuthFailure));
    let wrapped = RetryableProvider::new(inner.clone());
    let res = wrapped.stream_chat(req()).await;
    assert!(matches!(res, Err(LlmError::AuthFailure)));
    assert_eq!(inner.call_count(), 1, "auth failure must not retry");
}

#[tokio::test]
async fn no_retry_on_invalid_response() {
    let inner = Arc::new(ScriptedProvider::new(10, || {
        LlmError::InvalidResponse("schema mismatch".into())
    }));
    let wrapped = RetryableProvider::new(inner.clone());
    let res = wrapped.stream_chat(req()).await;
    assert!(matches!(res, Err(LlmError::InvalidResponse(_))));
    assert_eq!(inner.call_count(), 1);
}

#[tokio::test]
async fn retries_on_rate_limit_then_succeeds() {
    let inner = Arc::new(ScriptedProvider::new(2, || LlmError::RateLimit));
    let wrapped = RetryableProvider::new(inner.clone());
    let res = wrapped.stream_chat(req()).await;
    assert!(res.is_ok(), "should succeed after retries");
    assert_eq!(inner.call_count(), 3, "two failures + one success = 3 calls");
}

#[tokio::test]
async fn retries_on_network_then_succeeds() {
    let inner = Arc::new(ScriptedProvider::new(2, || {
        LlmError::Network("connect refused".into())
    }));
    let wrapped = RetryableProvider::new(inner.clone());
    let res = wrapped.stream_chat(req()).await;
    assert!(res.is_ok());
    assert_eq!(inner.call_count(), 3);
}

#[tokio::test]
async fn exhausted_retries_propagates_last_error() {
    let inner = Arc::new(ScriptedProvider::new(100, || LlmError::RateLimit));
    let wrapped = RetryableProvider::new(inner.clone());
    let res = wrapped.stream_chat(req()).await;
    assert!(matches!(res, Err(LlmError::RateLimit)));
    // Default policy = 3 retries -> 4 total attempts (initial + 3 retries).
    assert_eq!(inner.call_count(), 4);
}
