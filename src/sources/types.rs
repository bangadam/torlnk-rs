use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceId {
    Fitgirl,
    Yts,
    Eztv,
    Nyaa,
    Subsplease,
    #[serde(rename = "tpb-movies")]
    TpbMovies,
    #[serde(rename = "tpb-tv")]
    TpbTv,
    #[serde(rename = "x1337-movies")]
    X1337Movies,
    #[serde(rename = "x1337-tv")]
    X1337Tv,
    Bittorrented,
}

impl SourceId {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Fitgirl => "FitGirl",
            Self::Yts => "YTS",
            Self::Eztv => "EZTV",
            Self::Nyaa => "Nyaa",
            Self::Subsplease => "SubsPlease",
            Self::TpbMovies | Self::TpbTv => "TPB",
            Self::X1337Movies | Self::X1337Tv => "1337x",
            Self::Bittorrented => "BitTorrented",
        }
    }

    pub fn tag(&self) -> &'static str {
        match self {
            Self::Fitgirl => "FG",
            Self::Yts => "YTS",
            Self::Eztv => "EZTV",
            Self::Nyaa => "NYAA",
            Self::Subsplease => "SUB",
            Self::TpbMovies | Self::TpbTv => "TPB",
            Self::X1337Movies | Self::X1337Tv => "1337",
            Self::Bittorrented => "BT",
        }
    }
}

impl std::fmt::Display for SourceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fitgirl => write!(f, "fitgirl"),
            Self::Yts => write!(f, "yts"),
            Self::Eztv => write!(f, "eztv"),
            Self::Nyaa => write!(f, "nyaa"),
            Self::Subsplease => write!(f, "subsplease"),
            Self::TpbMovies => write!(f, "tpb-movies"),
            Self::TpbTv => write!(f, "tpb-tv"),
            Self::X1337Movies => write!(f, "x1337-movies"),
            Self::X1337Tv => write!(f, "x1337-tv"),
            Self::Bittorrented => write!(f, "bittorrented"),
        }
    }
}

impl std::str::FromStr for SourceId {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "fitgirl" => Ok(Self::Fitgirl),
            "yts" => Ok(Self::Yts),
            "eztv" => Ok(Self::Eztv),
            "nyaa" => Ok(Self::Nyaa),
            "subsplease" => Ok(Self::Subsplease),
            "tpb-movies" => Ok(Self::TpbMovies),
            "tpb-tv" => Ok(Self::TpbTv),
            "x1337-movies" => Ok(Self::X1337Movies),
            "x1337-tv" => Ok(Self::X1337Tv),
            "bittorrented" => Ok(Self::Bittorrented),
            _ => Err(format!("unknown source: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceGroup {
    Games,
    Movies,
    Tv,
    Anime,
}

impl SourceGroup {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Games => "Games",
            Self::Movies => "Movies",
            Self::Tv => "TV",
            Self::Anime => "Anime",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TorrentResult {
    pub info_hash: String,
    pub name: String,
    pub size_bytes: u64,
    pub seeders: u64,
    pub leechers: u64,
    pub num_files: Option<u64>,
    pub source: SourceId,
    pub magnet: String,
    pub added: Option<i64>,
}


impl TorrentResult {
    /// Unique ID for dedup/queue tracking = info hash.
    pub fn id(&self) -> String {
        self.info_hash.clone()
    }

    /// Magnet URI (already built with trackers by each source).
    pub fn magnet(&self) -> String {
        self.magnet.clone()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("{0}")]
    Http(#[from] crate::util::net::HttpError),
    #[error("{0}")]
    Other(String),
}

pub type SourceResult<T> = Result<T, SourceError>;

#[async_trait::async_trait]
pub trait Source: Send + Sync {
    fn id(&self) -> SourceId;
    fn label(&self) -> &str {
        self.id().label()
    }
    fn groups(&self) -> Vec<SourceGroup>;
    fn homepage(&self) -> &str;
    /// True when the source returns real swarm counts. False when its feed has
    /// none, so seeders: 0 means unknown, not dead.
    fn reports_health(&self) -> bool;
    async fn search(
        &self,
        query: &str,
        client: &reqwest::Client,
        cancel: Option<&tokio_util::sync::CancellationToken>,
    ) -> SourceResult<Vec<TorrentResult>>;
}
