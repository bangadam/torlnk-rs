use crate::download::types::{DownloadStatus, QueueItem};

/// Deduplicate items, drop completed ones, and normalize status.
/// A previous "downloading" becomes "downloading" again (will be re-started on
/// restore); "queued" stays "queued"; everything else keeps its status.
pub fn reconcile_queue(items: Vec<QueueItem>) -> Vec<QueueItem> {
    let mut seen = std::collections::HashSet::new();
    let mut out = vec![];
    for mut it in items {
        if it.id.is_empty() || seen.contains(&it.id) {
            continue;
        }
        seen.insert(it.id.clone());
        if it.status == DownloadStatus::Completed {
            continue;
        }
        // Normalize: zero live stats on restore
        it.speed = 0;
        it.peers = 0;
        it.eta = None;
        out.push(it);
    }
    out
}
