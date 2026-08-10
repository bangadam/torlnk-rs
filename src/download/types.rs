use crate::sources::SourceId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStatus {
    Downloading,
    Queued,
    Paused,
    Completed,
    Failed,
}

impl std::fmt::Display for DownloadStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Downloading => write!(f, "downloading"),
            Self::Queued => write!(f, "queued"),
            Self::Paused => write!(f, "paused"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SeedStatus {
    Seeding,
    Paused,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceId>,
    pub magnet: String,
    pub dir: String,
    pub status: DownloadStatus,
    pub progress: u8,
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub speed: u64,
    pub peers: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub added_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedItem {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceId>,
    pub magnet: String,
    pub dir: String,
    pub size_bytes: u64,
    pub status: SeedStatus,
    pub upload_speed: u64,
    pub uploaded: u64,
    pub peers: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceId>,
    pub size_bytes: u64,
    pub magnet: String,
    pub dir: String,
    pub completed_at: i64,
}

/// Persisted seed record — only id + status survive a restart.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedRecord {
    pub id: String,
    pub status: PersistedSeedStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PersistedSeedStatus {
    Seeding,
    Paused,
}
