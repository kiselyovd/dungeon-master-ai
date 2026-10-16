use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;

use sqlx::sqlite::SqliteConnectOptions;
use sqlx::ConnectOptions;

pub(super) struct BackendProfile {
    pub(super) database_url: String,
    pub(super) vault_environment: Option<String>,
    pub(super) vault_path: Option<PathBuf>,
    database_directory: Option<PathBuf>,
}

impl BackendProfile {
    pub(super) fn resolve(
        executable: &Path,
        app_data: &Path,
        working_directory: &Path,
        database_override: Option<OsString>,
        vault_override: Option<OsString>,
    ) -> Result<Self, &'static str> {
        let database_override = environment_text(database_override)?;
        let vault_override = environment_text(vault_override)?;
        let (database_url, inferred_vault, database_directory) =
            if let Some(database) = database_override {
                if !database.starts_with("sqlite:") {
                    return Err("backend_database_url_invalid");
                }
                let options = SqliteConnectOptions::from_str(&database)
                    .map_err(|_| "backend_database_url_invalid")?;
                let filename = options.get_filename();
                let database_name = database
                    .trim_start_matches("sqlite://")
                    .trim_start_matches("sqlite:")
                    .split('?')
                    .next()
                    .unwrap_or_default();
                let vault = if database_name == ":memory:" {
                    None
                } else if filename.as_os_str().is_empty() {
                    Some(normalize_path(
                        &working_directory.join("dmai-vault.hold"),
                        working_directory,
                    )?)
                } else {
                    let database_path = normalize_path(filename, working_directory)?;
                    Some(database_path.with_file_name("dmai-vault.hold"))
                };
                let vault = if vault_override.is_none() {
                    preserve_explicit_url_vault(&database, vault, working_directory)?
                } else {
                    vault
                };
                (database, vault, None)
            } else {
                let legacy = normalize_path(
                    executable
                        .parent()
                        .ok_or("backend_executable_path_invalid")?,
                    working_directory,
                )?;
                let managed = normalize_path(&app_data.join("backend"), working_directory)?;
                let legacy_exists = profile_exists(&legacy)?;
                let managed_exists = profile_exists(&managed)?;
                if !same_path(&legacy, &managed) && legacy_exists && managed_exists {
                    return Err("backend_profile_ambiguous_set_database_url");
                }
                let directory = if legacy_exists { legacy } else { managed };
                let database = directory.join("data.db");
                let database_url = SqliteConnectOptions::new()
                    .filename(&database)
                    .create_if_missing(true)
                    .to_url_lossy()
                    .to_string();
                (
                    database_url,
                    Some(directory.join("dmai-vault.hold")),
                    Some(directory),
                )
            };
        let (vault_environment, vault_path) = match vault_override {
            Some(explicit) => {
                let path = normalize_path(Path::new(&explicit), working_directory)?;
                (Some(explicit), Some(path))
            }
            None => {
                let environment = inferred_vault
                    .as_ref()
                    .map(|path| {
                        path.to_str()
                            .map(str::to_owned)
                            .ok_or("backend_profile_path_invalid")
                    })
                    .transpose()?;
                (environment, inferred_vault)
            }
        };
        if let Some(vault) = vault_path.as_ref() {
            let frontend = normalize_path(&app_data.join("dmai-vault.hold"), working_directory)?;
            if frontend_vault_collision(vault, &frontend) {
                return Err("backend_vault_frontend_collision");
            }
            snapshot_exists(vault)?;
        }
        Ok(Self {
            database_url,
            vault_environment,
            vault_path,
            database_directory,
        })
    }

    pub(super) fn prepare_database_directory(&self) -> Result<(), &'static str> {
        if let Some(directory) = self.database_directory.as_ref() {
            std::fs::create_dir_all(directory)
                .map_err(|_| "backend_profile_directory_unavailable")?;
        }
        Ok(())
    }
}

fn environment_text(value: Option<OsString>) -> Result<Option<String>, &'static str> {
    value
        .map(|value| {
            let value = value
                .into_string()
                .map_err(|_| "backend_profile_environment_invalid")?;
            if value.is_empty() || value.contains('\0') {
                return Err("backend_profile_environment_invalid");
            }
            Ok(value)
        })
        .transpose()
}

fn profile_exists(directory: &Path) -> Result<bool, &'static str> {
    let database = snapshot_exists(&directory.join("data.db"))?;
    let vault = snapshot_exists(&directory.join("dmai-vault.hold"))?;
    Ok(database || vault)
}

