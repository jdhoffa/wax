use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

/// Command-line interface for the `wax` discovery tool.
#[derive(Debug, Parser)]
#[command(
    name = "wax",
    version,
    about = "Dig through public music discovery signals from Bandcamp, SoundCloud, and YouTube",
    long_about = "Dig through public music discovery signals from Bandcamp, SoundCloud, and YouTube.\n\nUse `resolve` to inspect a supported URL, `dig` to rank nearby recommendations, `collectors` and `library` for Bandcamp-only exploration, and `cache` to inspect or clear local fetch state."
)]
pub struct Cli {
    /// Load shared request settings from a TOML config file.
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,
    /// Override the on-disk cache directory.
    #[arg(long, global = true)]
    pub cache_dir: Option<PathBuf>,
    /// Print structured JSON instead of table output.
    #[arg(long, global = true)]
    pub json: bool,
    /// Print CSV output where the command supports it.
    #[arg(long, global = true)]
    pub csv: bool,
    /// Enable debug logging.
    #[arg(long, short, global = true)]
    pub verbose: bool,
    /// Suppress non-error log output.
    #[arg(long, global = true)]
    pub quiet: bool,
    /// Override the HTTP user agent used for requests.
    #[arg(long, global = true)]
    pub user_agent: Option<String>,
    /// YouTube Data API key. Can also be provided via `YOUTUBE_API_KEY`.
    #[arg(long, global = true)]
    pub youtube_api_key: Option<String>,
    /// Delay between outbound requests in milliseconds.
    #[arg(long, global = true)]
    pub rate_limit_ms: Option<u64>,
    /// Reserved request concurrency setting loaded into runtime settings.
    #[arg(long, global = true)]
    pub concurrency: Option<usize>,
    /// Per-request timeout in milliseconds.
    #[arg(long, global = true)]
    pub timeout_ms: Option<u64>,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Resolve a supported URL into canonical seed metadata.
    Resolve {
        /// Bandcamp, SoundCloud, or YouTube URL.
        album_url: String,
    },
    /// List public Bandcamp collectors discovered for a seed album.
    Collectors(DigArgs),
    /// List albums from a public Bandcamp fan page.
    Library {
        /// Public Bandcamp fan URL.
        fan_url: String,
        /// Maximum number of albums to print.
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    /// Rank recommendations for a Bandcamp album, SoundCloud track, or YouTube video URL.
    Dig(DigArgs),
    /// Inspect or clear the local fetch cache.
    Cache {
        #[command(subcommand)]
        command: CacheCommands,
    },
}

#[derive(Debug, Subcommand)]
pub enum CacheCommands {
    /// Print cache directory, entry count, and size on disk.
    Stats,
    /// Remove cached responses from the local cache directory.
    Clear,
}

/// Shared arguments for discovery-style commands.
#[derive(Debug, Clone, Args)]
pub struct DigArgs {
    /// Seed URL. Bandcamp expects albums; SoundCloud expects tracks; YouTube expects videos.
    pub album_url: String,
    /// Maximum number of source collectors or likers to sample.
    #[arg(long, default_value_t = 75)]
    pub max_collectors: usize,
    /// Reserved crawl-depth flag. Current implementations use a single hop.
    #[arg(long, default_value_t = 1)]
    pub max_depth: usize,
    /// Maximum number of ranked results to print.
    #[arg(long, default_value_t = 25)]
    pub limit: usize,
    /// Keep discovery order or shuffle sources before truncating.
    #[arg(long, value_enum, default_value_t = SampleMode::Top)]
    pub sample: SampleMode,
    /// Minimum overlap count required for a result to be kept.
    #[arg(long, default_value_t = 2)]
    pub min_overlap: usize,
    /// Exclude candidates by the same artist as the seed.
    #[arg(long)]
    pub exclude_artist: bool,
    /// Exclude candidates on the same label as the seed when label data exists.
    #[arg(long)]
    pub exclude_label: bool,
    /// Require one or more tags to be present on candidate records.
    #[arg(long)]
    pub tag: Vec<String>,
    /// Sort recommendations by weighted score or raw overlap count.
    #[arg(long, value_enum, default_value_t = SortMode::Score)]
    pub sort: SortMode,
}

/// Sampling strategy for discovered source collectors or likers.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SampleMode {
    Top,
    Random,
}

/// Result ordering for ranked recommendations.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SortMode {
    Score,
    Overlap,
}
