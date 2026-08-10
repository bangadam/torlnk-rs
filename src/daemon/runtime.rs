use crate::config::config::Config;
use crate::download::engine::TorrentEngine;
use crate::download::queue::DownloadQueue;
use std::sync::Arc;

/// Shared runtime state for headless modes. Owns the engine + queue.
pub struct HeadlessRuntime {
    pub engine: Arc<TorrentEngine>,
    pub queue: Arc<DownloadQueue>,
    pub config: Config,
}

impl HeadlessRuntime {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        let engine = Arc::new(TorrentEngine::new().await?);
        let queue = Arc::new(DownloadQueue::new(engine.clone()));

        let trackers = crate::config::trackers::parse_trackers(&config.trackers.join(","));
        queue.set_trackers(trackers);

        Ok(Self { engine, queue, config })
    }

    pub async fn tick_loop(&self) {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(
                crate::download::queue::POLL_MS,
            )).await;
            self.queue.tick().await;
        }
    }
}
