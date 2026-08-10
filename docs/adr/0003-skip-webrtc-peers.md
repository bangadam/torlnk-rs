# Skip WebRTC peer support

WebTorrent's distinguishing feature is WebRTC peer connectivity — browser-based peers that exchange torrent data over DTLS/STUN/ICE. librqbit supports only TCP and uTP (standard BitTorrent transports). We accept this gap rather than implementing WebRTC manually, which would require a full WebRTC stack (DTLS, ICE, STUN, SDP) with no existing Rust BitTorrent library support.

This means torlnk-rs cannot connect to peers running WebTorrent in a browser. The impact is small: the vast majority of BitTorrent swarms are traditional TCP/uTP clients, and browser-based seeding is rare for the content types torlnk-rs targets (games, movies, TV, anime).
