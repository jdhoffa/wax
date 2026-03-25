use rand::seq::SliceRandom;
use tracing_subscriber::EnvFilter;

use crate::cli::{CacheCommands, Cli, Commands, DigArgs, SampleMode};
use crate::config::Settings;
use crate::error::{AppError, Result};
use crate::fetch::Fetcher;
use crate::model::{CollectorsOutput, CrawlSummary, DigOutput, LibraryOutput, ResolveOutput};
use crate::output::{print_collectors, print_dig, print_library, print_resolve, OutputFormat};
use crate::parser::{normalize_url, parse_collectors, parse_owned_albums, resolve_seed};
use crate::score::{rank_candidates, ScoreOptions};

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
            let output = resolve_command(&mut fetcher, &album_url).await?;
            print_resolve(&output, format)?;
        }
        Commands::Collectors(args) => {
            let mut fetcher = Fetcher::new(&settings).await?;
            let output = collectors_command(&mut fetcher, &args).await?;
            print_collectors(&output, format)?;
        }
        Commands::Library { fan_url, limit } => {
            let mut fetcher = Fetcher::new(&settings).await?;
            let output = library_command(&mut fetcher, &fan_url, limit).await?;
            print_library(&output, format)?;
        }
        Commands::Dig(args) => {
            let mut fetcher = Fetcher::new(&settings).await?;
            let output = dig_command(&mut fetcher, &args).await?;
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

async fn resolve_command(fetcher: &mut Fetcher, album_url: &str) -> Result<ResolveOutput> {
    let normalized = normalize_url(album_url)?;
    let html = fetcher.fetch_text(&normalized).await?;
    let seed = resolve_seed(&normalized, &html)?;
    Ok(ResolveOutput { seed })
}

async fn collectors_command(fetcher: &mut Fetcher, args: &DigArgs) -> Result<CollectorsOutput> {
    let resolved = resolve_command(fetcher, &args.album_url).await?;
    let html = fetcher.fetch_text(&resolved.seed.url).await?;
    let collectors = sample_collectors(parse_collectors(&html), args.max_collectors, args.sample);
    if collectors.is_empty() {
        return Err(AppError::NoPublicData);
    }

    Ok(CollectorsOutput {
        seed: resolved.seed,
        collectors_discovered: collectors.len(),
        collectors,
    })
}

async fn library_command(fetcher: &mut Fetcher, fan_url: &str, limit: usize) -> Result<LibraryOutput> {
    let normalized = normalize_url(fan_url)?;
    let html = fetcher.fetch_text(&normalized).await?;
    let mut albums = parse_owned_albums(&html);
    albums.sort_by(|a, b| a.artist.cmp(&b.artist).then_with(|| a.title.cmp(&b.title)));
    albums.truncate(limit);

    if albums.is_empty() {
        return Err(AppError::NoPublicData);
    }

    Ok(LibraryOutput {
        collector_url: normalized,
        albums,
    })
}

async fn dig_command(fetcher: &mut Fetcher, args: &DigArgs) -> Result<DigOutput> {
    let resolved = resolve_command(fetcher, &args.album_url).await?;
    let seed_html = fetcher.fetch_text(&resolved.seed.url).await?;
    let discovered_collectors = parse_collectors(&seed_html);
    let sampled_collectors =
        sample_collectors(discovered_collectors.clone(), args.max_collectors, args.sample);

    if sampled_collectors.is_empty() {
        return Err(AppError::NoPublicData);
    }

    let mut collector_albums = Vec::new();
    let mut collectors_scanned = 0usize;
    let mut collectors_skipped = 0usize;

    for collector in &sampled_collectors {
        match library_command(fetcher, &collector.url, usize::MAX).await {
            Ok(library) => {
                collectors_scanned += 1;
                collector_albums.push((collector.handle.clone(), library.albums));
            }
            Err(AppError::NoPublicData) => collectors_skipped += 1,
            Err(_) => collectors_skipped += 1,
        }
    }

    let results = rank_candidates(
        &resolved.seed,
        collector_albums,
        &ScoreOptions {
            min_overlap: args.min_overlap,
            exclude_artist: args.exclude_artist,
            exclude_label: args.exclude_label,
            required_tags: args.tag.clone(),
            sort: args.sort,
            limit: args.limit,
        },
    );

    if results.is_empty() {
        return Err(AppError::NoPublicData);
    }

    let summary = CrawlSummary {
        collectors_discovered: discovered_collectors.len(),
        collectors_sampled: sampled_collectors.len(),
        collectors_scanned,
        collectors_skipped,
        candidates_ranked: results.len(),
        cache_hits: fetcher.stats.hits,
        cache_misses: fetcher.stats.misses,
    };

    Ok(DigOutput {
        seed: resolved.seed,
        summary,
        results,
    })
}

fn sample_collectors(
    mut collectors: Vec<crate::model::Collector>,
    max_collectors: usize,
    sample_mode: SampleMode,
) -> Vec<crate::model::Collector> {
    if let SampleMode::Random = sample_mode {
        let mut rng = rand::thread_rng();
        collectors.shuffle(&mut rng);
    }

    collectors.truncate(max_collectors);
    collectors
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
