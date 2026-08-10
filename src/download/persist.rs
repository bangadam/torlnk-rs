use crate::config::paths;
use crate::download::types::{HistoryItem, QueueItem, SeedRecord};
use std::path::{Path, PathBuf};

/// Atomic JSON write: write to tmp, then rename.
async fn write_json_atomic(path: &Path, json: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    let tmp = path.with_extension("tmp");
    tokio::fs::write(&tmp, json).await?;
    tokio::fs::rename(&tmp, path).await?;
    Ok(())
}

/// Synchronous atomic JSON write for quit-time flush.
fn write_json_sync(path: &Path, json: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let tmp = path.with_extension("tmp");
    if std::fs::write(&tmp, json).is_ok() {
        std::fs::rename(&tmp, path).ok();
    }
}

pub async fn save_queue(items: &[QueueItem]) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(items)?;
    write_json_atomic(&paths::queue_file(), &json).await
}

pub fn save_queue_sync(items: &[QueueItem]) {
    if let Ok(json) = serde_json::to_string_pretty(items) {
        write_json_sync(&paths::queue_file(), &json);
    }
}

fn is_valid_queue_item(v: &serde_json::Value) -> bool {
    v.get("id").and_then(|v| v.as_str()).is_some()
        && v.get("magnet").and_then(|v| v.as_str()).is_some()
}

pub async fn load_queue() -> Vec<QueueItem> {
    let path = paths::queue_file();
    let raw = match tokio::fs::read_to_string(&path).await {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    let parsed: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let arr = match parsed.as_array() {
        Some(a) => a,
        None => return vec![],
    };
    arr.iter()
        .filter(|v| is_valid_queue_item(v))
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect()
}

pub async fn save_seeds(records: &[SeedRecord]) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(records)?;
    write_json_atomic(&paths::seeds_file(), &json).await
}

pub fn save_seeds_sync(records: &[SeedRecord]) {
    if let Ok(json) = serde_json::to_string_pretty(records) {
        write_json_sync(&paths::seeds_file(), &json);
    }
}

pub async fn load_seeds() -> Vec<SeedRecord> {
    let path = paths::seeds_file();
    let raw = match tokio::fs::read_to_string(&path).await {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    let parsed: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let arr = match parsed.as_array() {
        Some(a) => a,
        None => return vec![],
    };
    let mut out = vec![];
    for el in arr {
        // Legacy format: bare string id → treat as seeding
        if let Some(id) = el.as_str() {
            out.push(SeedRecord {
                id: id.to_string(),
                status: crate::download::types::PersistedSeedStatus::Seeding,
            });
        } else if let Some(id) = el.get("id").and_then(|v| v.as_str()) {
            let status = el
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("paused");
            let status = match status {
                "seeding" => crate::download::types::PersistedSeedStatus::Seeding,
                _ => crate::download::types::PersistedSeedStatus::Paused,
            };
            out.push(SeedRecord {
                id: id.to_string(),
                status,
            });
        }
    }
    out
}

/// Save .torrent metadata (binary) to the torrents cache dir.
pub async fn save_torrent_meta(id: &str, data: &[u8]) -> anyhow::Result<()> {
    paths::ensure_torrents_dir().ok();
    let path = paths::torrent_meta_path(id);
    let tmp = path.with_extension("torrent.tmp");
    tokio::fs::write(&tmp, data).await?;
    tokio::fs::rename(&tmp, &path).await?;
    Ok(())
}

/// Copy cached .torrent metadata into a target dir with a sanitized name.
pub async fn export_torrent_meta(id: &str, name: &str, dir: &str) -> Option<String> {
    let source = paths::torrent_meta_path(id);
    if !source.exists() {
        return None;
    }
    tokio::fs::create_dir_all(dir).await.ok()?;
    let export_name = paths::torrent_export_name(name, id);
    let target = PathBuf::from(dir).join(&export_name);
    tokio::fs::copy(&source, &target).await.ok()?;
    Some(target.to_string_lossy().to_string())
}

pub fn delete_torrent_meta(id: &str) {
    let path = paths::torrent_meta_path(id);
    std::fs::remove_file(&path).ok();
}
