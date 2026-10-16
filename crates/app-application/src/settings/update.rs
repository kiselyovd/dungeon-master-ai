use std::sync::Arc;

use async_trait::async_trait;
use thiserror::Error;

use crate::models::settings::SettingsConfigV2;
use crate::ports::secrets::SecretsStore;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretMutation {
    Set { key: String, value: String },
    Delete { key: String },
}

pub struct PreparedSettings<T> {
    pub snapshot: T,
    pub secret_mutations: Vec<SecretMutation>,
    pub license_restricted_no_compat: bool,
}

#[async_trait]
pub trait SettingsFactory<T>: Send + Sync {
    async fn prepare(&self, config: SettingsConfigV2) -> Result<PreparedSettings<T>, &'static str>;
}

pub trait SettingsCommit<T>: Send + Sync {
    fn commit(&self, snapshot: T) -> Result<u64, &'static str>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateSettingsResult {
    pub revision: u64,
    pub license_restricted_no_compat: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SettingsUpdateError {
    #[error("settings preparation failed with code {code}")]
    Prepare { code: &'static str },
    #[error(
        "settings secret transaction failed with code {code}; rollback_failed={rollback_failed}"
    )]
    Secret {
        code: &'static str,
        rollback_failed: bool,
    },
    #[error("settings commit failed with code {code}; rollback_failed={rollback_failed}")]
    Commit {
        code: &'static str,
        rollback_failed: bool,
    },
}

pub struct UpdateSettings<T> {
    factory: Arc<dyn SettingsFactory<T>>,
    secrets: Arc<dyn SecretsStore>,
    commit: Arc<dyn SettingsCommit<T>>,
}

