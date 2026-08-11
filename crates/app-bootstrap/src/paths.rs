//! Stable backend data paths, independent of the launching shell's cwd.

use std::path::{Path, PathBuf};

pub fn default_db_path_for_executable(executable: &Path) -> PathBuf {
    executable
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("data.db")
}

pub fn default_db_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .as_deref()
        .map(default_db_path_for_executable)
        .unwrap_or_else(|| PathBuf::from("data.db"))
}

pub fn vault_path() -> Option<PathBuf> {
    if let Ok(explicit) = std::env::var("DMAI_VAULT_PATH") {
        return Some(PathBuf::from(explicit));
    }
    let db_path = match std::env::var("DATABASE_URL") {
        Ok(db_url) => {
            let path = db_url.strip_prefix("sqlite://")?;
            if path.starts_with(':') || path.is_empty() {
                return None;
            }
            PathBuf::from(path)
        }
        Err(_) => default_db_path(),
    };
    Some(
        db_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("dmai-vault.hold"),
    )
}

#[cfg(test)]
mod tests {
    use super::default_db_path_for_executable;
    use std::path::Path;

    #[test]
    fn windows_executable_keeps_data_beside_binary() {
        assert_eq!(
            default_db_path_for_executable(Path::new(r"C:\Program Files\DMAI\app-server.exe")),
            Path::new(r"C:\Program Files\DMAI").join("data.db")
        );
    }

    #[test]
    fn unix_executable_keeps_data_beside_binary() {
        assert_eq!(
            default_db_path_for_executable(Path::new("/opt/dmai/app-server")),
            Path::new("/opt/dmai/data.db")
        );
    }
}
