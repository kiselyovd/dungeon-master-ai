//! Per-model state machine: Idle / Downloading{progress} / Completed / Failed.
//! Wraps `download.rs` with cancellation, progress broadcast, and HF Hub URL
//! resolution.

use crate::models::download::{download_diffusers_repo, download_to, DownloadEvent};
use crate::models::manifest::{lookup, ModelId, ModelKind};
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
    state: Arc<RwLock<HashMap<ModelId, DownloadStatus>>>,
    handles: Arc<RwLock<HashMap<ModelId, JoinHandle<()>>>>,
    pub events: Arc<broadcast::Sender<DownloadEvent>>,
}

impl DownloadManager {
    pub fn new(base_dir: PathBuf) -> Self {
        let (tx, _rx) = broadcast::channel(64);
        Self {
            base_dir,
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
        let m = lookup(id).ok_or_else(|| format!("unknown model {id:?}"))?;
        let state = self.state.clone();
        let events = self.events.clone();
        state.write().await.insert(
            id,
            DownloadStatus::Downloading {
                bytes_done: 0,
                total_bytes: Some(m.size_bytes_estimate),
            },
        );

        let handle = match m.kind {
            ModelKind::GgufFile => {
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
                                id,
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
                                id,
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
            ModelKind::DiffusersFolder => {
                let manifest_url = format!(
                    "https://huggingface.co/{}/resolve/main/model_index.json",
                    m.hf_repo
                );
                let base_url = format!("https://huggingface.co/{}/resolve/main", m.hf_repo);
                let dest_dir = self.base_dir.join(m.hf_repo);
                tokio::spawn(async move {
                    match download_diffusers_repo(
                        &manifest_url,
                        &base_url,
                        &dest_dir,
                        events.clone(),
                    )
                    .await
                    {
                        Ok(res) => {
                            state.write().await.insert(
                                id,
                                DownloadStatus::Completed {
                                    bytes_total: res.bytes_total,
                                },
                            );
                            let _ = events.send(DownloadEvent::Completed {
                                id,
                                bytes_total: res.bytes_total,
                            });
                        }
                        Err(e) => {
                            state.write().await.insert(
                                id,
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
        };
        self.handles.write().await.insert(id, handle);
        Ok(())
    }

    pub async fn cancel(&self, id: ModelId) {
        if let Some(h) = self.handles.write().await.remove(&id) {
            h.abort();
        }
        if let Some(m) = lookup(id) {
            match m.kind {
                ModelKind::GgufFile => {
                    let dest = self.base_dir.join(m.hf_filename);
                    let _ = tokio::fs::remove_file(&dest).await;
                }
                ModelKind::DiffusersFolder => {
                    let dest_dir = self.base_dir.join(m.hf_repo);
                    let _ = tokio::fs::remove_dir_all(&dest_dir).await;
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
}
