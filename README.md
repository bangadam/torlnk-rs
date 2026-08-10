<div align="center">

# torlnk

**A sleek, zero-setup torrent finder and downloader that lives right in your terminal.**

[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Linux%20%7C%20Windows-green.svg)](#platform-support)
[![Crates.io](https://img.shields.io/badge/crates.io-not%20published-red.svg)](#installation)

Search across 10+ curated sources, download via librqbit, seed automatically, and run headless for seedboxes — all from a single Rust binary.

</div>

---

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Usage](#usage)
  - [TUI Mode](#tui-mode-interactive)
  - [Headless Input](#headless-input-direct-download)
  - [JSON Search](#json-search-non-interactive)
  - [Serve Mode](#serve-mode-http-api)
  - [Files Mode](#files-mode-http-file-server)
  - [Watch Mode](#watch-mode-folder-watcher)
  - [Attach Mode](#attach-mode-tmux)
- [Configuration](#configuration)
- [Sources](#sources)
- [Architecture](#architecture)
- [Platform Support](#platform-support)
- [Contributing](#contributing)
- [License](#license)

## Overview

torlnk is a terminal-native torrent client built in Rust. It combines searching, downloading, and seeding into one cohesive TUI experience — no browser, no daemon setup, no configuration files. Just run `torlnk` and start searching.

For server and seedbox use cases, torlnk runs headless: watch a folder for `.torrent` files, expose a REST API for remote control, serve downloaded files over HTTP, or attach via tmux for persistent sessions.

### Why torlnk?

- **Zero setup** — no config files, no daemon, just run the binary
- **Multi-source search** — query 10+ sources simultaneously from one interface
- **Terminal-native** — keyboard-driven, fast, works over SSH
- **Seedbox-ready** — headless modes for servers and remote management
- **Crash-resilient** — bootguard safe-mode restore on unexpected exits
- **Single binary** — everything compiled into one executable, no runtime dependencies

## Features

### Search
- **10+ sources** — FitGirl, YTS, EZTV, Nyaa, SubsPlease, The Pirate Bay, 1337x, BitTorrented
- **Category filtering** — All, Games, Movies, TV, Anime
- **Streaming results** — results appear as each source responds
- **Dead torrent filtering** — hide results with zero seeders
- **Magnet copy** — copy magnet links to clipboard without downloading

### Download
- **librqbit engine** — TCP and uTP peer support, no WebRTC required
- **Full lifecycle** — queue, download, pause, resume, cancel, retry, remove
- **Concurrency cap** — configurable max simultaneous downloads
- **Progress tracking** — real-time speed, ETA, peer count, progress bar
- **Auto-seed** — completed downloads seed back automatically
- **Stray detection** — detects seeds with missing files and marks them

### Headless
- **`watch`** — monitor a folder for `.torrent` files, auto-download
- **`serve`** — REST API for remote search, download, and queue management
- **`files`** — HTTP file server with directory listing and path traversal protection
- **`attach`** — tmux session management for persistent TUI

### Resilience
- **Bootguard** — crash recovery with safe-mode (all items restored paused)
- **Persistence** — queue, seeds, and history saved to JSON on disk
- **Token auth** — optional API authentication for serve and files modes

### UX
- **Onboarding splash** — numbered getting-started guide for new users
- **Feedback alerts** — color-coded toasts (green ✓ success, red ✗ error, yellow ⚠ warning, purple • info)
- **Title bar** — always-visible context showing current section
- **Bordered panes** — focus indication with accent colors
- **Live counts** — sidebar shows active download and seed counts

## Installation

### From source

```bash
git clone https://github.com/bangadam/torlnk-rs.git
cd torlnk-rs
cargo install --path .
```

### Requirements

- Rust 1.75+ (2021 edition)
- Platform: macOS, Linux, or Windows

## Quick Start

```bash
# Search and download interactively
torlnk

# Or go straight to headless download
torlnk "magnet:?xt=urn:btih:08ada5a7a6183aae1e09d831df6748d566095a10&dn=Sintel"

# Check where torlnk stores data
torlnk --paths
```

## Usage

### TUI Mode (Interactive)

```bash
torlnk
# or
torlnk run
```

The TUI opens with a splash screen showing a numbered getting-started guide. Press `/` to search, navigate results with arrow keys, press `d` to download.

#### Keybindings

| Key | Action |
|-----|--------|
| `/` | Start search |
| `Enter` | Confirm search / open category |
| `↑` `↓` / `k` `j` | Navigate list |
| `Tab` | Switch between sidebar and content |
| `d` | Download selected result |
| `y` | Copy magnet to clipboard |
| `z` | Toggle hide dead torrents (0 seeders) |
| `p` | Pause/resume download · Stop seeding |
| `c` | Cancel download · Remove seed |
| `Shift+c` | Retry all failed downloads · Remove seed + delete files |
| `e` | Open download/seed folder in file manager |
| `?` | Show help overlay |
| `Esc` | Back to splash |
| `q` | Quit |

### Headless Input (Direct Download)

Download directly from the CLI without entering the TUI:

```bash
# Magnet link
torlnk "magnet:?xt=urn:btih:HASH&dn=Name"

# .torrent file
torlnk /path/to/file.torrent

# Bare info hash (auto-wrapped with default trackers)
torlnk 08ada5a7a6183aae1e09d831df6748d566095a10
```

### JSON Search (Non-Interactive)

```bash
torlnk --json "dune" | jq '.[] | select(.seeders > 100)'
```

Output is a JSON array with `name`, `source`, `seeders`, `leechers`, `size_bytes`, `magnet`, and `id` fields.

### Serve Mode (HTTP API)

```bash
torlnk serve --port 9161 --dir ~/Downloads
```

REST API endpoints:

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/health` | Health check |
| `GET` | `/search?q=<query>` | Search across sources |
| `POST` | `/download` | Add download (`{"input":"magnet:...}"}`) |
| `GET` | `/downloads` | List active downloads |
| `GET` | `/seeds` | List seeds |
| `POST` | `/pause/{id}` | Pause download |
| `POST` | `/resume/{id}` | Resume download |
| `POST` | `/cancel/{id}` | Cancel download |
| `POST` | `/remove/{id}` | Remove download |

```bash
# Example: search and download via API
curl "http://127.0.0.1:9161/search?q=dune"
curl -X POST http://127.0.0.1:9161/download \
  -H "Content-Type: application/json" \
  -d '{"input":"magnet:?xt=urn:btih:HASH"}'
```

With token authentication:

```bash
torlnk serve --port 9161 --dir ~/Downloads --token mysecret
curl "http://127.0.0.1:9161/downloads?token=mysecret"
```

### Files Mode (HTTP File Server)

```bash
torlnk files --port 9160 --dir ~/Downloads
```

Browse and download files via HTTP with directory listing and path traversal protection.

```bash
curl http://127.0.0.1:9160/           # list files
curl http://127.0.0.1:9160/movie.mp4  # download file
```

### Watch Mode (Folder Watcher)

```bash
torlnk watch --dir ~/Downloads/torrents
```

Drop `.torrent` files into the watched folder — they are automatically detected and downloaded.

### Attach Mode (tmux)

```bash
torlnk attach
```

Creates or attaches to a persistent tmux session named `torlnk` running the TUI. Requires `tmux` installed.

## Configuration

torlnk uses environment variables — no config files needed.

| Variable | Default | Description |
|----------|---------|-------------|
| `TORLINK_STATE_DIR` | Platform-specific | Directory for queue, seeds, and history files |
| `TORLINK_MAX_DOWNLOADS` | `4` | Maximum concurrent downloads |
| `TORLINK_DOWNLOAD_DIR` | `~/Downloads/torlnk-rs` | Default download directory |

Check your current paths:

```bash
torlnk --paths
```

## Sources

| Source | Categories | Method |
|--------|-----------|--------|
| FitGirl | Games | HTML scrape |
| YTS | Movies | JSON API |
| EZTV | TV | JSON API |
| Nyaa | Anime | RSS |
| SubsPlease | Anime | RSS |
| The Pirate Bay (Movies) | Movies | HTML scrape |
| The Pirate Bay (TV) | TV | HTML scrape |
| 1337x (Movies) | Movies | HTML scrape |
| 1337x (TV) | TV | HTML scrape |
| BitTorrented | Movies, TV | HTML scrape |

## Architecture

```
torlnk-rs/
├── src/
│   ├── cli/           # clap CLI definitions
│   ├── config/        # paths, config, trackers
│   ├── sources/       # Source trait + 10 implementations + registry
│   ├── download/      # librqbit engine, queue lifecycle, persistence, bootguard
│   ├── daemon/        # headless modes: serve, files, watch, attach, daemonize
│   ├── ui/            # ratatui TUI: app, state, theme, keymap, render modules
│   ├── util/          # formatting, network helpers
│   ├── lib.rs         # module root
│   └── main.rs        # entry point + CLI dispatch
└── Cargo.toml
```

### Tech Stack

| Component | Library |
|-----------|---------|
| TUI framework | [ratatui](https://crates.io/crates/ratatui) + [crossterm](https://crates.io/crates/crossterm) |
| Torrent engine | [librqbit](https://crates.io/crates/librqbit) |
| Async runtime | [tokio](https://crates.io/crates/tokio) |
| HTTP client | [reqwest](https://crates.io/crates/reqwest) |
| Web framework | [axum](https://crates.io/crates/axum) |
| CLI parser | [clap](https://crates.io/crates/clap) |
| Serialization | [serde](https://crates.io/crates/serde) + [serde_json](https://crates.io/crates/serde_json) |
| HTML parsing | [scraper](https://crates.io/crates/scraper) |
| File watching | [notify](https://crates.io/crates/notify) |
| Error handling | [anyhow](https://crates.io/crates/anyhow) + [thiserror](https://crates.io/crates/thiserror) |

### Design Decisions

- **Single tokio runtime** with mpsc channels for UI updates — no multi-runtime complexity
- **Central AppState** with `Arc<Mutex<>>` for shared state across async tasks
- **Trait-based sources** — `Source` trait + registry pattern for easy extensibility
- **Immediate-mode TUI** — render functions per module, not declarative widgets
- **JSON persistence** — human-readable state files, easy to inspect and recover
- **No WebRTC** — TCP/uTP only, simpler and sufficient for most use cases

## Platform Support

| Platform | Status | Notes |
|----------|--------|-------|
| macOS | ✅ Full | Primary development platform |
| Linux | ✅ Full | Daemonize uses fork+setsid |
| Windows | ✅ Partial | Daemonize is a no-op; tmux attach unavailable |

## Contributing

Contributions are welcome! This project is built by the community, for the community.

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs/))
- Basic familiarity with async Rust and terminal UI concepts

### Getting Started

1. **Fork and clone**

   ```bash
   git clone https://github.com/<your-username>/torlnk-rs.git
   cd torlnk-rs
   ```

2. **Build and verify**

   ```bash
   cargo build
   cargo test --lib
   ./target/debug/torlnk --version
   ```

3. **Run the TUI during development**

   ```bash
   ./target/debug/torlnk
   ```

### Project Structure

| Directory | Responsibility |
|-----------|---------------|
| `src/sources/` | Source trait, scrapers, and registry — **add new sources here** |
| `src/download/` | Engine adapter, queue lifecycle, persistence, bootguard |
| `src/ui/` | TUI state, rendering, keymap, theme — **UI changes here** |
| `src/daemon/` | Headless modes — serve, files, watch, attach |
| `src/config/` | Paths, config loading, tracker list |

### Adding a New Source

1. Create `src/sources/yoursource.rs`
2. Implement the `Source` trait:

   ```rust
   use async_trait::async_trait;
   use crate::sources::types::{Source, SourceId, SourceGroup, TorrentResult, SourceError};

   pub struct YourSource;

   #[async_trait]
   impl Source for YourSource {
       fn id(&self) -> SourceId { SourceId::YourSource }
       fn groups(&self) -> Vec<SourceGroup> { vec![SourceGroup::Movies] }

       async fn search(
           &self,
           query: &str,
           client: &reqwest::Client,
           cancel: Option<&tokio_util::sync::CancellationToken>,
       ) -> Result<Vec<TorrentResult>, SourceError> {
           // Fetch and parse results
           Ok(vec![])
       }
   }
   ```

3. Add the `SourceId` variant in `src/sources/types.rs`
4. Register in `src/sources/registry.rs` → `all_sources()`
5. Add a color in `src/ui/theme.rs` → `source_color()`

### Coding Standards

- Follow existing code style — `cargo fmt` before committing
- No `unwrap()`/`expect()` in library code — use `?` or `anyhow`
- Keep functions focused; prefer small modules over large files
- Add unit tests for parsing logic (see `src/util/format.rs` and `src/sources/magnet.rs` for examples)
- Run `cargo clippy` and fix warnings

### Pull Request Process

1. Create a feature branch: `git checkout -b feature/your-feature`
2. Make your changes, keeping commits focused
3. Ensure everything passes:

   ```bash
   cargo fmt -- --check
   cargo clippy
   cargo test --lib
   cargo build
   ```

4. Write a clear commit message following [conventional commits](https://www.conventionalcommits.org/):

   ```
   feat: add 1337x anime source
   fix: handle empty search query without panic
   docs: update serve mode API examples
   refactor: extract progress bar rendering
   ```

5. Open a PR with:
   - **What** — what changed
   - **Why** — the motivation
   - **How** — approach taken, alternatives considered
   - **Testing** — how you verified the change

### Reporting Bugs

Open an [issue](https://github.com/bangadam/torlnk-rs/issues) with:

- OS and Rust version (`rustc --version`)
- torlnk version (`torlnk --version`)
- Steps to reproduce
- Expected vs actual behavior
- Logs (run with `RUST_LOG=debug` if relevant)

### Feature Requests

Have an idea? Open an [issue](https://github.com/bangadam/torlnk-rs/issues) with the `enhancement` label. Describe the use case and proposed solution.

### Areas for Contribution

- **New sources** — additional torrent sites (follow the [Adding a New Source](#adding-a-new-source) guide)
- **Platform testing** — test and fix issues on Windows and various Linux distros
- **Performance** — optimize source scraping, reduce memory usage
- **Accessibility** — improve screen reader compatibility, add high-contrast theme
- **Internationalization** — i18n for UI strings

## License

This project is licensed under the [MIT License](LICENSE).

<div align="center">

Made with Rust 🦀

[Report Bug](https://github.com/bangadam/torlnk-rs/issues) · [Request Feature](https://github.com/bangadam/torlnk-rs/issues) · [Contributing Guide](#contributing)

</div>
