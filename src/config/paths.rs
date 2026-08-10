use std::path::{Path, PathBuf};

pub const APP_NAME: &str = "torlnk-rs";

/// Optional override that relocates all persisted state under one folder.
/// Tests point this at a temp dir; doubles as a portable-state escape hatch.
fn state_dir_override() -> Option<PathBuf> {
    std::env::var("TORLINK_STATE_DIR").ok().map(PathBuf::from)
}

pub fn config_dir() -> PathBuf {
    if let Some(override_dir) = state_dir_override() {
        return override_dir.join("config");
    }
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_NAME)
}

pub fn data_dir() -> PathBuf {
    if let Some(override_dir) = state_dir_override() {
        return override_dir.join("data");
    }
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_NAME)
}

pub fn default_download_dir() -> PathBuf {
    dirs::download_dir()
        .or_else(|| dirs::home_dir())
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_NAME)
}

pub fn config_file() -> PathBuf {
    config_dir().join("config.json")
}

pub fn queue_file() -> PathBuf {
    data_dir().join("queue.json")
}

pub fn history_file() -> PathBuf {
    data_dir().join("history.json")
}

pub fn seeds_file() -> PathBuf {
    data_dir().join("seeds.json")
}

/// Per-torrent .torrent metadata, captured during download so a re-seed can
/// verify the on-disk file locally instead of re-fetching from the swarm.
pub fn torrents_dir() -> PathBuf {
    data_dir().join("torrents")
}

pub fn torrent_meta_path(id: &str) -> PathBuf {
    torrents_dir().join(format!("{}.torrent", id))
}

/// Armed just before boot hands saved state to the torrent engine, disarmed
/// once the boot settles; see download/bootguard.rs.
pub fn boot_marker_file() -> PathBuf {
    data_dir().join("boot.marker")
}

/// Where a --daemon headless run writes its log and pidfile.
pub fn logs_dir() -> PathBuf {
    data_dir().join("logs")
}

pub fn ensure_data_dir() -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir())
}

pub fn ensure_torrents_dir() -> std::io::Result<()> {
    std::fs::create_dir_all(torrents_dir())
}

pub fn torrent_meta_exists(id: &str) -> bool {
    torrent_meta_path(id).exists()
}

/// Sanitized export name for a .torrent file.
pub fn torrent_export_name(name: &str, id: &str) -> String {
    let base: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '*' | '?' => ' ',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let trimmed = base.trim_end_matches(|c: char| c == '.' || c == ' ');
    let final_name = if trimmed.is_empty() { id } else { trimmed };
    let truncated: String = final_name.chars().take(180).collect();
    format!("{}.torrent", truncated)
}
