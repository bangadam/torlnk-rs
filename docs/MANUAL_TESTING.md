# Panduan Test Manual — torlnk-rs

Panduan untuk melakukan pengujian manual terhadap semua fitur torlnk-rs.

## Prasyarat

```bash
# Build binary
cargo build --release

# Binary tersedia di
./target/release/torlnk
# atau untuk debug
./target/debug/torlnk
```

## Daftar Isi

1. [CLI Dasar](#1-cli-dasar)
2. [TUI Mode](#2-tui-mode)
3. [Serve Mode (HTTP API)](#3-serve-mode-http-api)
4. [Files Mode (HTTP File Server)](#4-files-mode-http-file-server)
5. [Watch Mode (Folder Watcher)](#5-watch-mode-folder-watcher)
6. [Attach Mode (tmux)](#6-attach-mode-tmux)
7. [Headless Input (Magnet/Torrent)](#7-headless-input-magnettorrent)
8. [JSON Search Output](#8-json-search-output)
9. [Persistensi & Bootguard](#9-persistensi--bootguard)
10. [Edge Cases](#10-edge-cases)

---

## 1. CLI Dasar

### 1.1 Versi

```bash
./target/debug/torlnk --version
```

**Expected:** `torlnk 0.1.0`

### 1.2 Help

```bash
./target/debug/torlnk --help
```

**Expected:** Menampilkan semua subcommand (`run`, `watch`, `serve`, `files`, `attach`, `update`) dan semua opsi global (`--dir`, `--name`, `--max-downloads`, `--serve-dir`, `--api-port`, `--files-port`, `--token`, `--json`, `--paths`).

### 1.3 Paths

```bash
./target/debug/torlnk --paths
```

**Expected:** Menampilkan `config_dir`, `data_dir`, dan `download_dir` yang sesuai platform.

### 1.4 Subcommand help

```bash
./target/debug/torlnk serve --help
./target/debug/torlnk files --help
./target/debug/torlnk watch --help
```

---

## 2. TUI Mode

Mode interaktif default. Jalankan tanpa argumen atau dengan `run`.

### 2.1 Start TUI

```bash
./target/debug/torlnk
# atau
./target/debug/torlnk run
```

**Expected:** Layar splash dengan logo `torlnk`, teks "press / to search · enter to browse · ? for keys".

### 2.2 Navigasi sidebar

| Key | Aksi |
|-----|------|
| `↑` / `↓` atau `k` / `j` | Pindah antar kategori |
| `Tab` | Switch antara sidebar dan content area |
| `Enter` | Buka kategori yang dipilih |

**Expected:** Kategori berpindah, highlight (`❯`) mengikuti. Tab berpindah fokus sidebar ↔ content.

### 2.3 Search

1. Tekan `/` — input search muncul
2. Ketik query (misal: `dune`)
3. Tekan `Enter`

**Expected:** Status "Searching..." muncul, lalu hasil dari multiple sources mengalir masuk. Setiap hasil menampilkan: tag source `[FG]`, nama, ukuran, seeders, tanggal relatif.

### 2.4 Filter dead torrents

Tekan `z` di hasil search.

**Expected:** Torrents dengan 0 seeders disembunyikan. Tekan `z` lagi untuk show.

### 2.5 Download dari hasil search

1. Pilih hasil dengan `↑`/`↓`
2. Tekan `d`

**Expected:** Torrent masuk ke Downloads, notifikasi "Downloading: ..." muncul.

### 2.6 Copy magnet

Pilih hasil, tekan `y`.

**Expected:** Notifikasi "Magnet copied to clipboard". Paste di tempat lain untuk verifikasi.

### 2.7 Downloads panel

Pilih "Downloads" di sidebar.

| Key | Aksi |
|-----|------|
| `p` | Pause/resume download |
| `c` | Cancel download |
| `e` | Buka folder download di Finder/explorer |
| `↑`/`↓` | Navigasi list |

**Expected:** Status berubah (Downloading/Paused/Queued/Failed), progress bar update, speed dan peers terlihat.

### 2.8 Seeding panel

Pilih "Seeding" di sidebar.

| Key | Aksi |
|-----|------|
| `p` | Stop seeding |
| `c` | Remove seed |
| `e` | Buka folder seed |

**Expected:** Completed downloads muncul di sini dengan upload speed dan peers.

### 2.9 Help overlay

Tekan `?` di mana saja.

**Expected:** Overlay popup dengan 4 grup keybindings (Navigate, Search, Downloads, Seeding). Tekan `Esc` atau `?` untuk tutup.

### 2.10 Quit

Tekan `q` atau `Ctrl+C`.

**Expected:** TUI keluar, state di-persist ke disk, terminal kembali normal.

---

## 3. Serve Mode (HTTP API)

REST API untuk kontrol remote.

### 3.1 Start server

```bash
./target/debug/torlnk serve --port 9161 --dir ~/Downloads/torlnk-rs
```

**Expected:** Log "serve API on 0.0.0.0:9161", server berjalan.

### 3.2 Health check

```bash
curl http://127.0.0.1:9161/health
```

**Expected:** `ok`

### 3.3 Search via API

```bash
curl "http://127.0.0.1:9161/search?q=dune"
```

**Expected:** JSON array dengan results: `name`, `source`, `seeders`, `leechers`, `size_bytes`, `magnet`, `id`.

### 3.4 List downloads

```bash
curl http://127.0.0.1:9161/downloads
```

**Expected:** JSON array dari queue items (kosong `[]` jika belum ada).

### 3.5 List seeds

```bash
curl http://127.0.0.1:9161/seeds
```

**Expected:** JSON array dari seed items.

### 3.6 Download via API

```bash
curl -X POST http://127.0.0.1:9161/download \
  -H "Content-Type: application/json" \
  -d '{"input":"magnet:?xt=urn:btih:HASH&dn=Name"}'
```

**Expected:** Response `added`, torrent muncul di `/downloads`.

### 3.7 Pause / Resume / Cancel / Remove

```bash
curl -X POST http://127.0.0.1:9161/pause/HASH
curl -X POST http://127.0.0.1:9161/resume/HASH
curl -X POST http://127.0.0.1:9161/cancel/HASH
curl -X POST http://127.0.0.1:9161/remove/HASH
```

### 3.8 Dengan token auth

```bash
./target/debug/torlnk serve --port 9161 --dir ~/Downloads --token mysecret
curl "http://127.0.0.1:9161/downloads?token=mysecret"
curl http://127.0.0.1:9161/downloads  # tanpa token → 401 Unauthorized
```

---

## 4. Files Mode (HTTP File Server)

Browse file hasil download via HTTP.

### 4.1 Start server

```bash
mkdir -p /tmp/torlnk-files
echo "hello" > /tmp/torlnk-files/test.txt
./target/debug/torlnk files --port 9160 --dir /tmp/torlnk-files
```

### 4.2 List files

```bash
curl http://127.0.0.1:9160/
```

**Expected:** JSON array `[{"name":"test.txt","is_dir":false}]`

### 4.3 Download file

```bash
curl http://127.0.0.1:9160/test.txt
```

**Expected:** Isi file (`hello`).

### 4.4 Browse subdirectory

```bash
mkdir -p /tmp/torlnk-files/subdir
echo "nested" > /tmp/torlnk-files/subdir/nested.txt
curl http://127.0.0.1:9160/subdir
```

**Expected:** JSON listing isi subdirectory.

### 4.5 Path traversal protection

```bash
curl http://127.0.0.1:9160/../../etc/passwd
```

**Expected:** `403 Forbidden` atau `404 Not Found`. Tidak boleh mengakses file di luar `--dir`.

### 4.6 Dengan token auth

```bash
./target/debug/torlnk files --port 9160 --dir /tmp/torlnk-files --token secret
curl "http://127.0.0.1:9160/?token=secret"  # OK
curl http://127.0.0.1:9160/                   # 401
```

---

## 5. Watch Mode (Folder Watcher)

Download otomatis dari file `.torrent` yang di-drop ke folder.

### 5.1 Start watcher

```bash
mkdir -p /tmp/torlnk-watch
./target/debug/torlnk watch --dir /tmp/torlnk-watch
```

**Expected:** Log "watching /tmp/torlnk-watch for .torrent files".

### 5.2 Drop .torrent file

```bash
# Salin file .torrent ke folder watch
cp some.torrent /tmp/torlnk-watch/
```

**Expected:** Log "watch: adding NamaTorrent", torrent mulai download.

### 5.3 Multiple files

Drop beberapa `.torrent` sekaligus. Semua harus terdeteksi dan didownload.

---

## 6. Attach Mode (tmux)

### 6.1 Prasyarat

```bash
# Pastikan tmux terinstall
which tmux
# macOS: brew install tmux
# Ubuntu: sudo apt install tmux
```

### 6.2 Attach

```bash
./target/debug/torlnk attach
```

**Expected:** Jika session `torlnk` belum ada, buat session baru dan jalankan TUI. Jika sudah ada, attach ke session tersebut.

### 6.3 Tanpa tmux

Jika tmux tidak terinstall:

```bash
# Simulasi: uninstall/hide tmux
PATH=/usr/bin ./target/debug/torlnk attach
```

**Expected:** Pesan "tmux is not installed" dengan instruksi instalasi.

---

## 7. Headless Input (Magnet/Torrent)

Download langsung dari CLI tanpa TUI.

### 7.1 Magnet link

```bash
./target/debug/torlnk "magnet:?xt=urn:btih:08ada5a7a6183aae1e09d831df6748d566095a10&dn=Sintel"
```

**Expected:** Output `Added: Sintel → /path/to/download_dir`, torrent mulai download.

### 7.2 Torrent file

```bash
./target/debug/torlnk /path/to/file.torrent
```

### 7.3 Dengan custom name dan dir

```bash
./target/debug/torlnk --name "My Download" --dir ~/Downloads "magnet:?xt=urn:btih:HASH" 
```

### 7.4 Bare info hash

```bash
./target/debug/torlnk 08ada5a7a6183aae1e09d831df6748d566095a10
```

**Expected:** Hash di-normalize, dibungkus dengan default trackers, didownload.

---

## 8. JSON Search Output

Search non-interaktif, output JSON.

```bash
./target/debug/torlnk --json "dune"
```

**Expected:** JSON array pretty-printed dengan fields: `name`, `source`, `seeders`, `leechers`, `size_bytes`, `magnet`, `id`.

```bash
# Pipe ke jq untuk filter
./target/debug/torlnk --json "dune" | jq '.[] | select(.seeders > 100)'
```

---

## 9. Persistensi & Bootguard

### 9.1 State disimpan saat quit

1. Start TUI, download sesuatu
2. Quit dengan `q`
3. Start TUI lagi

**Expected:** Download yang belum selesai muncul kembali dengan status yang sesuai (Paused jika sedang downloading saat quit).

### 9.2 Bootguard (crash recovery)

1. Start TUI, mulai download
2. Kill process dengan `kill -9 <pid>` (simulate crash)
3. Start TUI lagi

**Expected:** Pesan "recovered from a crashed start · downloads paused". Semua download yang sedang aktif di-pause (safe mode).

### 9.3 History

Setelah download selesai:

```bash
cat ~/Library/Application\ Support/torlnk-rs/history.json
```

**Expected:** JSON array dengan entry yang berisi `id`, `name`, `source`, `size_bytes`, `magnet`, `dir`, `completed_at`.

### 9.4 Queue file

```bash
cat ~/Library/Application\ Support/torlnk-rs/queue.json
```

### 9.5 Seeds file

```bash
cat ~/Library/Application\ Support/torlnk-rs/seeds.json
```

---

## 10. Edge Cases

### 10.1 Search query kosong

Tekan `/` lalu `Enter` tanpa mengetik.

**Expected:** Tidak ada search yang dijalankan.

### 10.2 Search dengan karakter special

```bash
# Di TUI, search: hacker's (dengan apostrophe)
```

**Expected:** Tidak crash, URL di-encode dengan benar.

### 10.3 Invalid magnet

```bash
./target/debug/torlnk "magnet:?xt=urn:btih:invalid"
```

**Expected:** Tidak crash. Graceful error atau no-op.

### 10.4 Port sudah dipakai

```bash
./target/debug/torlnk serve --port 80
```

**Expected:** Error message yang jelas, bukan panic.

### 10.5 Folder tidak ada (watch mode)

```bash
./target/debug/torlnk watch --dir /nonexistent/path
```

**Expected:** Folder dibuat otomatis, atau error message yang jelas.

### 10.6 Max downloads concurrency

```bash
TORLINK_MAX_DOWNLOADS=2 ./target/debug/torlnk
```

Tambahkan 5 download. Hanya 2 yang Downloading, sisanya Queued. Saat satu selesai, queued berikutnya mulai.

### 10.7 Quit saat sedang download

1. Download 3 torrent
2. Tekan `q`

**Expected:** Engine dihentikan, state di-flush, terminal kembali normal. Saat restart, downloads restore dengan benar.

### 10.8 Custom state dir

```bash
TORLINK_STATE_DIR=/tmp/torlnk-state ./target/debug/torlnk --paths
```

**Expected:** `config_dir` dan `data_dir` menunjuk ke `/tmp/torlnk-state/`.

---

## Checklist Ringkas

```
[ ] CLI: --version, --help, --paths
[ ] TUI: splash, sidebar nav, search, download, pause, cancel
[ ] TUI: help overlay (?), quit (q)
[ ] TUI: results dengan hide dead (z), copy magnet (y)
[ ] Serve: health, search, downloads, seeds, download, pause/resume/cancel
[ ] Serve: token auth
[ ] Files: list, serve file, subdirectory, path traversal blocked
[ ] Files: token auth
[ ] Watch: detect .torrent file, auto-download
[ ] Attach: tmux session create + attach
[ ] Headless: magnet input, .torrent file, bare hash
[ ] JSON: search output
[ ] Persist: queue/seeds/history saved on quit
[ ] Bootguard: crash → safe mode restore
[ ] Concurrency: max_downloads cap respected
[ ] Edge: empty search, invalid magnet, custom state dir
```
