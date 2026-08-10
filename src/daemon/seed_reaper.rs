/// Seed reaper: removes seeds that have been idle (no peers, no upload)
/// for longer than a grace period. Runs as a periodic task in headless mode.

use crate::download::queue::DownloadQueue;
use std::sync::Arc;
use std::time::Duration;

const REAP_INTERVAL_MS: u64 = 60_000;
const IDLE_GRACE_MS: i64 = 86_400_000; // 24h

pub async fn run_seed_reaper(queue: Arc<DownloadQueue>) {
    loop {
        tokio::time::sleep(Duration::from_millis(REAP_INTERVAL_MS)).await;

        let seeds = queue.get_seeds().await;
        let now = chrono::Utc::now().timestamp_millis();

        for seed in &seeds {
            if seed.upload_speed == 0 && seed.peers == 0 {
                // Check age via history
                let history = queue.get_history().await;
                if let Some(h) = history.iter().find(|h| h.id == seed.id) {
                    if now - h.completed_at > IDLE_GRACE_MS {
                        tracing::info!("seed reaper: removing idle seed {}", seed.name);
                        queue.remove(&seed.id, false).await;
                    }
                }
            }
        }
    }
}
