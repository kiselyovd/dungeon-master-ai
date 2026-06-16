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
        /// True when the failure was a 401/403 from HuggingFace; the frontend
        /// uses this to surface an "Add HuggingFace token" affordance.
        auth_required: bool,
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
    #[error("HuggingFace authorization required")]
    Unauthorized,
    #[error("cancelled")]
    Cancelled,
}

#[derive(Debug)]
pub struct DownloadResult {
    pub bytes_downloaded: u64,
    pub final_path: PathBuf,
}

pub async fn download_to(
    url: &str,
    dest: &Path,
    expected_sha256: &str,
    token: Option<&str>,
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
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }
    if starting_offset > 0 {
        req = req.header("Range", format!("bytes={}-", starting_offset));
    }
    let resp = req.send().await?;
    if matches!(resp.status().as_u16(), 401 | 403) {
        return Err(DownloadError::Unauthorized);
    }
    let resp = resp.error_for_status()?;
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
/// Pick the subset of a diffusers repo needed for a `variant="fp16"` load:
/// keep configs/tokenizer/json, keep fp16 weights, keep weights that have no
/// fp16 variant, and drop onnx exports, sample images, and the fp32 weight
/// counterpart whenever an fp16 sibling exists. Avoids pulling the whole repo
/// (fp16 + fp32 + onnx) when only fp16 is loaded.
fn select_diffusers_files(all: Vec<String>) -> Vec<String> {
    // `.onnx_data` is the multi-GB external-data blob beside `.onnx`; matching
    // only `.onnx` let it (9.5 GB for SDXL-Turbo's unet) slip through.
    const DROP_EXT: &[&str] = &[
        ".onnx",
        ".onnx_data",
        ".png",
        ".jpg",
        ".jpeg",
        ".gif",
        ".ckpt",
        ".pb",
        ".h5",
        ".msgpack",
        ".pt",
    ];
    const WEIGHT_EXT: &[&str] = &[".safetensors", ".bin"];
    // Repos use both `.fp16.` and `_fp16.` to tag the half-precision variant.
    let is_fp16 = |p: &str| p.contains(".fp16.") || p.contains("_fp16.");
    // Non-fp16 paths that DO have an fp16 sibling, e.g.
    // "unet/diffusion_pytorch_model.safetensors" when the fp16 file exists.
    let fp16_counterparts: std::collections::HashSet<String> = all
        .iter()
        .filter(|p| is_fp16(p))
        .map(|p| p.replace(".fp16.", ".").replace("_fp16.", "."))
        .collect();
    all.into_iter()
        .filter(|p| {
            let lower = p.to_ascii_lowercase();
            if DROP_EXT.iter().any(|e| lower.ends_with(e)) {
                return false;
            }
            // Drop the fp32 weight when its fp16 variant is also present.
            if !is_fp16(p) && fp16_counterparts.contains(p) {
                return false;
            }
            // Drop root-level single-file checkpoints (e.g.
            // "sd_xl_turbo_1.0.safetensors"): a `from_pretrained(folder,
            // variant="fp16")` load reads only the diffusers SUBFOLDER weights,
            // never a top-level single-file SD checkpoint, so these ~13 GB blobs
            // are pure waste.
            let is_weight = WEIGHT_EXT.iter().any(|e| lower.ends_with(e));
            if is_weight && !p.contains('/') {
                return false;
            }
            true
        })
        .collect()
}

