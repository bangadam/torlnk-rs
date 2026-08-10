# Use librqbit as torrent engine

The original torlink is built on WebTorrent (Node.js, EventEmitter-based, supports WebRTC peers). We're rewriting in Rust, and librqbit is the only mature Rust BitTorrent library usable as a dependency. It's async tokio-based with a `Session` API rather than EventEmitter callbacks, so we wrap it in an adapter that exposes the add/metadata/done/error semantics the queue expects. The alternative — writing a BitTorrent engine from scratch — is a multi-month project covering BEP-3/5/6/9, piece verification, and choking algorithms, which is not realistic for a clone.

## Status

Accepted

## Consequences

- No WebRTC peer support: we cannot connect to browser-based WebTorrent peers. Standard TCP/uTP swarm access is unaffected; this matters only for peers running WebTorrent in a browser, which is a small fraction of real swarms.
- The engine adapter must bridge librqbit's async `ManagedTorrentHandle` polling model to the channel-based UI update pattern we chose.
- We inherit librqbit's download directory layout and storage backend decisions.
