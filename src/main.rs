use anyhow::Result;
use clap::Parser;

use torlnk::cli::{Cli, Mode};
use torlnk::config::config::{load_config, normalize_download_dir, Config};
use torlnk::config::paths;
use torlnk::config::trackers::parse_trackers;
use torlnk::download::bootguard;
use torlnk::download::engine::TorrentEngine;
use torlnk::download::persist::{load_queue, load_seeds};
use torlnk::download::queue::{DownloadQueue, RestoreOptions};
use torlnk::download::history::load_history;
use torlnk::sources::parse_input;
use torlnk::ui::App;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    if cli.paths {
        println!("config_dir:  {}", paths::config_dir().display());
        println!("data_dir:    {}", paths::data_dir().display());
        println!("download_dir: {}", paths::default_download_dir().display());
        return Ok(());
    }

    let mut config = load_config().await;
    if let Some(dir) = &cli.dir {
        config.download_dir = dir.clone();
    }
    config.download_dir = normalize_download_dir(&config.download_dir);
    if let Some(max) = cli.max_downloads {
        if max > 0 {
            std::env::set_var("TORLINK_MAX_DOWNLOADS", max.to_string());
        }
    }

    // Headless input mode: magnet or .torrent file
    if let Some(input) = &cli.input {
        if let Some(parsed) = parse_input(input) {
            let engine = TorrentEngine::new().await?;
            let queue = std::sync::Arc::new(DownloadQueue::new(std::sync::Arc::new(engine)));
            let trackers = parse_trackers(&config.trackers.join(","));
            queue.set_trackers(trackers);

            let name = cli.name.clone().unwrap_or_else(|| parsed.name.clone());
            queue.add(
                torlnk::download::queue::AddInput {
                    id: parsed.info_hash,
                    name,
                    magnet: parsed.magnet,
                    source: None,
                    size_bytes: None,
                },
                &config.download_dir,
            ).await;

            if cli.json {
                println!("{}", serde_json::json!({
                    "name": parsed.name,
                    "dir": config.download_dir,
                }).to_string());
            } else {
                println!("Added: {} → {}", parsed.name, config.download_dir);
            }
            return Ok(());
        }

        if cli.json {
            return run_json_search(input, &config).await;
        }
    }

    match cli.mode.unwrap_or(Mode::Run) {
        Mode::Run => run_tui(config).await,
        Mode::Watch { dir } => {
            let dir = dir.or(cli.serve_dir).unwrap_or(config.download_dir.clone());
            torlnk::daemon::watch::run_watch(dir, config).await
        }
        Mode::Serve { port, dir, token } => {
            let port = port.or(cli.api_port).unwrap_or(9161);
            let dir = dir.or(cli.serve_dir).unwrap_or(config.download_dir.clone());
            let token = token.or(cli.token);
            torlnk::daemon::serve::run_serve(port, dir, token, config).await
        }
        Mode::Files { port, dir, token } => {
            let port = port.or(cli.files_port).unwrap_or(9160);
            let dir = dir.or(cli.serve_dir).unwrap_or(config.download_dir.clone());
            let token = token.or(cli.token);
            torlnk::daemon::files::run_files(port, dir, token).await
        }
        Mode::Attach => torlnk::daemon::attach::run_attach().await,
        Mode::Update => {
            println!("torlnk {}", env!("CARGO_PKG_VERSION"));
            println!("Self-update is not yet implemented. Download from GitHub Releases.");
            Ok(())
        }
    }
}

async fn run_tui(config: Config) -> Result<()> {
    let engine = TorrentEngine::new().await?;
    let engine = std::sync::Arc::new(engine);
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
    let queue = std::sync::Arc::new(DownloadQueue::with_event_sender(engine.clone(), event_tx));

    let trackers = parse_trackers(&config.trackers.join(","));
    queue.set_trackers(trackers);

    let interrupted = bootguard::was_boot_interrupted();
    if interrupted {
        tracing::warn!("Previous boot was interrupted — restoring in safe mode");
    }
    bootguard::arm_boot_marker();

    let items = load_queue().await;
    let seeds = load_seeds().await;
    let history = load_history().await;

    let items = torlnk::download::reconcile::reconcile_queue(items);
    queue.restore(items, RestoreOptions { safe: interrupted }).await;
    queue.restore_history(history).await;
    queue.restore_seeds(seeds, RestoreOptions { safe: interrupted }).await;

    tokio::time::sleep(std::time::Duration::from_millis(
        bootguard::BOOT_SETTLE_MS,
    )).await;

    bootguard::disarm_boot_marker();

    let mut app = App::new(config, queue);
    app.queue_rx = Some(event_rx);
    app.run().await
}

async fn run_json_search(query: &str, config: &Config) -> Result<()> {
    use torlnk::sources::all_sources;
    let sources = all_sources();
    let client = torlnk::util::net::build_client();
    let mut all_results = vec![];

    for source in sources {
        match source.search(query, &client, None).await {
            Ok(results) => all_results.extend(results),
            Err(_) => continue,
        }
    }

    let json: Vec<serde_json::Value> = all_results
        .iter()
        .map(|r| {
            serde_json::json!({
                "name": r.name,
                "source": r.source.tag(),
                "seeders": r.seeders,
                "leechers": r.leechers,
                "size_bytes": r.size_bytes,
                "magnet": r.magnet,
                "id": r.info_hash,
            })
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&json)?);
    let _ = config;
    Ok(())
}
