use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedAlbum {
    pub title: String,
    pub artist: String,
    pub url: String,
    pub artist_url: Option<String>,
    pub tags: Vec<String>,
    pub label: Option<String>,
    pub release_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collector {
    pub handle: String,
    pub url: String,
    pub display_name: Option<String>,
    pub visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnedAlbum {
    pub title: String,
    pub artist: String,
    pub url: String,
    pub tags: Vec<String>,
    pub label: Option<String>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveOutput {
    pub seed: SeedAlbum,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectorsOutput {
    pub seed: SeedAlbum,
    pub collectors_discovered: usize,
    pub collectors: Vec<Collector>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryOutput {
    pub collector_url: String,
    pub albums: Vec<OwnedAlbum>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigOutput {
    pub seed: SeedAlbum,
    pub summary: CrawlSummary,
    pub results: Vec<CandidateRecord>,
}
