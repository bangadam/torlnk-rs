use librqbit::{AddTorrent, AddTorrentOptions, ManagedTorrent, Session};
use librqbit::api::TorrentIdOrHash;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Progress stats for a torrent, polled periodically.
#[derive(Debug, Clone, Default)]
pub struct TorrentProgress {
    pub progress: f64,
    pub downloaded: u64,
    pub total: u64,
    pub speed: u64,
    pub upload_speed: u64,
    pub uploaded: u64,
    pub peers: u32,
    pub time_remaining: u64,
    pub name: String,
}

/// Metadata captured when a torrent's metadata arrives.
#[derive(Debug, Clone, Default)]
pub struct TorrentMeta {
    pub name: String,
    pub total: u64,
    pub files: u32,
}

pub struct AddHandlers {
    pub on_metadata: Option<Box<dyn Fn(TorrentMeta) + Send + Sync>>,
    pub on_done: Option<Box<dyn Fn() + Send + Sync>>,
    pub on_error: Option<Box<dyn Fn(String) + Send + Sync>>,
}

/// Adapter wrapping librqbit's async Session.
/// The session's default output folder is a temp dir; each torrent sets its own
/// via AddTorrentOptions.output_folder.
pub struct TorrentEngine {
    session: Arc<Session>,
    torrents: Arc<Mutex<HashMap<String, Arc<ManagedTorrent>>>>,
}

impl TorrentEngine {
    pub async fn new() -> anyhow::Result<Self> {
        let default_dir = std::env::temp_dir().join("torlnk-engine");
        tokio::fs::create_dir_all(&default_dir).await.ok();
        let session = Session::new(default_dir).await?;
        Ok(Self {
            session,
            torrents: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Add a torrent by magnet URI, info hash, or .torrent file path.
    /// `announce` is unused — trackers come from the magnet URI itself.
    pub async fn add(
        &self,
        id: &str,
        source: &str,
        dir: &str,
        handlers: AddHandlers,
        _announce: Option<Vec<String>>,
    ) {
        // Remove existing torrent with this id if present
        {
            let mut torrents = self.torrents.lock().await;
            if let Some(handle) = torrents.remove(id) {
                let _ = self.session.delete(
                    TorrentIdOrHash::Hash(handle.info_hash()),
                    false,
                ).await;
            }
        }

        let mut opts = AddTorrentOptions::default();
        opts.output_folder = Some(dir.to_string());
        opts.overwrite = true;

        let add = match AddTorrent::from_cli_argument(source) {
            Ok(a) => a,
            Err(e) => {
                if let Some(cb) = handlers.on_error {
                    cb(e.to_string());
                }
                return;
            }
        };

        let response = match self.session.add_torrent(add, Some(opts)).await {
            Ok(r) => r,
            Err(e) => {
                if let Some(cb) = handlers.on_error {
                    cb(e.to_string());
                }
                return;
            }
        };

        let handle = match response.into_handle() {
            Some(h) => h,
            None => return,
        };

        self.torrents.lock().await.insert(id.to_string(), handle.clone());

        let handle_clone = handle.clone();
        let id_clone = id.to_string();
        let on_metadata = handlers.on_metadata;
        let on_done = handlers.on_done;
        let on_error = handlers.on_error;
        let torrents = self.torrents.clone();

        tokio::spawn(async move {
            // Wait for metadata
            match handle_clone.wait_until_initialized().await {
                Ok(()) => {
                    let name = handle_clone.name().unwrap_or_default();
                    let stats = handle_clone.stats();
                    let total = stats.total_bytes;
                    let files = stats.file_progress.len() as u32;
                    if let Some(cb) = &on_metadata {
                        cb(TorrentMeta { name, total, files });
                    }
                }
                Err(e) => {
                    if let Some(cb) = &on_error {
                        cb(e.to_string());
                    }
                    torrents.lock().await.remove(&id_clone);
                    return;
                }
            }

            // Wait for completion
            match handle_clone.wait_until_completed().await {
                Ok(()) => {
                    if let Some(cb) = &on_done {
                        cb();
                    }
                }
                Err(e) => {
                    if let Some(cb) = &on_error {
                        cb(e.to_string());
                    }
                }
            }
        });
    }

    /// Poll stats for a torrent by our id.
    pub async fn stats(&self, id: &str) -> Option<TorrentProgress> {
        let handle = {
            let torrents = self.torrents.lock().await;
            torrents.get(id).cloned()?
        };

        let stats = handle.stats();
        let name = handle.name().unwrap_or_default();
        let total = stats.total_bytes;
        let downloaded = stats.progress_bytes;

        let progress = if total > 0 {
            downloaded as f64 / total as f64
        } else {
            0.0
        };

        let (speed, upload_speed, uploaded, peers, time_remaining) =
            if let Some(live) = stats.live {
                let ds = (live.download_speed.mbps * 1_000_000.0) as u64;
                let us = (live.upload_speed.mbps * 1_000_000.0) as u64;
                let uploaded = stats.uploaded_bytes;
                let peers = live.snapshot.peer_stats.live as u32;
                let tr = if ds > 0 && total > downloaded {
                    ((total - downloaded) as f64 / ds as f64) as u64
                } else {
                    0
                };
                (ds, us, uploaded, peers, tr)
            } else {
                (0, 0, 0, 0, 0)
            };

        Some(TorrentProgress {
            progress,
            downloaded,
            total,
            speed,
            upload_speed,
            uploaded,
            peers,
            time_remaining,
            name,
        })
    }

    /// Remove a torrent from the engine.
    pub async fn remove(&self, id: &str) {
        let handle = {
            let mut torrents = self.torrents.lock().await;
            torrents.remove(id)
        };
        if let Some(handle) = handle {
            let _ = self.session
                .delete(TorrentIdOrHash::Hash(handle.info_hash()), false)
                .await;
        }
    }

    /// Pause a torrent.
    pub async fn pause(&self, id: &str) -> anyhow::Result<()> {
        let handle = {
            let torrents = self.torrents.lock().await;
            torrents.get(id).cloned()
        };
        if let Some(h) = handle {
            self.session.pause(&h).await?;
        }
        Ok(())
    }

    /// Unpause a torrent.
    pub async fn unpause(&self, id: &str) -> anyhow::Result<()> {
        let handle = {
            let torrents = self.torrents.lock().await;
            torrents.get(id).cloned()
        };
        if let Some(h) = handle {
            self.session.unpause(&h).await?;
        }
        Ok(())
    }

    /// Get the TCP listen port for diagnostics.
    pub fn listen_port(&self) -> Option<u16> {
        self.session.tcp_listen_port()
    }

    /// Stop the session and all managed tasks.
    pub async fn destroy(&self) {
        self.session.stop().await;
    }
}
