//! Outbound HTTP adapter to the Python sidecar video SSE endpoint.
//!
//! Same sidecar process as image generation (E.x dispatcher pattern); the
//! sidecar's `ltx-video` backend handles requests. Single port, single GPU
//! mutex - image and video serialise naturally.

use std::time::Duration;

use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use futures::StreamExt;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::video::provider::{
    VideoCapabilities, VideoError, VideoEvent, VideoPrompt, VideoProvider, VideoStream,
};

const REQUEST_TIMEOUT_SECS: u64 = 300;

pub struct LocalVideoSidecarProvider {
    base_url: String,
    client: reqwest::Client,
}

impl LocalVideoSidecarProvider {
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .expect("reqwest client");
        Self {
            base_url: base_url.into(),
            client,
        }
    }
}

#[async_trait]
impl VideoProvider for LocalVideoSidecarProvider {
    async fn generate(&self, prompt: VideoPrompt) -> Result<VideoStream, VideoError> {
        if prompt.init_image_b64.is_some() {
            return Err(VideoError::UnsupportedInitImage);
        }
        let (tx, rx) = mpsc::channel::<VideoEvent>(64);
        let request_id = Uuid::new_v4().to_string();
        let url = format!("{}/video/generate", self.base_url);
        let body = serde_json::json!({
            "contract_version": app_application::ports::media::MEDIA_CONTRACT_VERSION,
            "request_id": request_id,
            "prompt": prompt.text,
            "resolution": prompt.resolution,
            "frame_count": prompt.frame_count,
            "seed": prompt.seed,
        });
        let request = self.client.post(&url).json(&body).send();
        let cancel_client = self.client.clone();
        let cancel_url = format!("{}/video/cancel/{}", self.base_url, request_id);
        tokio::spawn(async move {
            run_video_request(request, cancel_client, cancel_url, tx).await;
        });
        Ok(VideoStream::from_events(rx))
    }

    fn capabilities(&self) -> VideoCapabilities {
        VideoCapabilities {
            duration_range_secs: (3, 8),
            max_resolution: (704, 480),
            // The Python backend currently has no image-conditioning path.
            supports_image_init: false,
            avg_seconds_per_clip: 24,
        }
    }
}

async fn run_video_request(
    request: impl std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
    cancel_client: reqwest::Client,
    cancel_url: String,
    tx: mpsc::Sender<VideoEvent>,
) {
    tokio::pin!(request);
    let resp = tokio::select! {
        _ = tx.closed() => {
            let _ = cancel_client.post(&cancel_url).send().await;
            return;
        }
        result = &mut request => match result {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx
                        .send(VideoEvent::Error {
                            message: e.to_string(),
                        })
                        .await;
                    return;
                }
            }
    };
    if !resp.status().is_success() {
        let _ = tx
            .send(VideoEvent::Error {
                message: format!("sidecar returned {}", resp.status()),
            })
            .await;
        return;
    }
    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();
    loop {
        let chunk = tokio::select! {
            _ = tx.closed() => {
                let _ = cancel_client.post(&cancel_url).send().await;
                return;
            }
            item = stream.next() => item,
        };
        let Some(chunk) = chunk else {
            break;
        };
        let chunk = match chunk {
            Ok(c) => c,
            Err(e) => {
                let _ = tx
                    .send(VideoEvent::Error {
                        message: e.to_string(),
                    })
                    .await;
                return;
            }
        };
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(frame) = take_sse_frame(&mut buffer) {
            if send_frame(&tx, &frame).await.is_err() {
                let _ = cancel_client.post(&cancel_url).send().await;
                return;
            }
        }
    }
    if !buffer.trim().is_empty() {
        let _ = send_frame(&tx, &buffer).await;
    }
}

fn take_sse_frame(buffer: &mut String) -> Option<String> {
    let separators = [("\r\n\r\n", 4), ("\n\n", 2), ("\r\r", 2)];
    let (index, width) = separators
        .iter()
        .filter_map(|(separator, width)| buffer.find(separator).map(|index| (index, *width)))
        .min_by_key(|(index, _)| *index)?;
    let frame = buffer[..index].to_string();
    buffer.drain(..index + width);
    Some(frame)
}

async fn send_frame(tx: &mpsc::Sender<VideoEvent>, frame: &str) -> Result<(), ()> {
    let normalized = frame.replace('\r', "\n");
    let Some(data) = normalized
        .lines()
        .find_map(|line| line.strip_prefix("data: "))
    else {
        return Ok(());
    };
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(data) else {
        return Ok(());
    };
    if let Some(encoded) = value.get("mp4_bytes_b64").and_then(|item| item.as_str()) {
        if let Ok(bytes) = B64.decode(encoded) {
            value["mp4_bytes"] = serde_json::Value::Array(
                bytes
                    .into_iter()
                    .map(|byte| serde_json::Value::from(byte as u64))
                    .collect(),
            );
        }
    }
    if let Ok(event) = serde_json::from_value::<VideoEvent>(value) {
        tx.send(event).await.map_err(|_| ())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::timeout;
    use wiremock::matchers::{method, path, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn capability_does_not_claim_ignored_image_conditioning() {
        let provider = LocalVideoSidecarProvider::new("http://127.0.0.1:1");
        assert!(!provider.capabilities().supports_image_init);
    }

    #[tokio::test]
    async fn unsupported_init_image_is_rejected_before_http() {
        let provider = LocalVideoSidecarProvider::new("http://127.0.0.1:1");
        let result = provider
            .generate(VideoPrompt {
                text: "fog".into(),
                init_image_b64: Some("cG5n".into()),
                ..Default::default()
            })
            .await;
        assert!(matches!(result, Err(VideoError::UnsupportedInitImage)));
    }

    #[tokio::test]
    async fn dropping_stream_posts_cancellation_to_the_sidecar() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/video/generate"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_secs(1))
                    .set_body_string(
                        "event: started\ndata: {\"type\":\"started\",\"estimated_seconds\":24}\n\n",
                    ),
            )
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path_regex(r"^/video/cancel/[0-9a-f-]+$"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        let provider = LocalVideoSidecarProvider::new(server.uri());
        let stream = provider
            .generate(VideoPrompt {
                text: "fog".into(),
                ..Default::default()
            })
            .await
            .unwrap();
        drop(stream);

        timeout(Duration::from_secs(2), async {
            loop {
                if server
                    .received_requests()
                    .await
                    .unwrap()
                    .iter()
                    .any(|request| request.url.path().starts_with("/video/cancel/"))
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("cancellation request arrives");
        server.verify().await;
    }

    #[tokio::test]
    async fn parser_accepts_crlf_and_eof_tail_and_ignores_malformed_frames() {
        let mut buffer =
            "event: started\r\ndata: {\"type\":\"started\",\"estimated_seconds\":24}\r\n\r\n"
                .to_string();
        let frame = take_sse_frame(&mut buffer).expect("CRLF frame");
        let (tx, mut rx) = mpsc::channel(4);
        send_frame(&tx, &frame).await.unwrap();
        assert!(matches!(rx.recv().await, Some(VideoEvent::Started { .. })));

        send_frame(
            &tx,
            "data: {\"type\":\"progress\",\"percent\":0.5,\"eta_seconds\":12}",
        )
        .await
        .unwrap();
        assert!(matches!(rx.recv().await, Some(VideoEvent::Progress { .. })));

        send_frame(&tx, "data: not-json").await.unwrap();
        assert!(rx.try_recv().is_err());
    }
}
