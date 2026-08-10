use crate::config::config::Config;
use crate::download::engine::TorrentEngine;
use crate::download::queue::{AddInput, DownloadQueue};
use crate::sources::parse_input;
use notify::{recommended_watcher, EventKind, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::Arc;

pub async fn run_watch(dir: String, config: Config) -> anyhow::Result<()> {
    let dir_path = Path::new(&dir);
    tokio::fs::create_dir_all(dir_path).await?;

    let engine = Arc::new(TorrentEngine::new().await?);
    let queue = Arc::new(DownloadQueue::new(engine));

    let trackers = crate::config::trackers::parse_trackers(&config.trackers.join(","));
    queue.set_trackers(trackers);

    // Process existing .torrent files in directory
    let initial: Vec<std::path::PathBuf> = {
        let mut entries = vec![];
        if let Ok(mut rd) = tokio::fs::read_dir(dir_path).await {
            while let Ok(Some(entry)) = rd.next_entry().await {
                let path = entry.path();
                if path.extension().map(|e| e == "torrent").unwrap_or(false) {
                    entries.push(path);
                }
            }
        }
        entries
    };

    for path in initial {
        process_torrent_file(&path, &queue, &dir).await;
    }

    // Watch for new .torrent files
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<std::path::PathBuf>();
    let mut watcher = recommended_watcher(move |res: Result<notify::Event, _>| {
        if let Ok(event) = res {
            if let EventKind::Create(_) | EventKind::Modify(_) = event.kind {
                for path in event.paths {
                    if path.extension().map(|e| e == "torrent").unwrap_or(false) {
                        let _ = tx.send(path);
                    }
                }
            }
        }
    })?;

    watcher.watch(dir_path, RecursiveMode::NonRecursive)?;

    tracing::info!("watching {} for .torrent files", dir);

    loop {
        match rx.recv().await {
            Some(path) => {
                process_torrent_file(&path, &queue, &dir).await;
            }
            None => break,
        }
    }

    Ok(())
}

async fn process_torrent_file(path: &Path, queue: &DownloadQueue, dir: &str) {
    let input = path.to_string_lossy().to_string();
    if let Some(parsed) = parse_input(&input) {
        tracing::info!("watch: adding {}", parsed.name);
        queue.add(
            AddInput {
                id: parsed.info_hash,
                name: parsed.name,
                magnet: parsed.magnet,
                source: None,
                size_bytes: None,
            },
            dir,
        ).await;
    }
}
