use crate::config::paths;
use crate::download::types::HistoryItem;

pub const HISTORY_CAP: usize = 500;

pub async fn save_history(items: &[HistoryItem]) -> anyhow::Result<()> {
    let capped: Vec<_> = items.iter().take(HISTORY_CAP).cloned().collect();
    let json = serde_json::to_string_pretty(&capped)?;
    let path = paths::history_file();
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    let tmp = path.with_extension("tmp");
    tokio::fs::write(&tmp, &json).await?;
    tokio::fs::rename(&tmp, &path).await?;
    Ok(())
}

pub fn save_history_sync(items: &[HistoryItem]) {
    let capped: Vec<_> = items.iter().take(HISTORY_CAP).cloned().collect();
    if let Ok(json) = serde_json::to_string_pretty(&capped) {
        let path = paths::history_file();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, &json).is_ok() {
            std::fs::rename(&tmp, &path).ok();
        }
    }
}

fn is_valid_history_item(v: &serde_json::Value) -> bool {
    v.get("id").and_then(|v| v.as_str()).is_some()
        && v.get("name").and_then(|v| v.as_str()).is_some()
        && v.get("magnet").and_then(|v| v.as_str()).is_some()
}

pub async fn load_history() -> Vec<HistoryItem> {
    let path = paths::history_file();
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
        .filter(|v| is_valid_history_item(v))
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .take(HISTORY_CAP)
        .collect()
}
