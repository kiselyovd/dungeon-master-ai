//! Resumable HTTP GET with SHA256 verification + tokio broadcast progress events.

use crate::models::manifest::ModelId;
use futures::StreamExt;
use serde::Deserialize;
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

#[derive(Debug)]
pub struct DiffusersResult {
    pub files_downloaded: u32,
    pub bytes_total: u64,
}

#[derive(Debug, Deserialize)]
struct HfTreeEntry {
    #[serde(rename = "type")]
    kind: String,
    path: String,
}

/// Endpoints used to talk to a HuggingFace-Hub-compatible host.
/// `Default` points at https://huggingface.co; tests inject a wiremock origin.
#[derive(Debug, Clone)]
pub struct HfEndpoints {
    pub api_base: String,
    pub resolve_base: String,
}

impl Default for HfEndpoints {
    fn default() -> Self {
        Self {
            api_base: "https://huggingface.co".into(),
            resolve_base: "https://huggingface.co".into(),
        }
    }
}

/// Walks a HuggingFace repo via the Hub tree API and downloads every file.
///
/// Uses `{api_base}/api/models/{repo}/tree/{rev}?recursive=true` to list all
/// blobs (LFS pointers and inline files alike), then resolves each via
/// `{resolve_base}/{repo}/resolve/{rev}/{path}` which auto-redirects LFS
/// objects to the CDN. Replaces an earlier model_index.json string-array
/// walker that only worked for mocked schemas.
pub async fn download_diffusers_repo(
    endpoints: &HfEndpoints,
    hf_repo: &str,
    revision: &str,
    dest_dir: &Path,
    tx: Arc<broadcast::Sender<DownloadEvent>>,
) -> Result<DiffusersResult, DownloadError> {
    let tree_url = format!(
        "{}/api/models/{hf_repo}/tree/{revision}?recursive=true",
        endpoints.api_base
    );
    let client = reqwest::Client::new();
    let entries: Vec<HfTreeEntry> = client
        .get(&tree_url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let files: Vec<String> = entries
        .into_iter()
        .filter(|e| e.kind == "file")
        .map(|e| e.path)
        .collect();

    let mut bytes_total = 0u64;
    let mut files_downloaded = 0u32;
    let semaphore = Arc::new(tokio::sync::Semaphore::new(4));
    let mut handles = Vec::new();
    let resolve_base = endpoints.resolve_base.clone();
    for relpath in files {
        let url = format!("{resolve_base}/{hf_repo}/resolve/{revision}/{relpath}");
        let dest = dest_dir.join(&relpath);
        let permit = semaphore.clone();
        let tx = tx.clone();
        handles.push(tokio::spawn(async move {
            let _p = permit.acquire_owned().await.expect("semaphore");
            download_to(&url, &dest, "", tx).await
        }));
    }
    for h in handles {
        let r = h
            .await
            .map_err(|e| DownloadError::Io(std::io::Error::other(e.to_string())))??;
        bytes_total += r.bytes_downloaded;
        files_downloaded += 1;
    }
    Ok(DiffusersResult {
        files_downloaded,
        bytes_total,
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
    async fn diffusers_walk_downloads_files_via_hf_tree_api() {
        let server = MockServer::start().await;
        // Real HF tree API returns an array of {type, path, size, oid, lfs?}.
        // `directory` entries are returned alongside files when `recursive=true`
        // and must be skipped.
        Mock::given(method("GET"))
            .and(path("/api/models/stabilityai/sdxl-test/tree/main"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"[
                    {"type":"directory","path":"unet","size":0,"oid":"d1"},
                    {"type":"file","path":"model_index.json","size":42,"oid":"a1"},
                    {"type":"file","path":"unet/diffusion_pytorch_model.safetensors","size":10,"oid":"b1","lfs":{"oid":"sha256:xxx","size":10,"pointerSize":120}},
                    {"type":"file","path":"vae/diffusion_pytorch_model.safetensors","size":10,"oid":"c1"}
                ]"#,
            ))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(
                "/stabilityai/sdxl-test/resolve/main/model_index.json",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"{}".to_vec()))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(
                "/stabilityai/sdxl-test/resolve/main/unet/diffusion_pytorch_model.safetensors",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"unet-bytes".to_vec()))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(
                "/stabilityai/sdxl-test/resolve/main/vae/diffusion_pytorch_model.safetensors",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"vae-bytes".to_vec()))
            .mount(&server)
            .await;

        let tmp = TempDir::new().unwrap();
        let (tx, _rx) = tokio::sync::broadcast::channel(8);
        let endpoints = HfEndpoints {
            api_base: server.uri(),
            resolve_base: server.uri(),
        };

        let result = download_diffusers_repo(
            &endpoints,
            "stabilityai/sdxl-test",
            "main",
            tmp.path(),
            Arc::new(tx),
        )
        .await
        .unwrap();

        assert!(tmp.path().join("model_index.json").exists());
        assert!(tmp
            .path()
            .join("unet/diffusion_pytorch_model.safetensors")
            .exists());
        assert!(tmp
            .path()
            .join("vae/diffusion_pytorch_model.safetensors")
            .exists());
        assert_eq!(result.files_downloaded, 3);
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
