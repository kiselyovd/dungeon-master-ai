//! Sub-task #3: retry semantics for ImageProvider wrapper.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use app_server::image::provider::{ImageBytes, ImageError, ImagePrompt, ImageProvider};
use app_server::image::retry::RetryableImageProvider;
use async_trait::async_trait;

struct ScriptedImage {
    calls: AtomicUsize,
    fail_for: usize,
    error_factory: Box<dyn Fn() -> ImageError + Send + Sync>,
}

impl ScriptedImage {
    fn new(fail_for: usize, ef: impl Fn() -> ImageError + Send + Sync + 'static) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            fail_for,
            error_factory: Box::new(ef),
        }
    }
    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl ImageProvider for ScriptedImage {
    async fn generate(&self, _p: ImagePrompt) -> Result<ImageBytes, ImageError> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if n < self.fail_for {
            Err((self.error_factory)())
        } else {
            Ok(ImageBytes {
                data: vec![0, 1, 2],
                mime_type: "image/png".into(),
            })
        }
    }
    fn estimated_seconds(&self) -> u32 {
        10
    }
    fn cost_per_image(&self) -> f32 {
        0.01
    }
}

fn prompt() -> ImagePrompt {
    ImagePrompt::default()
}

#[tokio::test]
async fn no_retry_on_auth() {
    let inner = Arc::new(ScriptedImage::new(10, || ImageError::Auth));
    let wrapped = RetryableImageProvider::new(inner.clone());
    let res = wrapped.generate(prompt()).await;
    assert!(matches!(res, Err(ImageError::Auth)));
    assert_eq!(inner.call_count(), 1);
}

#[tokio::test]
async fn retries_on_network_then_succeeds() {
    let inner = Arc::new(ScriptedImage::new(2, || ImageError::Network("dns".into())));
    let wrapped = RetryableImageProvider::new(inner.clone());
    let res = wrapped.generate(prompt()).await;
    assert!(res.is_ok());
    assert_eq!(inner.call_count(), 3);
}

#[tokio::test]
async fn retries_on_timeout_then_succeeds() {
    let inner = Arc::new(ScriptedImage::new(1, || ImageError::Timeout { secs: 60 }));
    let wrapped = RetryableImageProvider::new(inner.clone());
    let res = wrapped.generate(prompt()).await;
    assert!(res.is_ok());
    assert_eq!(inner.call_count(), 2);
}

#[tokio::test]
async fn exhausted_propagates_last_error() {
    let inner = Arc::new(ScriptedImage::new(100, || {
        ImageError::Network("oops".into())
    }));
    let wrapped = RetryableImageProvider::new(inner.clone());
    let res = wrapped.generate(prompt()).await;
    assert!(matches!(res, Err(ImageError::Network(_))));
    assert_eq!(inner.call_count(), 4);
}
