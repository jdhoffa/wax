use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    Album,
    Track,
    Playlist,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collector {
    pub handle: String,
    pub url: String,
    pub display_name: Option<String>,
    pub visible: bool,
}

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
