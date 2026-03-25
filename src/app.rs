use tracing_subscriber::EnvFilter;

use crate::cli::{CacheCommands, Cli, Commands};
use crate::config::Settings;
use crate::error::Result;
use crate::fetch::Fetcher;
use crate::output::{print_collectors, print_dig, print_library, print_resolve, OutputFormat};
use crate::provider;

pub async fn run(cli: Cli) -> Result<()> {
    init_tracing(cli.verbose, cli.quiet);
    let settings = Settings::load(&cli)?;
    let format = if cli.csv {
        OutputFormat::Csv
    } else if cli.json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };

    match cli.command {
        Commands::Resolve { album_url } => {
            let mut fetcher = Fetcher::new(&settings).await?;
            let output = provider::resolve_command(&mut fetcher, &album_url).await?;
            print_resolve(&output, format)?;
        }
        Commands::Collectors(args) => {
            let mut fetcher = Fetcher::new(&settings).await?;
            let output = provider::collectors_command(&mut fetcher, &args).await?;
            print_collectors(&output, format)?;
        }
        Commands::Library { fan_url, limit } => {
            let mut fetcher = Fetcher::new(&settings).await?;
            let output = provider::library_command(&mut fetcher, &fan_url, limit).await?;
            print_library(&output, format)?;
        }
        Commands::Dig(args) => {
            let mut fetcher = Fetcher::new(&settings).await?;
            let output = provider::dig_command(&mut fetcher, &args).await?;
            print_dig(&output, format)?;
        }
        Commands::Cache { command } => {
            let fetcher = Fetcher::new(&settings).await?;
            match command {
                CacheCommands::Stats => {
                    let (entries, bytes) = fetcher.cache().stats().await?;
                    println!("Cache dir : {}", fetcher.cache().root().display());
                    println!("Entries   : {entries}");
                    println!("Bytes     : {bytes}");
                }
                CacheCommands::Clear => {
                    fetcher.cache().clear().await?;
                    println!("Cleared {}", fetcher.cache().root().display());
                }
            }
        }
    }

    Ok(())
}

fn init_tracing(verbose: bool, quiet: bool) {
    let level = if quiet {
        "error"
    } else if verbose {
        "debug"
    } else {
        "info"
    };

    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(level))
        .with_target(false)
        .without_time()
        .try_init();
}
