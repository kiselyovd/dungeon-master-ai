use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use adapter_media::image::{
    ImageBytes, ImageError, ImagePrompt, ImageProvider, ImageSource, ResilientImageProvider,
};
use async_trait::async_trait;

#[derive(Clone, Copy)]
enum Outcome {
    Generated,
    Bundled,
    Provider,
    Network,
    Timeout,
    Degraded,
    Auth,
    Cancelled,
}

struct ScriptedProvider {
    outcome: Outcome,
    calls: Arc<AtomicUsize>,
    seconds: u32,
    cost: f32,
}

impl ScriptedProvider {
    fn new(outcome: Outcome, seconds: u32, cost: f32) -> (Arc<Self>, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        (
            Arc::new(Self {
                outcome,
                calls: calls.clone(),
                seconds,
                cost,
            }),
            calls,
        )
    }
}

#[async_trait]
impl ImageProvider for ScriptedProvider {
    async fn generate(&self, _prompt: ImagePrompt) -> Result<ImageBytes, ImageError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        match self.outcome {
            Outcome::Generated => Ok(ImageBytes {
                data: vec![1],
                mime_type: "image/webp".into(),
                source: ImageSource::Generated,
            }),
            Outcome::Bundled => Ok(ImageBytes {
                data: vec![2],
                mime_type: "image/webp".into(),
                source: ImageSource::Bundled {
                    asset_id: "illustration-tavern".into(),
                },
            }),
            Outcome::Provider => Err(ImageError::Provider("safe-provider-code".into())),
            Outcome::Network => Err(ImageError::Network("safe-network-code".into())),
            Outcome::Timeout => Err(ImageError::Timeout { secs: 2 }),
            Outcome::Degraded => Err(ImageError::Degraded { code: "degraded" }),
            Outcome::Auth => Err(ImageError::Auth),
            Outcome::Cancelled => Err(ImageError::Cancelled),
        }
    }

    fn estimated_seconds(&self) -> u32 {
        self.seconds
    }

    fn cost_per_image(&self) -> f32 {
        self.cost
    }
}

fn prompt() -> ImagePrompt {
    ImagePrompt {
        content_prompt: "quiet tavern".into(),
        style_preset: "classic".into(),
        ..ImagePrompt::default()
    }
}

#[tokio::test]
async fn generated_success_skips_fallback_and_keeps_primary_estimates() {
    let (primary, primary_calls) = ScriptedProvider::new(Outcome::Generated, 41, 0.07);
    let (fallback, fallback_calls) = ScriptedProvider::new(Outcome::Bundled, 0, 0.0);
    let provider = ResilientImageProvider::new(primary, fallback);

    let image = provider.generate(prompt()).await.expect("primary image");

    assert_eq!(image.source, ImageSource::Generated);
    assert_eq!(primary_calls.load(Ordering::SeqCst), 1);
    assert_eq!(fallback_calls.load(Ordering::SeqCst), 0);
    assert_eq!(provider.estimated_seconds(), 41);
    assert!((provider.cost_per_image() - 0.07).abs() < f32::EPSILON);
}

#[tokio::test]
async fn transient_and_degraded_failures_fall_back_exactly_once() {
    for outcome in [
        Outcome::Provider,
        Outcome::Network,
        Outcome::Timeout,
        Outcome::Degraded,
    ] {
        let (primary, primary_calls) = ScriptedProvider::new(outcome, 12, 0.01);
        let (fallback, fallback_calls) = ScriptedProvider::new(Outcome::Bundled, 0, 0.0);
        let provider = ResilientImageProvider::new(primary, fallback);

        let image = provider.generate(prompt()).await.expect("bundled fallback");

        assert_eq!(
            image.source,
            ImageSource::Bundled {
                asset_id: "illustration-tavern".into()
            }
        );
        assert_eq!(primary_calls.load(Ordering::SeqCst), 1);
        assert_eq!(fallback_calls.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn auth_and_cancellation_are_never_hidden_by_fallback() {
    for outcome in [Outcome::Auth, Outcome::Cancelled] {
        let (primary, primary_calls) = ScriptedProvider::new(outcome, 12, 0.01);
        let (fallback, fallback_calls) = ScriptedProvider::new(Outcome::Bundled, 0, 0.0);
        let provider = ResilientImageProvider::new(primary, fallback);

        let error = provider.generate(prompt()).await.expect_err("typed error");

        assert!(matches!(error, ImageError::Auth | ImageError::Cancelled));
        assert_eq!(primary_calls.load(Ordering::SeqCst), 1);
        assert_eq!(fallback_calls.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn a_failed_fallback_returns_a_safe_degraded_code() {
    let (primary, _) = ScriptedProvider::new(Outcome::Network, 12, 0.01);
    let (fallback, fallback_calls) = ScriptedProvider::new(Outcome::Provider, 0, 0.0);
    let provider = ResilientImageProvider::new(primary, fallback);

    let error = provider
        .generate(prompt())
        .await
        .expect_err("fallback failure");

    assert!(matches!(
        error,
        ImageError::Degraded {
            code: "bundled_fallback_failed"
        }
    ));
    assert_eq!(fallback_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn bundled_only_provider_remains_zero_cost() {
    let (fallback, _) = ScriptedProvider::new(Outcome::Bundled, 0, 0.0);
    assert_eq!(fallback.estimated_seconds(), 0);
    assert_eq!(fallback.cost_per_image(), 0.0);
}
