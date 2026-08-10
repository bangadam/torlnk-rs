use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "torlnk",
    version,
    about = "Curated torrents, straight from your terminal",
    long_about = None,
)]
pub struct Cli {
    #[command(subcommand)]
    pub mode: Option<Mode>,

    /// Directory to save downloads.
    #[arg(long, global = true)]
    pub dir: Option<String>,

    /// Override the download name (used with magnet/file input).
    #[arg(long, global = true)]
    pub name: Option<String>,

    /// Maximum concurrent downloads (0 = unlimited).
    #[arg(long, global = true, env = "TORLINK_MAX_DOWNLOADS")]
    pub max_downloads: Option<usize>,

    /// Output folder for --serve and --files modes.
    #[arg(long, global = true)]
    pub serve_dir: Option<String>,

    /// Port for --serve mode API.
    #[arg(long, global = true, env = "TORLINK_API_PORT")]
    pub api_port: Option<u16>,

    /// Port for --files mode.
    #[arg(long, global = true, env = "TORLINK_FILES_PORT")]
    pub files_port: Option<u16>,

    /// Token for --serve and --files authentication.
    #[arg(long, global = true, env = "TORLINK_TOKEN")]
    pub token: Option<String>,

    /// Run the search query non-interactively and print results as JSON.
    #[arg(long, global = true)]
    pub json: bool,

    /// Print paths and exit (debug).
    #[arg(long, global = true)]
    pub paths: bool,

    /// Positional input: magnet link, .torrent file path, or search query.
    pub input: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Mode {
    /// Start the interactive TUI (default).
    Run,
    /// Watch a folder for .torrent files and download them automatically.
    Watch {
        /// Folder to watch for .torrent files.
        #[arg(long, short)]
        dir: Option<String>,
    },
    /// Start a headless HTTP API server for remote control.
    Serve {
        #[arg(long, short)]
        port: Option<u16>,
        #[arg(long)]
        dir: Option<String>,
        #[arg(long)]
        token: Option<String>,
    },
    /// Start a headless HTTP file server for browsing completed downloads.
    Files {
        #[arg(long, short)]
        port: Option<u16>,
        #[arg(long)]
        dir: Option<String>,
        #[arg(long)]
        token: Option<String>,
    },
    /// Attach to a running daemon (tmux session).
    Attach,
    /// Show the version and update info.
    Update,
}