fn preserve_explicit_url_vault(
    database: &str,
    semantic: Option<PathBuf>,
    working_directory: &Path,
) -> Result<Option<PathBuf>, &'static str> {
    let legacy = database
        .strip_prefix("sqlite://")
        .filter(|path| !path.is_empty() && !path.starts_with(':'))
        .map(|path| {
            let database = Path::new(path);
            normalize_path(
                &database
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join("dmai-vault.hold"),
                working_directory,
            )
        })
        .transpose()?;
    match (semantic, legacy) {
        (Some(semantic), Some(legacy)) if !same_path(&semantic, &legacy) => {
            let old_exists = snapshot_exists(&legacy)?;
            let new_exists = snapshot_exists(&semantic)?;
            if old_exists && new_exists {
                Err("backend_vault_ambiguous_set_vault_path")
            } else {
                Ok(Some(if old_exists { legacy } else { semantic }))
            }
        }
        (semantic, _) => Ok(semantic),
    }
}

pub(super) fn snapshot_exists(path: &Path) -> Result<bool, &'static str> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => std::fs::metadata(path)
            .map_err(|_| "backend_profile_metadata_unavailable")
            .and_then(|metadata| {
                if metadata.is_file() {
                    Ok(true)
                } else {
                    Err("backend_profile_not_a_file")
                }
            }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err("backend_profile_metadata_unavailable"),
    }
}

pub(super) fn normalize_path(
    path: &Path,
    working_directory: &Path,
) -> Result<PathBuf, &'static str> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        working_directory.join(path)
    };
    if !absolute.is_absolute() {
        return Err("backend_profile_path_invalid");
    }
    let mut ancestor = absolute.as_path();
    let canonical = loop {
        match std::fs::canonicalize(ancestor) {
            Ok(canonical) => break canonical,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                ancestor = ancestor.parent().ok_or("backend_profile_path_invalid")?;
            }
            Err(_) => return Err("backend_profile_metadata_unavailable"),
        }
    };
    let suffix = absolute
        .strip_prefix(ancestor)
        .map_err(|_| "backend_profile_path_invalid")?;
    let mut normalized = ordinary_path(canonical);
    for component in suffix.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err("backend_profile_path_invalid");
                }
            }
            Component::Normal(part) => normalized.push(part),
            _ => return Err("backend_profile_path_invalid"),
        }
    }
    Ok(normalized)
}

fn frontend_vault_collision(vault: &Path, frontend: &Path) -> bool {
    if same_path(vault, frontend) {
        return true;
    }
    // Reserve spelling aliases before either snapshot exists on macOS volumes.
    #[cfg(target_os = "macos")]
    if let (Some(vault), Some(frontend)) = (vault.to_str(), frontend.to_str()) {
        return vault.to_lowercase() == frontend.to_lowercase();
    }
    false
}

