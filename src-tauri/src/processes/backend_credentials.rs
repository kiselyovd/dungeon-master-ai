use std::ffi::OsString;
use std::fs::{File, OpenOptions, TryLockError};
use std::path::Path;
use std::time::{Duration, Instant};

use super::backend_profile::{normalize_path, snapshot_exists};

const SERVICE: &str = "com.kiselyovd.dungeonmasterai.backend-vault";
const LOCK_TIMEOUT: Duration = Duration::from_secs(10);

pub(super) struct BackendCredential(String);

impl BackendCredential {
    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

trait CredentialStore {
    fn read(&self) -> Result<Option<String>, &'static str>;
    fn write(&self, value: &str) -> Result<(), &'static str>;
}

struct OsCredentialStore {
    entry: keyring::Entry,
}

impl OsCredentialStore {
    fn new(service: &str, account: &str) -> Result<Self, &'static str> {
        let entry = keyring::Entry::new(service, account)
            .map_err(|_| "backend_credential_store_unavailable")?;
        Ok(Self { entry })
    }
}

impl CredentialStore for OsCredentialStore {
    fn read(&self) -> Result<Option<String>, &'static str> {
        match self.entry.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err("backend_credential_store_unavailable"),
        }
    }

    fn write(&self, value: &str) -> Result<(), &'static str> {
        self.entry
            .set_password(value)
            .map_err(|_| "backend_credential_store_unavailable")
    }
}

pub(super) fn resolve(
    vault: Option<&Path>,
    environment: Option<OsString>,
) -> Result<Option<BackendCredential>, &'static str> {
    resolve_with(
        vault,
        environment,
        |account| OsCredentialStore::new(SERVICE, account),
        generate_credential,
    )
}

fn resolve_with<S: CredentialStore>(
    vault: Option<&Path>,
    environment: Option<OsString>,
    store_factory: impl FnOnce(&str) -> Result<S, &'static str>,
    generate: impl FnOnce() -> String,
) -> Result<Option<BackendCredential>, &'static str> {
    if let Some(environment) = environment {
        let value = environment
            .into_string()
            .map_err(|_| "backend_vault_passphrase_invalid")?;
        if !valid_credential(&value) {
            return Err("backend_vault_passphrase_invalid");
        }
        return Ok(Some(BackendCredential(value)));
    }
    let Some(vault) = vault else {
        return Ok(None);
    };
    let identity = credential_identity(vault)?;
    let store = store_factory(&identity)?;
    if let Some(value) = store.read()? {
        return checked_credential(value).map(Some);
    }
    if snapshot_exists(vault)? {
        return Err("backend_vault_unlock_required");
    }
    // Keep the lock file: unlinking it could let a later opener lock a different inode.
    let _lock = acquire_creation_lock(vault, LOCK_TIMEOUT)?;
    if let Some(value) = store.read()? {
        return checked_credential(value).map(Some);
    }
    if snapshot_exists(vault)? {
        return Err("backend_vault_unlock_required");
    }
    let generated = generate();
    if !valid_credential(&generated) {
        return Err("backend_credential_invalid");
    }
    store.write(&generated)?;
    let readback = store.read()?;
    if readback.as_deref() != Some(generated.as_str()) {
        return Err("backend_credential_readback_failed");
    }
    Ok(Some(BackendCredential(generated)))
}

fn checked_credential(value: String) -> Result<BackendCredential, &'static str> {
    if valid_credential(&value) {
        Ok(BackendCredential(value))
    } else {
        Err("backend_credential_invalid")
    }
}

fn valid_credential(value: &str) -> bool {
    !value.is_empty() && !value.contains('\0')
}

fn credential_identity(vault: &Path) -> Result<String, &'static str> {
    if !vault.is_absolute() {
        return Err("backend_profile_path_invalid");
    }
    let path = normalize_path(vault, Path::new("."))?;
    let text = path.to_str().ok_or("backend_profile_path_invalid")?;
    #[cfg(windows)]
    let text = text.replace('\\', "/").to_lowercase();
    Ok(format!("backend-vault:{text}"))
}

