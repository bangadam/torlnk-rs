# Single tokio runtime with mpsc channels for UI updates

torlink's original Ink/React architecture is reactive — state changes automatically trigger re-renders. ratatui is immediate-mode: the entire UI is redrawn each frame from a central state. We run one tokio runtime that drives the torrent engine (librqbit), source scraping (reqwest), and the UI event loop. Keyboard events from crossterm are fed into a tokio channel; engine and scraping tasks send state updates through a separate mpsc channel to the UI task, which mutates `AppState` and triggers a redraw.

The alternative was a dual-thread model (UI on main thread with blocking `event::read()`, engine on a separate tokio runtime). We rejected it because crossing a thread boundary with two runtimes adds complexity without benefit, and tokio's `spawn` + `select!` pattern is the idiomatic way to multiplex async I/O with terminal events in Rust TUI apps.
