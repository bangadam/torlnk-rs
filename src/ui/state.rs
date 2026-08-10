#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Splash,
    Browser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    All,
    Games,
    Movies,
    Tv,
    Anime,
}

impl Category {
    pub fn label(&self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Games => "Games",
            Self::Movies => "Movies",
            Self::Tv => "TV",
            Self::Anime => "Anime",
        }
    }

    pub fn group(&self) -> Option<crate::sources::SourceGroup> {
        match self {
            Self::Games => Some(crate::sources::SourceGroup::Games),
            Self::Movies => Some(crate::sources::SourceGroup::Movies),
            Self::Tv => Some(crate::sources::SourceGroup::Tv),
            Self::Anime => Some(crate::sources::SourceGroup::Anime),
            Self::All => None,
        }
    }

    pub fn all() -> &'static [Category] {
        &[
            Category::All,
            Category::Games,
            Category::Movies,
            Category::Tv,
            Category::Anime,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Category(Category),
    Downloads,
    Seeding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    Sidebar,
    Content,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    None,
    Text,
    Esc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadFocus {
    Downloading,
    Paused,
    Failed,
    Recent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedFocus {
    Seeding,
    Paused,
    Missing,
    Idle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultFocus {
    List,
    Detail,
}

/// Severity level for a notice toast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeLevel {
    Success,
    Error,
    Warn,
    Info,
}

/// A search result being streamed in from sources.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub result: crate::sources::TorrentResult,
    pub source_label: String,
    pub source_color: ratatui::style::Color,
}

/// A notice line shown briefly to the user.
#[derive(Debug, Clone)]
pub struct Notice {
    pub message: String,
    pub level: NoticeLevel,
    pub at: i64,
}
