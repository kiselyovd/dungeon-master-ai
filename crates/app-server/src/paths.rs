//! Platform-specific app data dir resolution.
//!
//! Windows: %APPDATA%\dungeon-master-ai\
//! macOS:   ~/Library/Application Support/dungeon-master-ai/
//! Linux:   ~/.local/share/dungeon-master-ai/

use std::path::PathBuf;

pub const APP_NAME: &str = "dungeon-master-ai";

/// Returns the canonical app data directory, creating parent path segments
/// lazily on first write. Falls back to `./` when no data dir is available
/// (unlikely; only happens on stripped-down container environments).
pub fn app_data_dir() -> PathBuf {
    dirs::data_dir()
        .map(|d| d.join(APP_NAME))
        .unwrap_or_else(|| PathBuf::from("."))
}
