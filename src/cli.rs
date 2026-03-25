use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "wax", version, about = "Dig through public Bandcamp collector overlap")]
pub struct Cli {
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,
    #[arg(long, global = true)]
    pub cache_dir: Option<PathBuf>,
    #[arg(long, global = true)]
    pub json: bool,
    #[arg(long, global = true)]
    pub csv: bool,
    #[arg(long, short, global = true)]
    pub verbose: bool,
    #[arg(long, global = true)]
    pub quiet: bool,
    #[arg(long, global = true)]
    pub user_agent: Option<String>,
    #[arg(long, global = true)]
    pub rate_limit_ms: Option<u64>,
    #[arg(long, global = true)]
    pub concurrency: Option<usize>,
    #[arg(long, global = true)]
    pub timeout_ms: Option<u64>,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Resolve {
        album_url: String,
    },
    Collectors(DigArgs),
    Library {
        fan_url: String,
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    Dig(DigArgs),
    Cache {
        #[command(subcommand)]
        command: CacheCommands,
    },
}

#[derive(Debug, Subcommand)]
pub enum CacheCommands {
    Stats,
    Clear,
}

#[derive(Debug, Clone, Args)]
pub struct DigArgs {
    pub album_url: String,
    #[arg(long, default_value_t = 75)]
    pub max_collectors: usize,
    #[arg(long, default_value_t = 1)]
    pub max_depth: usize,
    #[arg(long, default_value_t = 25)]
    pub limit: usize,
    #[arg(long, value_enum, default_value_t = SampleMode::Top)]
    pub sample: SampleMode,
    #[arg(long, default_value_t = 2)]
    pub min_overlap: usize,
    #[arg(long)]
    pub exclude_artist: bool,
    #[arg(long)]
    pub exclude_label: bool,
    #[arg(long)]
    pub tag: Vec<String>,
    #[arg(long, value_enum, default_value_t = SortMode::Score)]
    pub sort: SortMode,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SampleMode {
    Top,
    Random,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SortMode {
    Score,
    Overlap,
}
