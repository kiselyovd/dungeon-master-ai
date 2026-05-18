//! Per-model state machine: Idle / Downloading{progress} / Completed / Failed.
//! Wraps `download.rs` with cancellation, progress broadcast, and HF Hub URL
//! resolution.

use crate::models::download::{download_diffusers_repo, download_to, DownloadEvent, HfEndpoints};
use crate::models::manifest::{manifest_for, ModelId, ModelKind, ModelManifest};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::task::JoinHandle;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum DownloadStatus {
    Idle,
    Downloading {
        bytes_done: u64,
        total_bytes: Option<u64>,
    },
    Completed {
        bytes_total: u64,
    },
    Failed {
        reason: String,
    },
}

pub struct DownloadManager {
    base_dir: PathBuf,
    endpoints: HfEndpoints,
    state: Arc<RwLock<HashMap<ModelId, DownloadStatus>>>,
    handles: Arc<RwLock<HashMap<ModelId, JoinHandle<()>>>>,
    pub events: Arc<broadcast::Sender<DownloadEvent>>,
}

impl DownloadManager {
    pub fn new(base_dir: PathBuf) -> Self {
        Self::with_endpoints(base_dir, HfEndpoints::default())
    }

    pub fn with_endpoints(base_dir: PathBuf, endpoints: HfEndpoints) -> Self {
        let (tx, _rx) = broadcast::channel(64);
        Self {
            base_dir,
            endpoints,
            state: Arc::new(RwLock::new(HashMap::new())),
            handles: Arc::new(RwLock::new(HashMap::new())),
            events: Arc::new(tx),
        }
    }

    pub async fn status(&self, id: ModelId) -> DownloadStatus {
        self.state
            .read()
            .await
            .get(&id)
            .cloned()
            .unwrap_or(DownloadStatus::Idle)
    }

    pub async fn start(&self, id: ModelId) -> Result<(), String> {
        let m = manifest_for(&id).ok_or_else(|| format!("unknown model {id:?}"))?;
        let state = self.state.clone();
        let events = self.events.clone();
        state.write().await.insert(
            id.clone(),
            DownloadStatus::Downloading {
                bytes_done: 0,
                total_bytes: Some(m.size_bytes_estimate),
            },
        );

        let handle = match &m.kind {
            ModelKind::GgufFile => {
                let url = format!(
                    "https://huggingface.co/{}/resolve/main/{}",
                    m.hf_repo, m.hf_filename
                );
                let dest = self.base_dir.join(m.hf_filename);
                let sha = m.sha256.to_string();
                let id_for_task = id.clone();
                tokio::spawn(async move {
                    match download_to(&url, &dest, &sha, events.clone()).await {
                        Ok(res) => {
                            state.write().await.insert(
                                id_for_task.clone(),
                                DownloadStatus::Completed {
                                    bytes_total: res.bytes_downloaded,
                                },
                            );
                            let _ = events.send(DownloadEvent::Completed {
                                id: id_for_task,
                                bytes_total: res.bytes_downloaded,
                            });
                        }
                        Err(e) => {
                            state.write().await.insert(
                                id_for_task.clone(),
                                DownloadStatus::Failed {
                                    reason: e.to_string(),
                                },
                            );
                            let _ = events.send(DownloadEvent::Failed {
                                id: id_for_task,
                                reason: e.to_string(),
                            });
                        }
                    }
                })
            }
            ModelKind::DiffusersFolder => {
                let endpoints = self.endpoints.clone();
                let dest_dir = self.base_dir.join(m.hf_repo);
                let hf_repo = m.hf_repo;
                let id_for_task = id.clone();
                tokio::spawn(async move {
                    match download_diffusers_repo(
                        &endpoints,
                        hf_repo,
                        "main",
                        &dest_dir,
                        events.clone(),
                    )
                    .await
                    {
                        Ok(res) => {
                            state.write().await.insert(
                                id_for_task.clone(),
                                DownloadStatus::Completed {
                                    bytes_total: res.bytes_total,
                                },
                            );
                            let _ = events.send(DownloadEvent::Completed {
                                id: id_for_task,
                                bytes_total: res.bytes_total,
                            });
                        }
                        Err(e) => {
                            state.write().await.insert(
                                id_for_task.clone(),
                                DownloadStatus::Failed {
                                    reason: e.to_string(),
                                },
                            );
                            let _ = events.send(DownloadEvent::Failed {
                                id: id_for_task,
                                reason: e.to_string(),
                            });
                        }
                    }
                })
            }
            ModelKind::GgufWithMmproj { mmproj_filename } => {
                let gguf_url = format!(
                    "https://huggingface.co/{}/resolve/main/{}",
                    m.hf_repo, m.hf_filename
                );
                let gguf_dest = self.base_dir.join(m.hf_filename);
                let mmproj_url = format!(
                    "https://huggingface.co/{}/resolve/main/{}",
                    m.hf_repo, mmproj_filename
                );
                let mmproj_dest = self.base_dir.join(mmproj_filename);
                let sha = m.sha256.to_string();
                let id_for_task = id.clone();
                let id_for_event = id.clone();
                tokio::spawn(async move {
                    // Sequential: gguf first (sha-checked), then mmproj
                    // (no sha - Custom mmproj filenames are user-supplied).
                    // mistral.rs vision pipeline expects both files in the
                    // same dir, so we treat them as a single unit.
                    let outcome = match download_to(&gguf_url, &gguf_dest, &sha, events.clone()).await {
                        Ok(g) => match download_to(&mmproj_url, &mmproj_dest, "", events.clone()).await {
                            Ok(mm) => Ok(g.bytes_downloaded + mm.bytes_downloaded),
                            Err(e) => Err(e.to_string()),
                        },
                        Err(e) => Err(e.to_string()),
                    };
                    match outcome {
                        Ok(bytes_total) => {
                            state.write().await.insert(
                                id_for_task,
                                DownloadStatus::Completed { bytes_total },
                            );
                            let _ = events.send(DownloadEvent::Completed {
                                id: id_for_event,
                                bytes_total,
                            });
                        }
                        Err(reason) => {
                            state.write().await.insert(
                                id_for_task,
                                DownloadStatus::Failed {
                                    reason: reason.clone(),
                                },
                            );
                            let _ = events.send(DownloadEvent::Failed {
                                id: id_for_event,
                                reason,
                            });
                        }
                    }
                })
            }
            ModelKind::SafetensorsSingleFile
            | ModelKind::NunchakuSvdquant
            | ModelKind::LtxVideoSafetensors => self.spawn_single_file_download(id.clone(), m, state),
        };
        self.handles.write().await.insert(id, handle);
        Ok(())
    }

