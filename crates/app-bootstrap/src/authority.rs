//! An OS-backed lock, not a lock-file-presence heuristic.
use anyhow::Context;
use sqlx::sqlite::SqliteConnectOptions;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub(crate) fn acquire_for_path(database: &Path) -> anyhow::Result<File> {
    let parent = database
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent).context("create backend authority directory")?;
    let parent = parent
        .canonicalize()
        .context("resolve backend authority directory")?;
    let filename = database
        .file_name()
        .context("backend database filename missing")?;
    let mut lock_name = filename.to_os_string();
    lock_name.push(".authority.lock");
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(parent.join(lock_name))
        .context("open backend authority lock")?;
    file.try_lock()
        .context("another backend owns this campaign database")?;
    Ok(file)
}

pub(crate) fn retain_for_process() -> anyhow::Result<bool> {
    let path: PathBuf = match std::env::var("DATABASE_URL") {
        Ok(url) => {
            if url.contains(":memory:")
                || url
                    .split('?')
                    .nth(1)
                    .is_some_and(|query| query.split('&').any(|pair| pair == "mode=memory"))
            {
                return Ok(false);
            }
            SqliteConnectOptions::from_str(&url)
                .context("parse backend database options")?
                .get_filename()
                .to_path_buf()
        }
        Err(_) => crate::paths::default_db_path(),
    };
    let file = acquire_for_path(&path)?;
    // Exactly one handle per backend. Keep authority until OS process exit,
    // including shutdown while spawned turn tasks are still winding down.
    std::mem::forget(file);
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn second_authority_is_rejected_until_real_handle_release() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("campaign.db");
        let first = acquire_for_path(&database).unwrap();
        assert!(acquire_for_path(&database).is_err());
        drop(first);
        assert!(acquire_for_path(&database).is_ok());
    }
}
