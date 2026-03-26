//! Output rendering for CLI commands.

use crate::error::Result;
use crate::model::{
    CandidateRecord, CollectorsOutput, DigOutput, ItemKind, LibraryOutput, ResolveOutput,
};

/// Rendering format requested by the user.
pub enum OutputFormat {
    Table,
    Json,
    Csv,
}

/// Print `resolve` output in the requested format.
pub fn print_resolve(output: &ResolveOutput, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(output)?),
        OutputFormat::Table | OutputFormat::Csv => {
            println!("Platform: {}", output.seed.platform.as_str());
            println!(
                "Type    : {}",
                match output.seed.kind {
                    ItemKind::Album => "album",
                    ItemKind::Track => "track",
                    ItemKind::Playlist => "playlist",
                }
            );
            println!("Title   : {}", output.seed.title);
            println!("Artist  : {}", output.seed.artist);
            println!("URL     : {}", output.seed.url);
            if !output.seed.tags.is_empty() {
                println!("Tags    : {}", output.seed.tags.join(", "));
            }
        }
    }
    Ok(())
}

/// Print `collectors` output in the requested format.
pub fn print_collectors(output: &CollectorsOutput, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(output)?),
        OutputFormat::Csv => {
            let mut writer = csv::Writer::from_writer(std::io::stdout());
            writer.write_record(["handle", "url", "display_name"])?;
            for collector in &output.collectors {
                writer.write_record([
                    collector.handle.as_str(),
                    collector.url.as_str(),
                    collector.display_name.as_deref().unwrap_or(""),
                ])?;
            }
            writer.flush()?;
        }
        OutputFormat::Table => {
            println!("Seed: {} - {}", output.seed.artist, output.seed.title);
            println!("Collectors discovered: {}", output.collectors_discovered);
            for collector in &output.collectors {
                println!(
                    "- {} {}",
                    collector.handle,
                    collector.display_name.as_deref().unwrap_or("")
                );
            }
        }
    }
    Ok(())
}

/// Print `library` output in the requested format.
pub fn print_library(output: &LibraryOutput, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(output)?),
        OutputFormat::Csv => {
            let mut writer = csv::Writer::from_writer(std::io::stdout());
            writer.write_record(["artist", "album", "url"])?;
            for album in &output.albums {
                writer.write_record([
                    album.artist.as_str(),
                    album.title.as_str(),
                    album.url.as_str(),
                ])?;
            }
            writer.flush()?;
        }
        OutputFormat::Table => {
            println!("Collector: {}", output.collector_url);
            for album in &output.albums {
                println!("- {} - {}", album.artist, album.title);
            }
        }
    }
    Ok(())
}

/// Print `dig` output in the requested format.
pub fn print_dig(output: &DigOutput, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(output)?),
        OutputFormat::Csv => print_results_csv(&output.results)?,
        OutputFormat::Table => print_results_table(output),
    }
    Ok(())
}

fn print_results_csv(results: &[CandidateRecord]) -> Result<()> {
    let mut writer = csv::Writer::from_writer(std::io::stdout());
    writer.write_record([
        "rank",
        "artist",
        "album",
        "url",
        "overlap_count",
        "overlap_ratio",
        "score",
        "reason",
    ])?;
    for result in results {
        writer.write_record([
            result.rank.to_string(),
            result.artist.clone(),
            result.title.clone(),
            result.url.clone(),
            result.overlap_count.to_string(),
            format!("{:.4}", result.overlap_ratio),
            format!("{:.2}", result.score),
            result.reason.clone(),
        ])?;
    }
    writer.flush()?;
    Ok(())
}

fn print_results_table(output: &DigOutput) {
    println!("Seed: {} - {}", output.seed.artist, output.seed.title);
    println!(
        "Sources: discovered={} sampled={} scanned={} skipped={}",
        output.summary.collectors_discovered,
        output.summary.collectors_sampled,
        output.summary.collectors_scanned,
        output.summary.collectors_skipped
    );
    println!();
    println!(
        "{:<4} {:<24} {:<28} {:>8} {:>8} {:>8}  Reason",
        "#", "Artist", "Album", "Overlap", "Pct", "Score"
    );
    for item in &output.results {
        println!(
            "{:<4} {:<24} {:<28} {:>8} {:>7.1}% {:>8.2}  {}",
            item.rank,
            truncate(&item.artist, 24),
            truncate(&item.title, 28),
            item.overlap_count,
            item.overlap_ratio * 100.0,
            item.score,
            item.reason
        );
    }
}

fn truncate(value: &str, width: usize) -> String {
    let mut chars = value.chars();
    let collected: String = chars.by_ref().take(width).collect();
    if chars.next().is_some() && width > 1 {
        format!("{}…", collected.chars().take(width - 1).collect::<String>())
    } else {
        collected
    }
}