    /// Download a single safetensors file. Shared by SafetensorsSingleFile,
    /// NunchakuSvdquant, and LtxVideoSafetensors kinds - all three resolve to
    /// `https://huggingface.co/{hf_repo}/resolve/main/{hf_filename}` and land
    /// at `base_dir / hf_filename`.
    fn spawn_single_file_download(
        &self,
        id: ModelId,
        m: &'static ModelManifest,
        state: Arc<RwLock<HashMap<ModelId, DownloadStatus>>>,
    ) -> JoinHandle<()> {
        let events = self.events.clone();
        let url = format!(
            "https://huggingface.co/{}/resolve/main/{}",
            m.hf_repo, m.hf_filename
        );
        let dest = self.base_dir.join(m.hf_filename);
        let sha = m.sha256.to_string();
        tokio::spawn(async move {
            match download_to(&url, &dest, &sha, events.clone()).await {
                Ok(res) => {
                    state.write().await.insert(
                        id.clone(),
                        DownloadStatus::Completed {
                            bytes_total: res.bytes_downloaded,
                        },
                    );
                    let _ = events.send(DownloadEvent::Completed {
                        id,
                        bytes_total: res.bytes_downloaded,
                    });
                }
                Err(e) => {
                    state.write().await.insert(
                        id.clone(),
                        DownloadStatus::Failed {
                            reason: e.to_string(),
                        },
                    );
                    let _ = events.send(DownloadEvent::Failed {
                        id,
                        reason: e.to_string(),
                    });
                }
            }
        })
    }