fn generate_credential() -> String {
    // Two UUIDv4 values provide 244 random bits from the existing OS-backed generator.
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

fn acquire_creation_lock(vault: &Path, timeout: Duration) -> Result<File, &'static str> {
    let parent = vault.parent().ok_or("backend_profile_path_invalid")?;
    std::fs::create_dir_all(parent).map_err(|_| "backend_profile_directory_unavailable")?;
    let mut filename = vault
        .file_name()
        .ok_or("backend_profile_path_invalid")?
        .to_os_string();
    filename.push(".credential-init.lock");
    let path = parent.join(filename);
    match std::fs::symlink_metadata(&path) {
        Ok(metadata) if !metadata.is_file() || metadata.file_type().is_symlink() => {
            return Err("backend_credential_lock_unavailable");
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err("backend_credential_lock_unavailable"),
        Ok(_) => {}
    }
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
        .map_err(|_| "backend_credential_lock_unavailable")?;
    let start = Instant::now();
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(file),
            Err(TryLockError::WouldBlock) if start.elapsed() < timeout => {
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(TryLockError::WouldBlock) => return Err("backend_credential_lock_timeout"),
            Err(_) => return Err("backend_credential_lock_unavailable"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Clone, Default)]
    struct FakeStore(Rc<RefCell<FakeState>>);

    #[derive(Default)]
    struct FakeState {
        value: Option<String>,
        reads: usize,
        writes: usize,
        read_error: bool,
        write_error: bool,
        corrupt_readback: bool,
        value_on_second_read: Option<String>,
        snapshot_on_second_read: Option<std::path::PathBuf>,
    }

    impl CredentialStore for FakeStore {
        fn read(&self) -> Result<Option<String>, &'static str> {
            let mut state = self.0.borrow_mut();
            state.reads += 1;
            if state.reads == 2 {
                if let Some(value) = state.value_on_second_read.take() {
                    state.value = Some(value);
                }
                if let Some(path) = state.snapshot_on_second_read.take() {
                    std::fs::write(path, b"another-owner-snapshot").unwrap();
                }
            }
            if state.read_error {
                Err("backend_credential_store_unavailable")
            } else if state.corrupt_readback && state.writes > 0 {
                Ok(Some("unexpected-readback".into()))
            } else {
                Ok(state.value.clone())
            }
        }

        fn write(&self, value: &str) -> Result<(), &'static str> {
            let mut state = self.0.borrow_mut();
            state.writes += 1;
            if state.write_error {
                return Err("backend_credential_store_unavailable");
            }
            state.value = Some(value.into());
            Ok(())
        }
    }

    fn resolve_fake(
        path: &Path,
        environment: Option<OsString>,
        store: FakeStore,
    ) -> Result<Option<BackendCredential>, &'static str> {
        resolve_with(
            Some(path),
            environment,
            |_| Ok(store),
            || "generated-secret".into(),
        )
    }

    #[test]
    fn clean_install_persists_and_reuses_the_credential_after_restart() {
        let directory = tempfile::tempdir().unwrap();
        let vault = directory.path().join("vault.hold");
        let store = FakeStore::default();
        let first = resolve_fake(&vault, None, store.clone()).unwrap().unwrap();
        assert_eq!(first.as_str(), "generated-secret");
        assert_eq!(store.0.borrow().writes, 1);
        std::fs::write(&vault, b"encrypted-snapshot").unwrap();
        let second = resolve_fake(&vault, None, store.clone()).unwrap().unwrap();
        assert_eq!(second.as_str(), first.as_str());
        assert_eq!(store.0.borrow().writes, 1);
        assert_eq!(std::fs::read(vault).unwrap(), b"encrypted-snapshot");
    }

    #[test]
    fn environment_override_never_constructs_a_keyring_entry() {
        let directory = tempfile::tempdir().unwrap();
        let value = resolve_with(
            Some(&directory.path().join("vault.hold")),
            Some("operator-secret".into()),
            |_| -> Result<FakeStore, &'static str> { panic!("keyring must be bypassed") },
            || panic!("random generation must be bypassed"),
        )
        .unwrap()
        .unwrap();
        assert_eq!(value.as_str(), "operator-secret");
    }

    #[test]
    fn empty_environment_override_is_an_error_without_store_access() {
        let directory = tempfile::tempdir().unwrap();
        let store = FakeStore::default();
        assert_eq!(
            resolve_fake(
                &directory.path().join("vault.hold"),
                Some("".into()),
                store.clone()
            )
            .err(),
            Some("backend_vault_passphrase_invalid")
        );
        assert_eq!(store.0.borrow().reads, 0);
        assert_eq!(store.0.borrow().writes, 0);
    }

    #[cfg(unix)]
    #[test]
    fn non_unicode_environment_override_fails_without_store_access() {
        use std::os::unix::ffi::OsStringExt;
        let directory = tempfile::tempdir().unwrap();
        let store = FakeStore::default();
        assert!(resolve_fake(
            &directory.path().join("vault.hold"),
            Some(OsString::from_vec(vec![0xff])),
            store.clone()
        )
        .is_err());
        assert_eq!(store.0.borrow().reads, 0);
    }

    #[test]
    fn existing_snapshot_without_credential_is_preserved_without_lock_writes() {
        let directory = tempfile::tempdir().unwrap();
        let vault = directory.path().join("vault.hold");
        std::fs::write(&vault, b"preserve-snapshot").unwrap();
        let store = FakeStore::default();
        assert_eq!(
            resolve_fake(&vault, None, store.clone()).err(),
            Some("backend_vault_unlock_required")
        );
        assert_eq!(store.0.borrow().writes, 0);
        assert_eq!(std::fs::read(&vault).unwrap(), b"preserve-snapshot");
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn denied_store_and_failed_write_never_return_a_generated_credential() {
        for fail_read in [true, false] {
            let directory = tempfile::tempdir().unwrap();
            let store = FakeStore::default();
            store.0.borrow_mut().read_error = fail_read;
            store.0.borrow_mut().write_error = !fail_read;
            assert!(
                resolve_fake(&directory.path().join("vault.hold"), None, store.clone()).is_err()
            );
            assert!(store.0.borrow().value.is_none());
        }
    }

    #[test]
    fn mismatched_readback_fails_without_overwriting_snapshot_or_retrying_write() {
        let directory = tempfile::tempdir().unwrap();
        let store = FakeStore::default();
        store.0.borrow_mut().corrupt_readback = true;
        assert_eq!(
            resolve_fake(&directory.path().join("vault.hold"), None, store.clone()).err(),
            Some("backend_credential_readback_failed")
        );
        assert_eq!(store.0.borrow().writes, 1);
        assert!(!directory.path().join("vault.hold").try_exists().unwrap());
    }

    #[test]
    fn empty_existing_credential_is_not_replaced() {
        let directory = tempfile::tempdir().unwrap();
        let store = FakeStore::default();
        store.0.borrow_mut().value = Some(String::new());
        assert_eq!(
            resolve_fake(&directory.path().join("vault.hold"), None, store.clone()).err(),
            Some("backend_credential_invalid")
        );
        assert_eq!(store.0.borrow().writes, 0);
    }

    #[test]
    fn credential_created_before_lock_acquisition_is_not_overwritten() {
        let directory = tempfile::tempdir().unwrap();
        let store = FakeStore::default();
        store.0.borrow_mut().value_on_second_read = Some("other-owner-secret".into());
        let value = resolve_fake(&directory.path().join("vault.hold"), None, store.clone())
            .unwrap()
            .unwrap();
        assert_eq!(value.as_str(), "other-owner-secret");
        assert_eq!(store.0.borrow().writes, 0);
    }

    #[test]
    fn snapshot_created_before_locked_recheck_is_never_replaced() {
        let directory = tempfile::tempdir().unwrap();
        let vault = directory.path().join("vault.hold");
        let store = FakeStore::default();
        store.0.borrow_mut().snapshot_on_second_read = Some(vault.clone());
        assert_eq!(
            resolve_fake(&vault, None, store.clone()).err(),
            Some("backend_vault_unlock_required")
        );
        assert_eq!(store.0.borrow().writes, 0);
        assert_eq!(std::fs::read(vault).unwrap(), b"another-owner-snapshot");
    }

    #[test]
    fn creation_lock_has_bounded_wait_and_is_reusable_without_unlink() {
        let directory = tempfile::tempdir().unwrap();
        let vault = directory.path().join("vault.hold");
        let first = acquire_creation_lock(&vault, Duration::ZERO).unwrap();
        assert_eq!(
            acquire_creation_lock(&vault, Duration::ZERO).err(),
            Some("backend_credential_lock_timeout")
        );
        drop(first);
        let second = acquire_creation_lock(&vault, Duration::ZERO).unwrap();
        drop(second);
        assert!(directory
            .path()
            .join("vault.hold.credential-init.lock")
            .is_file());
    }

    #[test]
    fn concurrent_initializers_share_one_persisted_credential() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::{Arc, Barrier, Mutex};

        #[derive(Clone)]
        struct SharedStore {
            value: Arc<Mutex<Option<String>>>,
            writes: Arc<AtomicUsize>,
        }

        impl CredentialStore for SharedStore {
            fn read(&self) -> Result<Option<String>, &'static str> {
                Ok(self.value.lock().unwrap().clone())
            }

            fn write(&self, value: &str) -> Result<(), &'static str> {
                self.writes.fetch_add(1, Ordering::SeqCst);
                *self.value.lock().unwrap() = Some(value.into());
                Ok(())
            }
        }

        let directory = tempfile::tempdir().unwrap();
        let vault = directory.path().join("vault.hold");
        let store = SharedStore {
            value: Arc::new(Mutex::new(None)),
            writes: Arc::new(AtomicUsize::new(0)),
        };
        let barrier = Arc::new(Barrier::new(4));
        let threads: Vec<_> = (0..4)
            .map(|owner| {
                let vault = vault.clone();
                let store = store.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    resolve_with(
                        Some(&vault),
                        None,
                        |_| Ok(store),
                        || format!("owner-{owner}"),
                    )
                    .unwrap()
                    .unwrap()
                    .as_str()
                    .to_owned()
                })
            })
            .collect();
        let values: Vec<_> = threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect();
        assert!(values.iter().all(|value| value == &values[0]));
        assert_eq!(store.writes.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn profile_identities_are_distinct_and_lexically_stable() {
        let directory = tempfile::tempdir().unwrap();
        let first = credential_identity(&directory.path().join("one/vault.hold")).unwrap();
        let same = credential_identity(&directory.path().join("one/../one/vault.hold")).unwrap();
        let second = credential_identity(&directory.path().join("two/vault.hold")).unwrap();
        assert_eq!(first, same);
        assert_ne!(first, second);
    }

    #[test]
    fn explicit_memory_profile_does_not_access_the_os_store() {
        assert!(resolve_with(
            None,
            None,
            |_| -> Result<FakeStore, &'static str> { panic!("no persistent vault") },
            || panic!("no random credential")
        )
        .unwrap()
        .is_none());
    }

    #[test]
    #[ignore = "opt-in: writes and removes a unique test entry in the real OS credential store"]
    fn real_os_credential_store_round_trip() {
        let identity = uuid::Uuid::new_v4().to_string();
        let service = format!("com.kiselyovd.dungeonmasterai.test.{identity}");
        let store = OsCredentialStore::new(&service, &identity).unwrap();
        let secret = generate_credential();
        let mut write_attempted = false;
        let result = (|| {
            if store.read()?.is_some() {
                return Err("test_credential_already_exists");
            }
            write_attempted = true;
            store.write(&secret)?;
            if store.read()?.as_deref() != Some(secret.as_str()) {
                return Err("test_credential_readback_failed");
            }
            Ok::<_, &'static str>(())
        })();
        let cleanup = if write_attempted {
            match store.entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(_) => Err("test_credential_cleanup_failed"),
            }
        } else {
            Ok(())
        };
        assert!(result.is_ok());
        assert!(cleanup.is_ok());
    }
}