pub async fn download_diffusers_repo(
    endpoints: &HfEndpoints,
    hf_repo: &str,
    revision: &str,
    dest_dir: &Path,
    token: Option<&str>,
    tx: Arc<broadcast::Sender<DownloadEvent>>,
) -> Result<DiffusersResult, DownloadError> {
    let tree_url = format!(
        "{}/api/models/{hf_repo}/tree/{revision}?recursive=true",
        endpoints.api_base
    );
    let client = reqwest::Client::new();
    let mut tree_req = client.get(&tree_url);
    if let Some(t) = token {
        tree_req = tree_req.bearer_auth(t);
    }
    let tree_resp = tree_req.send().await?;
    if matches!(tree_resp.status().as_u16(), 401 | 403) {
        return Err(DownloadError::Unauthorized);
    }
    let entries: Vec<HfTreeEntry> = tree_resp.error_for_status()?.json().await?;
    let all_files: Vec<String> = entries
        .into_iter()
        .filter(|e| e.kind == "file")
        .map(|e| e.path)
        .collect();
    // Only fetch what a `variant="fp16"` diffusers load needs: configs +
    // fp16 weights. Skip onnx exports, sample images, and the fp32 weight
    // counterparts when an fp16 variant exists - downloading the whole repo
    // pulled ~40 GB for SDXL-Turbo (fp16 + fp32 + onnx) instead of ~7 GB.
    let files = select_diffusers_files(all_files);

    let mut bytes_total = 0u64;
    let mut files_downloaded = 0u32;
    let semaphore = Arc::new(tokio::sync::Semaphore::new(4));
    let mut handles = Vec::new();
    let resolve_base = endpoints.resolve_base.clone();
    let token_owned = token.map(|s| s.to_string());
    for relpath in files {
        let url = format!("{resolve_base}/{hf_repo}/resolve/{revision}/{relpath}");
        let dest = dest_dir.join(&relpath);
        let permit = semaphore.clone();
        let tx = tx.clone();
        let token = token_owned.clone();
        handles.push(tokio::spawn(async move {
            let _p = permit.acquire_owned().await.expect("semaphore");
            download_to(&url, &dest, "", token.as_deref(), tx).await
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

    #[test]
    fn select_diffusers_files_keeps_fp16_drops_fp32_and_onnx() {
        let all = vec![
            "model_index.json".to_string(),
            "unet/config.json".to_string(),
            "unet/diffusion_pytorch_model.safetensors".to_string(), // fp32 - drop
            "unet/diffusion_pytorch_model.fp16.safetensors".to_string(), // keep
            "text_encoder/model.onnx".to_string(),                  // drop
            "vae/diffusion_pytorch_model.fp16.safetensors".to_string(), // keep
            "tokenizer/merges.txt".to_string(),                     // keep
            "sample.png".to_string(),                               // drop
        ];
        let mut got = select_diffusers_files(all);
        got.sort();
        assert_eq!(
            got,
            vec![
                "model_index.json".to_string(),
                "tokenizer/merges.txt".to_string(),
                "unet/config.json".to_string(),
                "unet/diffusion_pytorch_model.fp16.safetensors".to_string(),
                "vae/diffusion_pytorch_model.fp16.safetensors".to_string(),
            ]
        );
    }

    #[test]
    fn select_diffusers_files_drops_sdxl_turbo_root_checkpoints_and_onnx_data() {
        // Real stabilityai/sdxl-turbo tree. The fp16 folder load needs only the
        // subfolder fp16 weights (~6.5 GB); without these drops the walker pulled
        // ~38 GB: root single-file checkpoints (incl. `_fp16.` naming) + the huge
        // `.onnx_data` blobs that `.onnx` alone did not match.
        let all = vec![
            "model_index.json".to_string(),
            "sd_xl_turbo_1.0.safetensors".to_string(), // root fp32 single-file - drop
            "sd_xl_turbo_1.0_fp16.safetensors".to_string(), // root fp16 single-file - drop
            "unet/diffusion_pytorch_model.fp16.safetensors".to_string(), // keep
            "unet/model.onnx".to_string(),             // drop
            "unet/model.onnx_data".to_string(),        // drop (9.5 GB leak)
            "text_encoder_2/model.fp16.safetensors".to_string(), // keep
            "text_encoder_2/model.onnx_data".to_string(), // drop (2.6 GB leak)
            "vae/diffusion_pytorch_model.fp16.safetensors".to_string(), // keep
        ];
        let mut got = select_diffusers_files(all);
        got.sort();
        assert_eq!(
            got,
            vec![
                "model_index.json".to_string(),
                "text_encoder_2/model.fp16.safetensors".to_string(),
                "unet/diffusion_pytorch_model.fp16.safetensors".to_string(),
                "vae/diffusion_pytorch_model.fp16.safetensors".to_string(),
            ]
        );
    }

    #[test]
    fn select_diffusers_files_drops_fp32_single_file_with_underscore_fp16_sibling() {
        // `_fp16.` (underscore) naming must be recognised the same as `.fp16.`.
        let all = vec![
            "transformer/model.safetensors".to_string(), // subfolder, keep (no fp16 sibling here)
            "pipeline_fp16.safetensors".to_string(),     // root - drop (single-file)
        ];
        let mut got = select_diffusers_files(all);
        got.sort();
        assert_eq!(got, vec!["transformer/model.safetensors".to_string()]);
    }

    #[test]
    fn select_diffusers_files_keeps_weights_without_fp16_variant() {
        // A repo that ships only a single non-fp16 weight must still download it.
        let all = vec![
            "model_index.json".to_string(),
            "transformer/diffusion_pytorch_model.safetensors".to_string(),
        ];
        let mut got = select_diffusers_files(all);
        got.sort();
        assert_eq!(
            got,
            vec![
                "model_index.json".to_string(),
                "transformer/diffusion_pytorch_model.safetensors".to_string(),
            ]
        );
    }
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
        let result = download_to(&url, &dest, "", None, Arc::new(tx))
            .await
            .unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), payload);
        assert_eq!(result.bytes_downloaded, 11);
    }

    #[tokio::test]
    async fn download_to_sends_bearer_token_when_present() {
        use wiremock::matchers::header;

        let server = MockServer::start().await;
        let body = b"hello-weights".to_vec();
        Mock::given(method("GET"))
            .and(header("authorization", "Bearer secret-tok"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(body.clone()))
            .mount(&server)
            .await;

        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("w.gguf");
        let url = format!("{}/file", server.uri());
        let (tx, _rx) = tokio::sync::broadcast::channel(8);
        let res = download_to(&url, &dest, "", Some("secret-tok"), Arc::new(tx)).await;

        assert!(
            res.is_ok(),
            "download with bearer token should succeed: {res:?}"
        );
        assert_eq!(std::fs::read(&dest).unwrap(), body);
    }

    #[tokio::test]
    async fn download_to_maps_401_to_unauthorized() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/f"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("f");
        let url = format!("{}/f", server.uri());
        let (tx, _rx) = tokio::sync::broadcast::channel(8);
        let res = download_to(&url, &dest, "", None, Arc::new(tx)).await;
        assert!(
            matches!(res, Err(DownloadError::Unauthorized)),
            "got {res:?}"
        );
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
            None,
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
            .and(path("/stabilityai/sdxl-test/resolve/main/model_index.json"))
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
            None,
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
        let result = download_to(&url, &dest, "", None, Arc::new(tx))
            .await
            .unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), full);
        assert_eq!(result.bytes_downloaded, 5);
    }
}
