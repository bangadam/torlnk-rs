use crate::download::bootguard;
use crate::download::engine::{AddHandlers, TorrentEngine, TorrentProgress};
use crate::download::history::{save_history, save_history_sync, load_history, HISTORY_CAP};
use crate::download::persist::{
    delete_torrent_meta, export_torrent_meta, load_queue, load_seeds, save_queue, save_queue_sync,
    save_seeds, save_seeds_sync,
};
use crate::download::reconcile::reconcile_queue;
use crate::download::types::{
    DownloadStatus, HistoryItem, PersistedSeedStatus, QueueItem, SeedItem, SeedRecord, SeedStatus,
};
use crate::config::paths;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

pub const POLL_MS: u64 = 500;
const SEED_GRACE_MS: u64 = 10_000;
const STRAY_TICKS: u32 = 2;
const FETCH_METADATA_TIMEOUT_MS: u64 = 20_000;

/// A real seed never pulls data off the network: verifying on-disk files reads
/// the disk (network speed stays 0), only fetching *missing* data raises it.
/// So sustained network download on a "seed" means its files are gone/partial.
pub fn stray_download(s: &TorrentProgress) -> bool {
    s.total > 0 && s.progress < 1.0 && s.speed > 0
}

/// Read max concurrent downloads from env. 0 = unlimited.
fn read_max_downloads() -> usize {
    std::env::var("TORLINK_MAX_DOWNLOADS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(0)
}

#[derive(Debug, Clone)]
pub struct AddInput {
    pub id: String,
    pub name: String,
    pub magnet: String,
    pub source: Option<crate::sources::SourceId>,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct RestoreOptions {
    pub safe: bool,
}

impl Default for RestoreOptions {
    fn default() -> Self {
        Self { safe: false }
    }
}

/// Messages emitted by the queue to notify the UI/runtime.
#[derive(Debug, Clone)]
pub enum QueueEvent {
    Update,
    Completed(String),
    Failed(String),
}

pub struct DownloadQueue {
    items: Arc<Mutex<HashMap<String, QueueItem>>>,
    seeds: Arc<Mutex<HashMap<String, SeedItem>>>,
    history: Arc<Mutex<Vec<HistoryItem>>>,
    engine: Arc<TorrentEngine>,
    trackers: Arc<Mutex<Vec<String>>>,
    stray_hits: Arc<Mutex<HashMap<String, u32>>>,
    seed_started_at: Arc<Mutex<HashMap<String, i64>>>,
    max_downloads: usize,
    event_tx: mpsc::UnboundedSender<QueueEvent>,
}

impl DownloadQueue {
    pub fn new(engine: Arc<TorrentEngine>) -> Self {
        Self {
            items: Arc::new(Mutex::new(HashMap::new())),
            seeds: Arc::new(Mutex::new(HashMap::new())),
            history: Arc::new(Mutex::new(vec![])),
            engine,
            trackers: Arc::new(Mutex::new(vec![])),
            stray_hits: Arc::new(Mutex::new(HashMap::new())),
            seed_started_at: Arc::new(Mutex::new(HashMap::new())),
            max_downloads: read_max_downloads(),
            event_tx: mpsc::unbounded_channel().0,
        }
    }

    pub fn with_event_sender(engine: Arc<TorrentEngine>, tx: mpsc::UnboundedSender<QueueEvent>) -> Self {
        Self {
            items: Arc::new(Mutex::new(HashMap::new())),
            seeds: Arc::new(Mutex::new(HashMap::new())),
            history: Arc::new(Mutex::new(vec![])),
            engine,
            trackers: Arc::new(Mutex::new(vec![])),
            stray_hits: Arc::new(Mutex::new(HashMap::new())),
            seed_started_at: Arc::new(Mutex::new(HashMap::new())),
            max_downloads: read_max_downloads(),
            event_tx: tx,
        }
    }

    pub fn set_trackers(&self, trackers: Vec<String>) {
        if let Ok(mut t) = self.trackers.try_lock() {
            *t = trackers;
        }
    }

    fn emit(&self, event: QueueEvent) {
        let _ = self.event_tx.send(event);
    }

    fn changed(&self) {
        self.emit(QueueEvent::Update);
    }

    async fn active_count(&self) -> usize {
        let items = self.items.lock().await;
        items
            .values()
            .filter(|it| it.status == DownloadStatus::Downloading)
            .count()
    }

    async fn seeding_count(&self) -> usize {
        let seeds = self.seeds.lock().await;
        seeds.values().filter(|s| s.status == SeedStatus::Seeding).count()
    }

    pub async fn get_items(&self) -> Vec<QueueItem> {
        let items = self.items.lock().await;
        let mut out: Vec<QueueItem> = items.values().cloned().collect();
        out.sort_by(|a, b| b.added_at.cmp(&a.added_at));
        out
    }

    pub async fn get_seeds(&self) -> Vec<SeedItem> {
        let seeds = self.seeds.lock().await;
        seeds.values().cloned().collect()
    }

    pub async fn get_seed(&self, id: &str) -> Option<SeedItem> {
        let seeds = self.seeds.lock().await;
        seeds.get(id).cloned()
    }

    pub async fn get_history(&self) -> Vec<HistoryItem> {
        let history = self.history.lock().await;
        history.clone()
    }

    pub async fn has(&self, id: &str) -> bool {
        let items = self.items.lock().await;
        items.contains_key(id)
    }

    /// Synchronous snapshot of queue items (for UI rendering).
    pub fn get_items_sync(&self) -> Vec<QueueItem> {
        if let Ok(items) = self.items.try_lock() {
            let mut out: Vec<QueueItem> = items.values().cloned().collect();
            out.sort_by(|a, b| b.added_at.cmp(&a.added_at));
            out
        } else {
            vec![]
        }
    }

    /// Synchronous snapshot of seeds (for UI rendering).
    pub fn get_seeds_sync(&self) -> Vec<SeedItem> {
        if let Ok(seeds) = self.seeds.try_lock() {
            seeds.values().cloned().collect()
        } else {
            vec![]
        }
    }

    pub async fn add(&self, input: AddInput, dir: &str) {
        // If it's a seed, remove the seed first
        {
            let mut seeds = self.seeds.lock().await;
            if seeds.remove(input.id.as_str()).is_some() {
                self.engine.remove(&input.id).await;
                drop(seeds);
                self.persist_seeds().await;
            }
        }

        // Check if already exists and not failed
        {
            let items = self.items.lock().await;
            if let Some(existing) = items.get(&input.id) {
                if existing.status != DownloadStatus::Failed {
                    return;
                }
            }
        }

        let now = chrono::Utc::now().timestamp_millis();
        let item = QueueItem {
            id: input.id.clone(),
            name: input.name.clone(),
            source: input.source,
            magnet: input.magnet.clone(),
            dir: dir.to_string(),
            status: DownloadStatus::Downloading,
            progress: 0,
            total_bytes: input.size_bytes.unwrap_or(0),
            downloaded_bytes: 0,
            speed: 0,
            peers: 0,
            eta: None,
            files: None,
            error: None,
            added_at: now,
        };

        // Respect concurrency cap
        let start = self.max_downloads == 0 || self.active_count().await < self.max_downloads;
        let mut item = item;
        if !start {
            item.status = DownloadStatus::Queued;
        }

        self.items.lock().await.insert(item.id.clone(), item.clone());

        if start {
            self.start_engine(&item).await;
        }

        self.changed();
        self.persist().await;
    }

    async fn start_engine(&self, item: &QueueItem) {
        let trackers = self.trackers.lock().await.clone();
        let announce = if trackers.is_empty() { None } else { Some(trackers) };

        let id = item.id.clone();
        let items = self.items.clone();
        let seeds = self.seeds.clone();
        let stray_hits = self.stray_hits.clone();
        let seed_started_at = self.seed_started_at.clone();
        let history = self.history.clone();

        let handlers = AddHandlers {
            on_metadata: Some(Box::new({
                let id = id.clone();
                move |meta| {
                    let id_clone = id.clone();
                    let meta_name = meta.name.clone();
                    tokio::spawn(async move {
                        let _ = (id_clone, meta_name);
                    });
                }
            })),
            on_done: Some(Box::new({
                let id = id.clone();
                let items = items.clone();
                let seeds = seeds.clone();
                let history = history.clone();
                let stray_hits = stray_hits.clone();
                let seed_started_at = seed_started_at.clone();
                let event_tx = self.event_tx.clone();
                move || {
                    let id = id.clone();
                    let items = items.clone();
                    let seeds = seeds.clone();
                    let history = history.clone();
                    let stray_hits = stray_hits.clone();
                    let seed_started_at = seed_started_at.clone();
                    let event_tx = event_tx.clone();
                    tokio::spawn(async move {
                        let mut items_guard = items.lock().await;
                        if let Some(it) = items_guard.get_mut(&id) {
                            if it.total_bytes > 0 {
                                it.downloaded_bytes = it.total_bytes;
                            }
                            let completed_item = it.clone();
                            drop(items_guard);

                            let rec = HistoryItem {
                                id: completed_item.id.clone(),
                                name: completed_item.name.clone(),
                                source: completed_item.source,
                                size_bytes: completed_item.total_bytes,
                                magnet: completed_item.magnet.clone(),
                                dir: completed_item.dir.clone(),
                                completed_at: chrono::Utc::now().timestamp_millis(),
                            };
                            let mut hist = history.lock().await;
                            hist.retain(|h| h.id != rec.id);
                            hist.insert(0, rec.clone());
                            hist.truncate(HISTORY_CAP);
                            drop(hist);
                            save_history(&history.lock().await.clone()).await.ok();

                            items.lock().await.remove(&id);
                            let seed = SeedItem {
                                id: completed_item.id.clone(),
                                name: completed_item.name.clone(),
                                source: completed_item.source,
                                magnet: completed_item.magnet.clone(),
                                dir: completed_item.dir.clone(),
                                size_bytes: completed_item.total_bytes,
                                status: SeedStatus::Seeding,
                                upload_speed: 0,
                                uploaded: 0,
                                peers: 0,
                            };
                            seeds.lock().await.insert(id.clone(), seed);
                            stray_hits.lock().await.insert(id.clone(), 0);
                            seed_started_at.lock().await.insert(id.clone(), chrono::Utc::now().timestamp_millis());

                            let _ = event_tx.send(QueueEvent::Completed(completed_item.name.clone()));
                            let _ = event_tx.send(QueueEvent::Update);
                        } else {
                            drop(items_guard);
                            if seeds.lock().await.contains_key(&id) {
                                stray_hits.lock().await.insert(id.clone(), 0);
                                seed_started_at.lock().await.remove(&id);
                            }
                            let _ = event_tx.send(QueueEvent::Update);
                        }
                    });
                }
            })),
            on_error: Some(Box::new({
                let id = id.clone();
                let items = items.clone();
                let seeds = seeds.clone();
                let seed_started_at = seed_started_at.clone();
                let event_tx = self.event_tx.clone();
                move |msg| {
                    let id = id.clone();
                    let items = items.clone();
                    let seeds = seeds.clone();
                    let seed_started_at = seed_started_at.clone();
                    let event_tx = event_tx.clone();
                    tokio::spawn(async move {
                        let mut items_guard = items.lock().await;
                        if let Some(it) = items_guard.get_mut(&id) {
                            it.status = DownloadStatus::Failed;
                            let err_msg = msg.clone();
                            it.error = Some(msg);
                            it.speed = 0;
                            it.peers = 0;
                            let name = it.name.clone();
                            drop(items_guard);
                            let _ = event_tx.send(QueueEvent::Failed(name));
                            let _ = event_tx.send(QueueEvent::Update);
                        } else {
                            drop(items_guard);
                            let mut seeds_guard = seeds.lock().await;
                            if let Some(sd) = seeds_guard.get_mut(&id) {
                                sd.status = SeedStatus::Missing;
                                sd.upload_speed = 0;
                                sd.peers = 0;
                                drop(seeds_guard);
                                seed_started_at.lock().await.remove(&id);
                                let _ = event_tx.send(QueueEvent::Update);
                            }
                        }
                    });
                }
            })),
        };

        self.engine
            .add(&item.id, &item.magnet, &item.dir, handlers, announce)
            .await;
    }

    /// Start queued torrents while download slots are free.
    async fn promote(&self) {
        let cap = if self.max_downloads == 0 {
            usize::MAX
        } else {
            self.max_downloads
        };

        loop {
            if self.active_count().await >= cap {
                break;
            }

            let next = {
                let items = self.items.lock().await;
                let mut queued: Vec<QueueItem> = items
                    .values()
                    .filter(|it| it.status == DownloadStatus::Queued)
                    .cloned()
                    .collect();
                queued.sort_by(|a, b| a.added_at.cmp(&b.added_at));
                queued.into_iter().next()
            };

            if let Some(mut next) = next {
                next.status = DownloadStatus::Downloading;
                next.speed = 0;
                self.items.lock().await.insert(next.id.clone(), next.clone());
                self.start_engine(&next).await;
            } else {
                break;
            }
        }
    }

    pub async fn pause(&self, id: &str) {
        let mut items = self.items.lock().await;
        let it = match items.get_mut(id) {
            Some(it) => it,
            None => return,
        };
        if it.status != DownloadStatus::Downloading && it.status != DownloadStatus::Queued {
            return;
        }
        let was_downloading = it.status == DownloadStatus::Downloading;
        it.status = DownloadStatus::Paused;
        it.speed = 0;
        it.peers = 0;
        it.eta = None;
        drop(items);

        if was_downloading {
            self.engine.remove(id).await;
        }

        self.changed();
        self.persist().await;
        if was_downloading {
            self.promote().await;
        }
    }

    pub async fn resume(&self, id: &str) {
        let mut items = self.items.lock().await;
        let it = match items.get_mut(id) {
            Some(it) => it,
            None => return,
        };
        if it.status != DownloadStatus::Paused {
            return;
        }
        let start = self.max_downloads == 0 || self.active_count().await < self.max_downloads;
        it.status = if start {
            DownloadStatus::Downloading
        } else {
            DownloadStatus::Queued
        };
        let item = it.clone();
        drop(items);

        if start {
            self.start_engine(&item).await;
        }

        self.changed();
        self.persist().await;
    }

    pub async fn toggle_pause(&self, id: &str) {
        let status = {
            let items = self.items.lock().await;
            items.get(id).map(|it| it.status)
        };
        match status {
            Some(DownloadStatus::Downloading) | Some(DownloadStatus::Queued) => {
                self.pause(id).await
            }
            Some(DownloadStatus::Paused) => self.resume(id).await,
            _ => {}
        }
    }

    pub async fn cancel(&self, id: &str) {
        let existed = {
            let mut items = self.items.lock().await;
            items.remove(id).is_some()
        };
        if !existed {
            return;
        }
        self.engine.remove(id).await;
        delete_torrent_meta(id);
        self.changed();
        self.persist().await;
        self.promote().await;
    }

    pub async fn remove(&self, id: &str, delete_files: bool) -> bool {
        let item_dir: Option<(String, String)> = {
            let items = self.items.lock().await;
            let seeds = self.seeds.lock().await;
            let history = self.history.lock().await;
            let it = items.get(id);
            let sd = seeds.get(id);
            let ht = history.iter().find(|h| h.id == id);
            if it.is_none() && sd.is_none() && ht.is_none() {
                return false;
            }
            let dir = it.map(|i| i.dir.clone())
                .or_else(|| sd.map(|s| s.dir.clone()))
                .or_else(|| ht.map(|h| h.dir.clone()));
            let name = it.map(|i| i.name.clone())
                .or_else(|| sd.map(|s| s.name.clone()))
                .or_else(|| ht.map(|h| h.name.clone()));
            dir.zip(name)
        };

        self.engine.remove(id).await;
        self.items.lock().await.remove(id);
        self.seeds.lock().await.remove(id);
        self.stray_hits.lock().await.remove(id);
        self.seed_started_at.lock().await.remove(id);
        delete_torrent_meta(id);
        self.remove_history(id).await;

        if delete_files {
            if let Some((dir, name)) = item_dir {
                delete_seed_data(&dir, &name).await;
            }
        }

        self.changed();
        self.persist().await;
        self.persist_seeds().await;
        self.promote().await;
        true
    }

    pub async fn retry(&self, id: &str) {
        let mut items = self.items.lock().await;
        let it = match items.get_mut(id) {
            Some(it) => it,
            None => return,
        };
        if it.status != DownloadStatus::Failed {
            return;
        }
        it.error = None;
        let start = self.max_downloads == 0 || self.active_count().await < self.max_downloads;
        it.status = if start {
            DownloadStatus::Downloading
        } else {
            DownloadStatus::Queued
        };
        let item = it.clone();
        drop(items);

        if start {
            self.start_engine(&item).await;
        }
        self.changed();
        self.persist().await;
    }

    pub async fn retry_failed(&self) {
        let failed_ids: Vec<String> = {
            let items = self.items.lock().await;
            items
                .values()
                .filter(|it| it.status == DownloadStatus::Failed)
                .map(|it| it.id.clone())
                .collect()
        };
        for id in failed_ids {
            self.retry(&id).await;
        }
    }

    pub async fn stop_seeding(&self, id: &str) {
        let mut seeds = self.seeds.lock().await;
        let s = match seeds.get_mut(id) {
            Some(s) => s,
            None => return,
        };
        self.engine.remove(id).await;
        self.stray_hits.lock().await.remove(id);
        self.seed_started_at.lock().await.remove(id);
        if s.status == SeedStatus::Seeding {
            s.status = SeedStatus::Paused;
            s.upload_speed = 0;
            s.peers = 0;
        }
        drop(seeds);
        self.changed();
        self.persist_seeds().await;
    }

    pub async fn start_seeding(&self, h: &HistoryItem) {
        {
            let seeds = self.seeds.lock().await;
            if let Some(s) = seeds.get(&h.id) {
                if s.status == SeedStatus::Seeding {
                    return;
                }
            }
        }
        {
            let items = self.items.lock().await;
            if items.contains_key(&h.id) {
                return; // don't seed a file that's downloading
            }
        }

        if h.magnet.is_empty() {
            let mut seeds = self.seeds.lock().await;
            seeds.insert(
                h.id.clone(),
                SeedItem {
                    id: h.id.clone(),
                    name: h.name.clone(),
                    source: h.source,
                    magnet: h.magnet.clone(),
                    dir: h.dir.clone(),
                    size_bytes: h.size_bytes,
                    status: SeedStatus::Missing,
                    upload_speed: 0,
                    uploaded: 0,
                    peers: 0,
                },
            );
            drop(seeds);
            self.changed();
            self.persist_seeds().await;
            return;
        }

        let seed = SeedItem {
            id: h.id.clone(),
            name: h.name.clone(),
            source: h.source,
            magnet: h.magnet.clone(),
            dir: h.dir.clone(),
            size_bytes: h.size_bytes,
            status: SeedStatus::Seeding,
            upload_speed: 0,
            uploaded: 0,
            peers: 0,
        };
        self.seeds.lock().await.insert(h.id.clone(), seed);
        self.stray_hits.lock().await.insert(h.id.clone(), 0);
        self.seed_started_at.lock().await.insert(h.id.clone(), chrono::Utc::now().timestamp_millis());

        // Use .torrent metadata if available, otherwise magnet
        let source = if paths::torrent_meta_path(&h.id).exists() {
            paths::torrent_meta_path(&h.id).to_string_lossy().to_string()
        } else {
            h.magnet.clone()
        };

        let trackers = self.trackers.lock().await.clone();
        let announce = if trackers.is_empty() { None } else { Some(trackers) };

        let handlers = AddHandlers {
            on_metadata: None,
            on_done: Some(Box::new(|| {})),
            on_error: Some(Box::new(|_msg| {})),
        };

        self.engine.add(&h.id, &source, &h.dir, handlers, announce).await;
        self.changed();
        self.persist_seeds().await;
    }

    /// Poll stats for all active downloads and seeds. Called periodically.
    pub async fn tick(&self) {
        let now = chrono::Utc::now().timestamp_millis();
        let mut any = false;

        // Update downloading items
        let item_ids: Vec<String> = {
            let items = self.items.lock().await;
            items
                .values()
                .filter(|it| it.status == DownloadStatus::Downloading)
                .map(|it| it.id.clone())
                .collect()
        };

        for id in item_ids {
            if let Some(s) = self.engine.stats(&id).await {
                let mut items = self.items.lock().await;
                if let Some(it) = items.get_mut(&id) {
                    it.progress = (s.progress * 100.0).min(100.0) as u8;
                    it.downloaded_bytes = s.downloaded;
                    if s.total > 0 {
                        it.total_bytes = s.total;
                    }
                    it.speed = s.speed;
                    it.peers = s.peers;
                    it.eta = if s.time_remaining > 0 { Some(s.time_remaining) } else { None };
                    if !s.name.is_empty() {
                        it.name = s.name;
                    }
                    any = true;
                }
            }
        }

        // Update seeds + stray detection
        let seed_ids: Vec<String> = {
            let seeds = self.seeds.lock().await;
            seeds
                .values()
                .filter(|s| s.status == SeedStatus::Seeding)
                .map(|s| s.id.clone())
                .collect()
        };

        for id in seed_ids {
            if let Some(s) = self.engine.stats(&id).await {
                let age = now - self.seed_started_at.lock().await.get(&id).copied().unwrap_or(0);
                if age > SEED_GRACE_MS as i64 && stray_download(&s) {
                    let mut stray = self.stray_hits.lock().await;
                    let hits = stray.entry(id.clone()).or_insert(0);
                    *hits += 1;
                    if *hits >= STRAY_TICKS {
                        drop(stray);
                        self.engine.remove(&id).await;
                        self.stray_hits.lock().await.remove(&id);
                        self.seed_started_at.lock().await.remove(&id);
                        let mut seeds = self.seeds.lock().await;
                        if let Some(sd) = seeds.get_mut(&id) {
                            sd.status = SeedStatus::Missing;
                            sd.upload_speed = 0;
                            sd.peers = 0;
                        }
                        drop(seeds);
                        self.persist_seeds().await;
                    }
                    any = true;
                    continue;
                }

                self.stray_hits.lock().await.insert(id.clone(), 0);
                let mut seeds = self.seeds.lock().await;
                if let Some(sd) = seeds.get_mut(&id) {
                    sd.upload_speed = s.upload_speed;
                    sd.uploaded = s.uploaded;
                    sd.peers = s.peers;
                }
                drop(seeds);
                any = true;
            }
        }

        if any {
            self.changed();
        }
    }

    // --- Restore ---

    pub async fn restore(&self, items: Vec<QueueItem>, opts: RestoreOptions) {
        if opts.safe {
            let mut items_guard = self.items.lock().await;
            for mut raw in items {
                if raw.status == DownloadStatus::Downloading || raw.status == DownloadStatus::Queued
                {
                    raw.status = DownloadStatus::Paused;
                }
                items_guard.insert(raw.id.clone(), raw);
            }
            drop(items_guard);
            self.changed();
            self.persist().await;
            return;
        }

        let mut active = 0usize;
        let mut items_guard = self.items.lock().await;
        for mut raw in items {
            items_guard.insert(raw.id.clone(), raw.clone());
            if raw.status != DownloadStatus::Downloading {
                continue;
            }
            if self.max_downloads == 0 || active < self.max_downloads {
                self.start_engine(&raw).await;
                active += 1;
            } else {
                raw.status = DownloadStatus::Queued;
                items_guard.insert(raw.id.clone(), raw);
            }
        }
        drop(items_guard);
        self.changed();
        self.promote().await;
    }

    pub async fn restore_history(&self, items: Vec<HistoryItem>) {
        let mut history = self.history.lock().await;
        *history = items.into_iter().take(HISTORY_CAP).collect();
    }

    pub async fn restore_seeds(&self, records: Vec<SeedRecord>, opts: RestoreOptions) {
        for r in records {
            let h = {
                let history = self.history.lock().await;
                history.iter().find(|x| x.id == r.id).cloned()
            };
            let Some(h) = h else { continue };

            if r.status == PersistedSeedStatus::Seeding && !opts.safe {
                self.start_seeding(&h).await;
            } else {
                // Restore as paused
                let seed = SeedItem {
                    id: h.id.clone(),
                    name: h.name.clone(),
                    source: h.source,
                    magnet: h.magnet.clone(),
                    dir: h.dir.clone(),
                    size_bytes: h.size_bytes,
                    status: SeedStatus::Paused,
                    upload_speed: 0,
                    uploaded: 0,
                    peers: 0,
                };
                self.seeds.lock().await.insert(h.id, seed);
                self.changed();
            }
        }
        if opts.safe {
            self.persist_seeds().await;
        }
    }

    pub async fn remove_history(&self, id: &str) {
        let mut history = self.history.lock().await;
        let next: Vec<HistoryItem> = history.iter().filter(|h| h.id != id).cloned().collect();
        if next.len() == history.len() {
            return;
        }
        *history = next;
        drop(history);

        if self.seeds.lock().await.contains_key(id) {
            self.engine.remove(id).await;
            self.seeds.lock().await.remove(id);
            self.stray_hits.lock().await.remove(id);
            self.seed_started_at.lock().await.remove(id);
            self.persist_seeds().await;
        }
        delete_torrent_meta(id);
        save_history(&self.history.lock().await.clone()).await.ok();
        self.changed();
    }

    pub async fn clear_history(&self) {
        let mut history = self.history.lock().await;
        if history.is_empty() {
            return;
        }
        for h in history.iter() {
            delete_torrent_meta(&h.id);
        }
        history.clear();
        drop(history);

        let seed_ids: Vec<String> = self.seeds.lock().await.keys().cloned().collect();
        for id in &seed_ids {
            self.engine.remove(id).await;
        }
        self.seeds.lock().await.clear();
        self.stray_hits.lock().await.clear();
        self.seed_started_at.lock().await.clear();
        self.persist_seeds().await;
        save_history(&self.history.lock().await.clone()).await.ok();
        self.changed();
    }

    pub async fn export_torrent_file(&self, id: &str, name: &str) -> Option<String> {
        let dir = if let Some(it) = self.items.lock().await.get(id).map(|it| it.dir.clone()) {
            it
        } else if let Some(s) = self.seeds.lock().await.get(id).map(|s| s.dir.clone()) {
            s
        } else {
            self.history.lock().await.iter().find(|h| h.id == id).map(|h| h.dir.clone())?
        };
        export_torrent_meta(id, name, &dir).await
    }

    // --- Persistence ---

    async fn persist(&self) {
        let items = self.get_items().await;
        save_queue(&items).await.ok();
    }

    async fn persist_seeds(&self) {
        let seeds = self.seeds.lock().await;
        let records: Vec<SeedRecord> = seeds
            .values()
            .map(|s| SeedRecord {
                id: s.id.clone(),
                status: if s.status == SeedStatus::Seeding {
                    PersistedSeedStatus::Seeding
                } else {
                    PersistedSeedStatus::Paused
                },
            })
            .collect();
        drop(seeds);
        save_seeds(&records).await.ok();
    }

    /// Synchronously flush all state files. Used on quit.
    pub fn persist_sync(&self) {
        // We need to try_lock since this may be called from a non-async context
        if let Ok(items) = self.items.try_lock() {
            let items_vec: Vec<QueueItem> = items.values().cloned().collect();
            save_queue_sync(&items_vec);
        }
        if let Ok(seeds) = self.seeds.try_lock() {
            let records: Vec<SeedRecord> = seeds
                .values()
                .map(|s| SeedRecord {
                    id: s.id.clone(),
                    status: if s.status == SeedStatus::Seeding {
                        PersistedSeedStatus::Seeding
                    } else {
                        PersistedSeedStatus::Paused
                    },
                })
                .collect();
            save_seeds_sync(&records);
        }
        if let Ok(history) = self.history.try_lock() {
            save_history_sync(&history);
        }
        bootguard::disarm_boot_marker();
    }

    /// Zero live stats, flush state, stop engine. Called on quit/suspend.
    pub async fn suspend(&self) {
        let mut items = self.items.lock().await;
        for it in items.values_mut() {
            if it.status == DownloadStatus::Downloading {
                it.speed = 0;
                it.peers = 0;
                it.eta = None;
            }
        }
        drop(items);
        self.persist_sync();
        self.engine.destroy().await;
    }
}

/// Delete a seed's downloaded data. Best-effort, never panics.
async fn delete_seed_data(dir: &str, name: &str) {
    let path = PathBuf::from(dir).join(name);
    if path.is_dir() {
        tokio::fs::remove_dir_all(&path).await.ok();
    } else if path.is_file() {
        tokio::fs::remove_file(&path).await.ok();
    }
}
