use crate::config::paths;
use std::path::Path;

/// How long after restore the process must survive before the marker is disarmed.
pub const BOOT_SETTLE_MS: u64 = 4000;

pub fn was_boot_interrupted() -> bool {
    paths::boot_marker_file().exists()
}

pub fn arm_boot_marker() {
    let path = paths::boot_marker_file();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let content = serde_json::json!({
        "at": chrono::Utc::now().timestamp_millis(),
        "pid": std::process::id(),
    });
    std::fs::write(&path, content.to_string()).ok();
}

pub fn disarm_boot_marker() {
    let path = paths::boot_marker_file();
    std::fs::remove_file(&path).ok();
}
