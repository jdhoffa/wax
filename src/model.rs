//! Shared data model used across providers, ranking, and output rendering.

use serde::{Deserialize, Serialize};

/// Supported upstream platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Bandcamp,
    Soundcloud,
}

impl Platform {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bandcamp => "bandcamp",
            Self::Soundcloud => "soundcloud",
        }
    }
}

/// High-level type of resolved item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    Album,
    Track,
    Playlist,
}

/// Canonical metadata for the seed item a command operates on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedAlbum {
    pub platform: Platform,
    pub kind: ItemKind,
    pub title: String,
    pub artist: String,
    pub url: String,
    pub artist_url: Option<String>,
    pub tags: Vec<String>,
    pub label: Option<String>,
    pub release_id: Option<String>,
}

/// Public collector or liker that can act as a discovery source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collector {
    pub handle: String,
    pub url: String,
    pub display_name: Option<String>,
    pub visible: bool,
}

/// Normalized record discovered in a collector or likes library.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnedAlbum {
    pub platform: Platform,
    pub kind: ItemKind,
    pub title: String,
    pub artist: String,
    pub url: String,
    pub tags: Vec<String>,
    pub label: Option<String>,
}

/// Ranked recommendation produced by `dig`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateRecord {
    pub rank: usize,
    pub title: String,
    pub artist: String,
    pub url: String,
    pub overlap_count: usize,
    pub overlap_ratio: f64,
    pub score: f64,
    pub reason: String,
    pub collectors: Vec<String>,
}

/// Summary counters collected while crawling a provider workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlSummary {
    pub collectors_discovered: usize,
    pub collectors_sampled: usize,
    pub collectors_scanned: usize,
    pub collectors_skipped: usize,
    pub candidates_ranked: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

/// Output payload for `resolve`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveOutput {
    pub seed: SeedAlbum,
}

/// Output payload for `collectors`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectorsOutput {
    pub seed: SeedAlbum,
    pub collectors_discovered: usize,
    pub collectors: Vec<Collector>,
}

/// Output payload for `library`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryOutput {
    pub collector_url: String,
    pub albums: Vec<OwnedAlbum>,
}

/// Output payload for `dig`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigOutput {
    pub seed: SeedAlbum,
    pub summary: CrawlSummary,
    pub results: Vec<CandidateRecord>,
}