    pub async fn cancel(&self, id: ModelId) {
        if let Some(h) = self.handles.write().await.remove(&id) {
            h.abort();
        }
        if let Some(m) = manifest_for(&id) {
            match &m.kind {
                ModelKind::GgufFile => {
                    let dest = self.base_dir.join(m.hf_filename);
                    let _ = tokio::fs::remove_file(&dest).await;
                }
                ModelKind::DiffusersFolder => {
                    let dest_dir = self.base_dir.join(m.hf_repo);
                    let _ = tokio::fs::remove_dir_all(&dest_dir).await;
                }
                ModelKind::GgufWithMmproj { mmproj_filename } => {
                    let gguf_dest = self.base_dir.join(m.hf_filename);
                    let mmproj_dest = self.base_dir.join(mmproj_filename);
                    let _ = tokio::fs::remove_file(&gguf_dest).await;
                    let _ = tokio::fs::remove_file(&mmproj_dest).await;
                }
                ModelKind::SafetensorsSingleFile
                | ModelKind::NunchakuSvdquant
                | ModelKind::LtxVideoSafetensors => {
                    let dest = self.base_dir.join(m.hf_filename);
                    let _ = tokio::fs::remove_file(&dest).await;
                }
            }
        }
        self.state.write().await.insert(id, DownloadStatus::Idle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::manifest::ModelId;

    #[tokio::test]
    async fn idle_initially() {
        let m = DownloadManager::new(std::env::temp_dir());
        assert!(matches!(
            m.status(ModelId::Qwen3_5_0_8b).await,
            DownloadStatus::Idle
        ));
    }

    #[tokio::test]
    async fn cancel_unstarted_returns_idle() {
        let m = DownloadManager::new(std::env::temp_dir());
        m.cancel(ModelId::Qwen3_5_0_8b).await;
        assert!(matches!(
            m.status(ModelId::Qwen3_5_0_8b).await,
            DownloadStatus::Idle
        ));
    }

    #[tokio::test]
    async fn cancel_custom_with_mmproj_removes_both_files() {
        let tmp = tempfile::tempdir().unwrap();
        tokio::fs::write(tmp.path().join("model.gguf"), b"fake")
            .await
            .unwrap();
        tokio::fs::write(tmp.path().join("mmproj-model.gguf"), b"fake")
            .await
            .unwrap();
        let m = DownloadManager::new(tmp.path().to_path_buf());
        let id = ModelId::Custom {
            hf_repo: "x/y".into(),
            gguf_filename: "model.gguf".into(),
            mmproj_filename: Some("mmproj-model.gguf".into()),
        };
        m.cancel(id.clone()).await;
        assert!(
            !tmp.path().join("model.gguf").exists(),
            "main gguf must be removed"
        );
        assert!(
            !tmp.path().join("mmproj-model.gguf").exists(),
            "mmproj must be removed"
        );
        assert!(matches!(m.status(id).await, DownloadStatus::Idle));
    }

    #[tokio::test]
    async fn cancel_custom_no_mmproj_removes_single_file() {
        let tmp = tempfile::tempdir().unwrap();
        tokio::fs::write(tmp.path().join("solo.gguf"), b"fake")
            .await
            .unwrap();
        let m = DownloadManager::new(tmp.path().to_path_buf());
        let id = ModelId::Custom {
            hf_repo: "x/y".into(),
            gguf_filename: "solo.gguf".into(),
            mmproj_filename: None,
        };
        m.cancel(id.clone()).await;
        assert!(
            !tmp.path().join("solo.gguf").exists(),
            "gguf must be removed"
        );
        assert!(matches!(m.status(id).await, DownloadStatus::Idle));
    }

    /// Generic single-file cancel coverage for the 3 new wired ModelKinds. We
    /// can't smoke `start()` without real HF network; cancel exercises the
    /// match arm + the `remove_file(base_dir.join(hf_filename))` cleanup path.
    async fn assert_single_file_cancel_removes(id: ModelId) {
        let m = crate::models::manifest::manifest_for(&id).expect("static manifest entry");
        let tmp = tempfile::tempdir().unwrap();
        tokio::fs::write(tmp.path().join(m.hf_filename), b"fake")
            .await
            .unwrap();
        let mgr = DownloadManager::new(tmp.path().to_path_buf());
        mgr.cancel(id.clone()).await;
        assert!(
            !tmp.path().join(m.hf_filename).exists(),
            "{} must be removed",
            m.hf_filename
        );
        assert!(matches!(mgr.status(id).await, DownloadStatus::Idle));
    }

    #[tokio::test]
    async fn cancel_safetensors_single_file_removes_file() {
        assert_single_file_cancel_removes(ModelId::SdxlLightning4StepLora).await;
    }

    #[tokio::test]
    async fn cancel_nunchaku_svdquant_removes_file() {
        assert_single_file_cancel_removes(ModelId::NunchakuFluxDevInt4).await;
    }

    #[tokio::test]
    async fn cancel_ltx_video_safetensors_removes_file() {
        assert_single_file_cancel_removes(ModelId::LtxVideo09_6Distilled).await;
    }
}
