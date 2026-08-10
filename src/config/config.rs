use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::config::paths;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub download_dir: String,
    pub trackers: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            download_dir: paths::default_download_dir().to_string_lossy().to_string(),
            trackers: vec![],
        }
    }
}

pub async fn load_config() -> Config {
    let path = paths::config_file();
    let raw = match tokio::fs::read_to_string(&path).await {
        Ok(s) => s,
        Err(_) => return Config::default(),
    };
    serde_json::from_str::<Config>(&raw).unwrap_or_default()
}

pub async fn save_config(config: &Config) -> anyhow::Result<()> {
    let path = paths::config_file();
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    let json = serde_json::to_string_pretty(config)?;
    let tmp = path.with_extension("json.tmp");
    tokio::fs::write(&tmp, json).await?;
    tokio::fs::rename(&tmp, &path).await?;
    Ok(())
}

/// Synchronous save for quit-time flush.
pub fn save_config_sync(config: &Config) {
    let path = paths::config_file();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    if let Ok(json) = serde_json::to_string_pretty(config) {
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, json).is_ok() {
            std::fs::rename(&tmp, &path).ok();
        }
    }
}

pub fn normalize_download_dir(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let path = PathBuf::from(trimmed);
    path.to_string_lossy().to_string()
}
