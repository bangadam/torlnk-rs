# torlnk-rs

A terminal-native torrent finder and downloader, rewritten in Rust from the original torlink (TypeScript/Ink). Searches curated sources, downloads via librqbit, seeds automatically, and runs headless for seedboxes.

## Language

**Source**:
A site torlnk-rs searches for torrents. Each has an id, label, category groups, and a search function.
_Avoid_: Tracker (a tracker is a BitTorrent announce server, not a search site), provider, indexer

**SourceGroup**:
A category tab a source feeds: Games, Movies, TV, or Anime.
_Avoid_: Category, tab

**TorrentResult**:
A single search hit from a Source: infohash, name, size, seeder/leecher counts, magnet link.
_Avoid_: Search result, hit, entry

**Magnet**:
A magnet URI (`magnet:?xt=urn:btih:...`) identifying a torrent by infohash plus optional trackers and display name.
_Avoid_: Magnet link (redundant), URI

**InfoHash**:
The 40-character SHA-1 hash uniquely identifying a torrent's content. The primary key for downloads and seeds.
_Avoid_: Hash, torrent id

**QueueItem**:
An entry in the download queue: a magnet being downloaded, queued, paused, completed, or failed.
_Avoid_: Download, task, job

**SeedItem**:
A completed download kept alive for sharing back: seeding, paused, or missing.
_Avoid_: Seeded torrent, shared file

**DownloadStatus**:
The lifecycle state of a QueueItem: `downloading`, `queued`, `paused`, `completed`, `failed`.
_Avoid_: State, phase

**SeedStatus**:
The lifecycle state of a SeedItem: `seeding`, `paused`, `missing`.
_Avoid_: State, phase

**Seeding**:
Sharing a completed torrent back to the swarm so others can download it. torlnk-rs does this automatically on completion (opt-out).
_Avoid_: Uploading, sharing

**Stray Download**:
A seed that reports active download speed with progress below 100% — meaning its files are gone or partial on disk. Detected after a grace period to distinguish from normal piece verification.
_Avoid_: Phantom download, zombie seed

**Seed Reaper**:
A headless-mode timer that auto-stops seeding a torrent after a configurable duration, optionally deleting the downloaded data.
_Avoid_: Seed killer, auto-stop

**Bootguard**:
A crash-recovery mechanism: a marker file is armed before restoring persisted state and disarmed after the boot settles. Finding a marker at startup means the previous boot crashed mid-restore, triggering safe mode (all items restored paused, no engines started).
_Avoid_: Crash marker, safe boot

**Headless Mode**:
Running torlnk-rs without the TUI for servers and seedboxes: `watch` (folder watch), `serve` (HTTP add API), `files` (HTTP file server), `attach` (tmux persistent session).
_Avoid_: Daemon mode (daemonize is the backgrounding mechanism, not the mode itself)

**Daemonize**:
Backgrounding a headless process (own session, log to file) so it survives logout.
_Avoid_: Fork, detach