impl<T: Send + 'static> UpdateSettings<T> {
    pub fn new(
        factory: Arc<dyn SettingsFactory<T>>,
        secrets: Arc<dyn SecretsStore>,
        commit: Arc<dyn SettingsCommit<T>>,
    ) -> Self {
        Self {
            factory,
            secrets,
            commit,
        }
    }

    #[tracing::instrument(skip_all)]
    pub async fn execute(
        &self,
        config: SettingsConfigV2,
    ) -> Result<UpdateSettingsResult, SettingsUpdateError> {
        let prepared = self
            .factory
            .prepare(config)
            .await
            .map_err(|code| SettingsUpdateError::Prepare { code })?;
        let originals = self.capture_originals(&prepared.secret_mutations).await?;
        if let Err(code) = self.apply_mutations(&prepared.secret_mutations).await {
            let rollback_failed = self.restore_originals(&originals).await.is_err();
            return Err(SettingsUpdateError::Secret {
                code,
                rollback_failed,
            });
        }
        let revision = match self.commit.commit(prepared.snapshot) {
            Ok(revision) => revision,
            Err(code) => {
                let rollback_failed = self.restore_originals(&originals).await.is_err();
                return Err(SettingsUpdateError::Commit {
                    code,
                    rollback_failed,
                });
            }
        };
        tracing::info!(revision, "settings snapshot committed");
        Ok(UpdateSettingsResult {
            revision,
            license_restricted_no_compat: prepared.license_restricted_no_compat,
        })
    }

    async fn capture_originals(
        &self,
        mutations: &[SecretMutation],
    ) -> Result<Vec<(String, Option<String>)>, SettingsUpdateError> {
        let mut originals = Vec::with_capacity(mutations.len());
        for mutation in mutations {
            let key = match mutation {
                SecretMutation::Set { key, .. } | SecretMutation::Delete { key } => key,
            };
            let value = self
                .secrets
                .get(key)
                .await
                .map_err(|_| SettingsUpdateError::Secret {
                    code: "capture",
                    rollback_failed: false,
                })?;
            originals.push((key.clone(), value));
        }
        Ok(originals)
    }

    async fn apply_mutations(&self, mutations: &[SecretMutation]) -> Result<(), &'static str> {
        for mutation in mutations {
            let result = match mutation {
                SecretMutation::Set { key, value } => self.secrets.set(key, value).await,
                SecretMutation::Delete { key } => self.secrets.delete(key).await,
            };
            if result.is_err() {
                return Err("write");
            }
        }
        Ok(())
    }

    async fn restore_originals(&self, originals: &[(String, Option<String>)]) -> Result<(), ()> {
        let mut failed = false;
        for (key, value) in originals.iter().rev() {
            let result = match value {
                Some(value) => self.secrets.set(key, value).await,
                None => self.secrets.delete(key).await,
            };
            failed |= result.is_err();
        }
        if failed {
            Err(())
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

    use tokio::sync::RwLock;

    use super::*;
    use crate::ports::secrets::SecretsError;

    struct FakeFactory {
        fail: bool,
        mutations: Vec<SecretMutation>,
    }

    #[async_trait]
    impl SettingsFactory<u64> for FakeFactory {
        async fn prepare(
            &self,
            _config: SettingsConfigV2,
        ) -> Result<PreparedSettings<u64>, &'static str> {
            if self.fail {
                return Err("factory");
            }
            Ok(PreparedSettings {
                snapshot: 42,
                secret_mutations: self.mutations.clone(),
                license_restricted_no_compat: false,
            })
        }
    }

    #[derive(Default)]
    struct FakeSecrets {
        values: RwLock<HashMap<String, String>>,
        writes: AtomicUsize,
        fail_on_write: AtomicUsize,
    }

    #[async_trait]
    impl SecretsStore for FakeSecrets {
        async fn get(&self, key: &str) -> Result<Option<String>, SecretsError> {
            Ok(self.values.read().await.get(key).cloned())
        }

        async fn set(&self, key: &str, value: &str) -> Result<(), SecretsError> {
            let write = self.writes.fetch_add(1, Ordering::SeqCst) + 1;
            if self.fail_on_write.load(Ordering::SeqCst) == write {
                return Err(secret_error());
            }
            self.values
                .write()
                .await
                .insert(key.to_string(), value.to_string());
            Ok(())
        }

        async fn delete(&self, key: &str) -> Result<(), SecretsError> {
            let write = self.writes.fetch_add(1, Ordering::SeqCst) + 1;
            if self.fail_on_write.load(Ordering::SeqCst) == write {
                return Err(secret_error());
            }
            self.values.write().await.remove(key);
            Ok(())
        }
    }

    fn secret_error() -> SecretsError {
        SecretsError::Operation {
            operation: "test",
            code: "injected",
        }
    }

    #[derive(Default)]
    struct FakeCommit {
        committed: AtomicBool,
        revision: AtomicU64,
        fail: bool,
    }

    impl SettingsCommit<u64> for FakeCommit {
        fn commit(&self, snapshot: u64) -> Result<u64, &'static str> {
            if self.fail {
                return Err("commit");
            }
            assert_eq!(snapshot, 42);
            self.committed.store(true, Ordering::SeqCst);
            Ok(self.revision.fetch_add(1, Ordering::SeqCst) + 1)
        }
    }

    fn config() -> SettingsConfigV2 {
        serde_json::from_value(serde_json::json!({
            "chat": {
                "active_provider_id": "openai-compat",
                "active_model_id": "model",
                "providers": {},
                "vision_enabled": false,
                "reasoning_enabled": false,
                "reasoning_budget": "medium"
            },
            "image": {
                "enabled": false,
                "active_provider_id": "local-sdxl-lightning",
                "active_model_id": "image",
                "providers": {},
                "preset": "balanced",
                "style_lora": null
            },
            "video": {
                "enabled": false,
                "active_provider_id": "local-ltx-video",
                "active_model_id": "video",
                "providers": {},
                "mode": "prerecorded"
            },
            "behavior": {
                "system_prompt": "DM",
                "temperature": 0.7,
                "ui_language": "en",
                "narration_language": "en"
            }
        }))
        .unwrap()
    }

    #[tokio::test]
    async fn preparation_failure_performs_no_writes_or_commit() {
        let secrets = Arc::new(FakeSecrets::default());
        let commit = Arc::new(FakeCommit::default());
        let update = UpdateSettings::new(
            Arc::new(FakeFactory {
                fail: true,
                mutations: vec![],
            }),
            secrets.clone(),
            commit.clone(),
        );
        assert!(matches!(
            update.execute(config()).await,
            Err(SettingsUpdateError::Prepare { .. })
        ));
        assert_eq!(secrets.writes.load(Ordering::SeqCst), 0);
        assert!(!commit.committed.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn secret_failure_restores_originals_and_skips_snapshot_commit() {
        let secrets = Arc::new(FakeSecrets::default());
        secrets
            .values
            .write()
            .await
            .insert("one".into(), "old".into());
        secrets.fail_on_write.store(2, Ordering::SeqCst);
        let commit = Arc::new(FakeCommit::default());
        let update = UpdateSettings::new(
            Arc::new(FakeFactory {
                fail: false,
                mutations: vec![
                    SecretMutation::Set {
                        key: "one".into(),
                        value: "new".into(),
                    },
                    SecretMutation::Set {
                        key: "two".into(),
                        value: "new".into(),
                    },
                ],
            }),
            secrets.clone(),
            commit.clone(),
        );
        assert!(matches!(
            update.execute(config()).await,
            Err(SettingsUpdateError::Secret {
                rollback_failed: false,
                ..
            })
        ));
        assert_eq!(secrets.get("one").await.unwrap().as_deref(), Some("old"));
        assert_eq!(secrets.get("two").await.unwrap(), None);
        assert!(!commit.committed.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn commit_failure_rolls_back_secrets() {
        let secrets = Arc::new(FakeSecrets::default());
        let commit = Arc::new(FakeCommit {
            fail: true,
            ..FakeCommit::default()
        });
        let update = UpdateSettings::new(
            Arc::new(FakeFactory {
                fail: false,
                mutations: vec![SecretMutation::Set {
                    key: "one".into(),
                    value: "new".into(),
                }],
            }),
            secrets.clone(),
            commit,
        );
        assert!(matches!(
            update.execute(config()).await,
            Err(SettingsUpdateError::Commit {
                rollback_failed: false,
                ..
            })
        ));
        assert_eq!(secrets.get("one").await.unwrap(), None);
    }

    #[tokio::test]
    async fn successful_update_commits_one_revision_after_secret_writes() {
        let secrets = Arc::new(FakeSecrets::default());
        let commit = Arc::new(FakeCommit::default());
        let update = UpdateSettings::new(
            Arc::new(FakeFactory {
                fail: false,
                mutations: vec![SecretMutation::Set {
                    key: "one".into(),
                    value: "new".into(),
                }],
            }),
            secrets.clone(),
            commit.clone(),
        );
        assert_eq!(update.execute(config()).await.unwrap().revision, 1);
        assert_eq!(secrets.get("one").await.unwrap().as_deref(), Some("new"));
        assert!(commit.committed.load(Ordering::SeqCst));
    }
}
