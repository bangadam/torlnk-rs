pub mod bittorrented;
pub mod eztv;
pub mod fitgirl;
pub mod magnet;
pub mod nyaa;
pub mod piratebay;
pub mod registry;
pub mod rss;
pub mod subsplease;
pub mod types;
pub mod x1337;
pub mod yts;

pub use magnet::{build_magnet, is_info_hash, parse_input, parse_magnet, ParsedMagnet};
pub use registry::{all_sources, sources_by_group};
pub use types::{Source, SourceError, SourceGroup, SourceId, SourceResult, TorrentResult};
