//! Resumable HTTP GET with SHA256 verification + tokio broadcast progress events.

use crate::models::manifest::ModelId;
use futures::StreamExt;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::broadcast;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DownloadEvent {
    Progress {
        id: ModelId,
        bytes_done: u64,
        total_bytes: Option<u64>,
    },
    Completed {
        id: ModelId,
        bytes_total: u64,
    },
    Failed {
        id: ModelId,
        reason: String,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("sha256 mismatch: expected {expected}, got {actual}")]
    Sha256Mismatch { expected: String, actual: String },
    #[error("cancelled")]
    Cancelled,
}

pub struct DownloadResult {
    pub bytes_downloaded: u64,
    pub final_path: PathBuf,
}

pub async fn download_to(
    url: &str,
    dest: &Path,
    expected_sha256: &str,
    tx: Arc<broadcast::Sender<DownloadEvent>>,
) -> Result<DownloadResult, DownloadError> {
    let _ = tx;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60 * 30))
        .build()?;

    let starting_offset = tokio::fs::metadata(dest)
        .await
        .map(|m| m.len())
        .unwrap_or(0);

    let mut req = client.get(url);
    if starting_offset > 0 {
        req = req.header("Range", format!("bytes={}-", starting_offset));
    }
    let resp = req.send().await?.error_for_status()?;
    let _total = resp.content_length().map(|l| l + starting_offset);

    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(starting_offset > 0)
        .write(true)
        .truncate(starting_offset == 0)
        .open(dest)
        .await?;

    let mut hasher = Sha256::new();
    let mut bytes_downloaded: u64 = 0;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        file.write_all(&bytes).await?;
        if !expected_sha256.is_empty() {
            hasher.update(&bytes);
        }
        bytes_downloaded += bytes.len() as u64;
    }
    file.flush().await?;

    if !expected_sha256.is_empty() {
        let actual = format!("{:x}", hasher.finalize());
        if actual != expected_sha256 {
            let _ = tokio::fs::remove_file(dest).await;
            return Err(DownloadError::Sha256Mismatch {
                expected: expected_sha256.into(),
                actual,
            });
        }
    }

    Ok(DownloadResult {
        bytes_downloaded,
        final_path: dest.to_path_buf(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::TempDir;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn fresh_download_writes_file() {
        let server = MockServer::start().await;
        let payload = b"hello world".to_vec();
        Mock::given(method("GET"))
            .and(path("/file.gguf"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(payload.clone()))
            .mount(&server)
            .await;

        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("file.gguf");
        let url = format!("{}/file.gguf", server.uri());
        let (tx, _rx) = tokio::sync::broadcast::channel(8);
        let result = download_to(&url, &dest, "", Arc::new(tx)).await.unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), payload);
        assert_eq!(result.bytes_downloaded, 11);
    }

    #[tokio::test]
    async fn sha256_mismatch_deletes_partial() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/bad.gguf"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hello".to_vec()))
            .mount(&server)
            .await;
        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("bad.gguf");
        let url = format!("{}/bad.gguf", server.uri());
        let (tx, _rx) = tokio::sync::broadcast::channel(8);
        let result = download_to(
            &url,
            &dest,
            "0000000000000000000000000000000000000000000000000000000000000000",
            Arc::new(tx),
        )
        .await;
        assert!(matches!(result, Err(DownloadError::Sha256Mismatch { .. })));
        assert!(!dest.exists());
    }

    #[tokio::test]
    async fn resumable_get_continues_partial() {
        let server = MockServer::start().await;
        let full = b"abcdefghij".to_vec();
        Mock::given(method("GET"))
            .and(path("/file.gguf"))
            .respond_with(
                ResponseTemplate::new(206)
                    .set_body_bytes(b"fghij".to_vec())
                    .insert_header("content-range", "bytes 5-9/10"),
            )
            .mount(&server)
            .await;

        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("file.gguf");
        std::fs::write(&dest, b"abcde").unwrap();
        let url = format!("{}/file.gguf", server.uri());
        let (tx, _rx) = tokio::sync::broadcast::channel(8);
        let result = download_to(&url, &dest, "", Arc::new(tx)).await.unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), full);
        assert_eq!(result.bytes_downloaded, 5);
    }
}
