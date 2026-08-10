# torlnk

A sleek, zero-setup torrent finder and downloader that lives right in your terminal.

Search across 10+ curated sources, download via librqbit, seed automatically, and run headless for seedboxes — all from a single Rust binary.

## Features

- **Search 10+ sources** — FitGirl, YTS, EZTV, Nyaa, SubsPlease, TPB, 1337x, BitTorrented
- **TUI** — ratatui-based interface with sidebar navigation, real-time search results, download progress, and seeding panel
- **Download engine** — powered by librqbit (TCP/uTP, no WebRTC)
- **Auto-seed** — completed downloads seed back automatically with stray detection
- **Headless modes** — `watch` (folder watcher), `serve` (HTTP API), `files` (file server), `attach` (tmux)
- **Bootguard** — crash recovery with safe-mode restore
- **Persistence** — queue, seeds, and history saved to JSON
- **Feedback alerts** — success/error/warn/info toasts for all key actions and events

## Installation

```bash
cargo install --path .
```

## Usage

### TUI (interactive)

```bash
torlnk
```

| Key | Action |
|-----|--------|
| `/` | Search |
| `d` | Download selected result |
| `y` | Copy magnet to clipboard |
| `z` | Toggle hide dead torrents |
| `p` | Pause/resume download or stop seeding |
| `c` | Cancel download / remove seed |
| `e` | Open download folder |
| `Tab` | Switch between sidebar and content |
| `?` | Show help overlay |
| `q` | Quit |

### Headless input (direct download)

```bash
torlnk "magnet:?xt=urn:btih:HASH&dn=Name"
torlnk /path/to/file.torrent
torlnk 08ada5a7a6183aae1e09d831df6748d566095a10  # bare info hash
```

### JSON search (non-interactive)

```bash
torlnk --json "dune" | jq '.[] | select(.seeders > 100)'
```

### Serve mode (HTTP API)

```bash
torlnk serve --port 9161 --dir ~/Downloads

# Search
curl "http://127.0.0.1:9161/search?q=dune"

# Add download
curl -X POST http://127.0.0.1:9161/download \
  -H "Content-Type: application/json" \
  -d '{"input":"magnet:?xt=urn:btih:HASH"}'

# List downloads / seeds
curl http://127.0.0.1:9161/downloads
curl http://127.0.0.1:9161/seeds

# Pause / resume / cancel / remove
curl -X POST http://127.0.0.1:9161/pause/HASH
curl -X POST http://127.0.0.1:9161/resume/HASH
curl -X POST http://127.0.0.1:9161/cancel/HASH
curl -X POST http://127.0.0.1:9161/remove/HASH
```

### Files mode (HTTP file server)

```bash
torlnk files --port 9160 --dir ~/Downloads
```

Browse and download files via HTTP. Path traversal protected.

### Watch mode (folder watcher)

```bash
torlnk watch --dir ~/Downloads/torrents
```

Drop `.torrent` files into the watched folder — they auto-download.

### Attach mode (tmux)

```bash
torlnk attach
```

Creates or attaches to a persistent tmux session running the TUI.

## Configuration

Environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `TORLINK_STATE_DIR` | Platform-specific | State directory (queue, seeds, history) |
| `TORLINK_MAX_DOWNLOADS` | `4` | Max concurrent downloads |
| `TORLINK_DOWNLOAD_DIR` | `~/Downloads/torlnk-rs` | Download directory |

Check paths:

```bash
torlnk --paths
```

## Sources

| Source | Categories | Type |
|--------|-----------|------|
| FitGirl | Games | HTML |
| YTS | Movies | RSS/API |
| EZTV | TV | RSS/API |
| Nyaa | Anime | RSS |
| SubsPlease | Anime | RSS |
| TPB (Movies) | Movies | HTML |
| TPB (TV) | TV | HTML |
| 1337x (Movies) | Movies | HTML |
| 1337x (TV) | TV | HTML |
| BitTorrented | Movies, TV | HTML |

## Tech Stack

- **TUI**: ratatui + crossterm
- **Torrent engine**: librqbit
- **Async runtime**: tokio (single runtime, mpsc channels)
- **HTTP client**: reqwest
- **Web framework**: axum
- **CLI**: clap
- **Persistence**: serde_json + tokio::fs

## License

MIT