fn same_path(left: &Path, right: &Path) -> bool {
    #[cfg(windows)]
    {
        match (left.to_str(), right.to_str()) {
            (Some(left), Some(right)) => left.to_lowercase() == right.to_lowercase(),
            _ => left == right,
        }
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

#[cfg(not(windows))]
fn ordinary_path(path: PathBuf) -> PathBuf {
    path
}

#[cfg(windows)]
fn ordinary_path(path: PathBuf) -> PathBuf {
    match path.to_str() {
        Some(value) if value.starts_with(r"\\?\UNC\") => {
            PathBuf::from(format!(r"\\{}", &value[8..]))
        }
        Some(value) if value.starts_with(r"\\?\") => PathBuf::from(&value[4..]),
        _ => path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fixture() -> TempDir {
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir(directory.path().join("install")).unwrap();
        std::fs::create_dir(directory.path().join("app-data")).unwrap();
        directory
    }

    fn root(directory: &TempDir) -> PathBuf {
        ordinary_path(directory.path().canonicalize().unwrap())
    }

    fn resolve(
        directory: &TempDir,
        database: Option<&str>,
        vault: Option<&str>,
    ) -> Result<BackendProfile, &'static str> {
        BackendProfile::resolve(
            &directory.path().join("install/desktop"),
            &directory.path().join("app-data"),
            directory.path(),
            database.map(Into::into),
            vault.map(Into::into),
        )
    }

    fn write(directory: &TempDir, path: &str) {
        let path = directory.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"preserve-existing-bytes").unwrap();
    }

    #[test]
    fn clean_install_selects_separate_writable_backend_profile() {
        let directory = fixture();
        write(&directory, "app-data/dmai-vault.hold");
        let profile = resolve(&directory, None, None).unwrap();
        let options = SqliteConnectOptions::from_str(&profile.database_url).unwrap();
        assert_eq!(
            options.get_filename(),
            root(&directory).join("app-data/backend/data.db")
        );
        assert!(profile.database_url.contains("mode=rwc"));
        assert_eq!(
            profile.vault_path,
            Some(root(&directory).join("app-data/backend/dmai-vault.hold"))
        );
        assert!(!directory
            .path()
            .join("app-data/backend")
            .try_exists()
            .unwrap());
        assert_eq!(
            std::fs::read(directory.path().join("app-data/dmai-vault.hold")).unwrap(),
            b"preserve-existing-bytes"
        );
    }

    #[tokio::test]
    async fn managed_database_round_trips_with_spaces_and_cyrillic_in_profile_path() {
        let directory = fixture();
        let profile = BackendProfile::resolve(
            &directory.path().join("install/desktop"),
            &directory.path().join("Данные игрока"),
            directory.path(),
            None,
            None,
        )
        .unwrap();
        profile.prepare_database_directory().unwrap();

        let pool = sqlx::SqlitePool::connect(&profile.database_url)
            .await
            .unwrap();
        sqlx::query("CREATE TABLE profile_round_trip (value TEXT NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO profile_round_trip (value) VALUES (?)")
            .bind("Сохранённый тестовый профиль")
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;

        let reopened = sqlx::SqlitePool::connect(&profile.database_url)
            .await
            .unwrap();
        let value: String = sqlx::query_scalar("SELECT value FROM profile_round_trip")
            .fetch_one(&reopened)
            .await
            .unwrap();
        assert_eq!(value, "Сохранённый тестовый профиль");
        reopened.close().await;
    }

    #[test]
    fn environment_credential_still_prepares_the_default_database_directory() {
        let directory = fixture();
        let profile = resolve(&directory, None, Some("separate/vault.hold")).unwrap();
        let credential = super::super::backend_credentials::resolve(
            profile.vault_path.as_deref(),
            Some("operator-secret".into()),
        )
        .unwrap()
        .unwrap();
        assert_eq!(credential.as_str(), "operator-secret");
        assert!(!directory
            .path()
            .join("app-data/backend")
            .try_exists()
            .unwrap());
        profile.prepare_database_directory().unwrap();
        assert!(directory.path().join("app-data/backend").is_dir());
        assert!(!directory
            .path()
            .join("app-data/backend/data.db")
            .try_exists()
            .unwrap());
        assert!(!directory.path().join("separate").try_exists().unwrap());
    }

    #[test]
    fn explicit_database_directory_is_not_created_by_native_profile_preparation() {
        let directory = fixture();
        let profile =
            resolve(&directory, Some("sqlite://operator/data.db?mode=rwc"), None).unwrap();
        profile.prepare_database_directory().unwrap();
        assert!(!directory.path().join("operator").try_exists().unwrap());
    }

    #[test]
    fn either_legacy_file_keeps_the_legacy_profile() {
        for filename in ["data.db", "dmai-vault.hold"] {
            let directory = fixture();
            write(&directory, &format!("install/{filename}"));
            let profile = resolve(&directory, None, None).unwrap();
            assert_eq!(
                profile.vault_path,
                Some(root(&directory).join("install/dmai-vault.hold"))
            );
            assert!(!directory
                .path()
                .join("app-data/backend")
                .try_exists()
                .unwrap());
        }
    }

    #[test]
    fn competing_profiles_require_an_explicit_database_override() {
        let directory = fixture();
        write(&directory, "install/data.db");
        write(&directory, "app-data/backend/dmai-vault.hold");
        assert_eq!(
            resolve(&directory, None, None).err(),
            Some("backend_profile_ambiguous_set_database_url")
        );
        let database = format!(
            "sqlite://{}?mode=rw",
            directory.path().join("install/data.db").display()
        );
        assert_eq!(
            resolve(&directory, Some(&database), None)
                .unwrap()
                .database_url,
            database
        );
    }

    #[test]
    fn explicit_relative_database_and_vault_values_are_preserved() {
        let directory = fixture();
        let profile = resolve(
            &directory,
            Some("sqlite://relative/data.db?mode=rwc&cache=shared"),
            Some("custom/../custom/vault.hold"),
        )
        .unwrap();
        assert_eq!(
            profile.database_url,
            "sqlite://relative/data.db?mode=rwc&cache=shared"
        );
        assert_eq!(
            profile.vault_environment.as_deref(),
            Some("custom/../custom/vault.hold")
        );
        assert_eq!(
            profile.vault_path,
            Some(root(&directory).join("custom/vault.hold"))
        );
    }

    #[test]
    fn encoded_database_url_keeps_an_existing_legacy_vault() {
        let directory = fixture();
        write(&directory, "profile%20space/dmai-vault.hold");
        let database = format!(
            "sqlite://{}/profile%20space/data.db?mode=rwc",
            directory.path().display()
        );
        let profile = resolve(&directory, Some(&database), None).unwrap();
        assert_eq!(profile.database_url, database);
        assert_eq!(
            profile.vault_path,
            Some(root(&directory).join("profile%20space/dmai-vault.hold"))
        );
        assert!(!directory.path().join("profile space").try_exists().unwrap());
    }

    #[test]
    fn encoded_database_url_with_two_existing_vaults_is_ambiguous() {
        let directory = fixture();
        write(&directory, "profile%20space/dmai-vault.hold");
        write(&directory, "profile space/dmai-vault.hold");
        let database = format!(
            "sqlite://{}/profile%20space/data.db?mode=rwc",
            directory.path().display()
        );
        assert_eq!(
            resolve(&directory, Some(&database), None).err(),
            Some("backend_vault_ambiguous_set_vault_path")
        );
    }

    #[test]
    fn frontend_vault_is_rejected_even_when_explicit() {
        let directory = fixture();
        assert_eq!(
            resolve(&directory, None, Some("app-data/dmai-vault.hold")).err(),
            Some("backend_vault_frontend_collision")
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn frontend_case_alias_is_reserved_before_the_snapshot_is_created() {
        let directory = fixture();
        assert_eq!(
            resolve(&directory, None, Some("app-data/DMAI-VAULT.HOLD")).err(),
            Some("backend_vault_frontend_collision")
        );
        assert!(!directory
            .path()
            .join("app-data/dmai-vault.hold")
            .try_exists()
            .unwrap());
    }

    #[test]
    fn explicit_memory_database_has_no_automatic_persistent_vault() {
        let directory = fixture();
        let profile = resolve(&directory, Some("sqlite::memory:"), None).unwrap();
        assert_eq!(profile.database_url, "sqlite::memory:");
        assert!(profile.vault_path.is_none());
        assert!(resolve(
            &directory,
            Some("sqlite::memory:"),
            Some("custom/vault.hold")
        )
        .unwrap()
        .vault_path
        .is_some());
    }

    #[test]
    fn query_only_memory_url_keeps_its_legacy_cwd_vault_location() {
        let directory = fixture();
        let profile = resolve(&directory, Some("sqlite://?mode=memory"), None).unwrap();
        assert_eq!(profile.database_url, "sqlite://?mode=memory");
        assert_eq!(
            profile.vault_path,
            Some(root(&directory).join("dmai-vault.hold"))
        );
    }

    #[test]
    fn invalid_database_or_empty_override_fails_closed() {
        let directory = fixture();
        assert!(resolve(&directory, Some("postgres://localhost/example"), None).is_err());
        assert!(resolve(&directory, Some(""), None).is_err());
        assert!(resolve(&directory, None, Some("")).is_err());
    }

    #[test]
    fn metadata_errors_are_not_treated_as_missing_profiles() {
        let directory = fixture();
        write(&directory, "blocked");
        assert!(resolve(&directory, None, Some("blocked/vault.hold")).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_alias_cannot_select_the_frontend_vault() {
        let directory = fixture();
        write(&directory, "app-data/dmai-vault.hold");
        std::os::unix::fs::symlink(
            directory.path().join("app-data/dmai-vault.hold"),
            directory.path().join("alias.hold"),
        )
        .unwrap();
        assert_eq!(
            resolve(&directory, None, Some("alias.hold")).err(),
            Some("backend_vault_frontend_collision")
        );
    }

    #[cfg(windows)]
    #[test]
    fn frontend_collision_is_case_insensitive_on_windows() {
        let directory = fixture();
        assert_eq!(
            resolve(&directory, None, Some("app-data/DMAI-VAULT.HOLD")).err(),
            Some("backend_vault_frontend_collision")
        );
    }
}
