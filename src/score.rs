//! Candidate aggregation and ranking.

use std::collections::HashMap;

use crate::cli::SortMode;
use crate::model::{CandidateRecord, OwnedAlbum, SeedAlbum};

/// Options that control filtering and ranking behavior.
#[derive(Debug, Clone)]
pub struct ScoreOptions {
    pub min_overlap: usize,
    pub exclude_artist: bool,
    pub exclude_label: bool,
    pub required_tags: Vec<String>,
    pub source_label_plural: &'static str,
    pub sort: SortMode,
    pub limit: usize,
}

#[derive(Debug, Default)]
struct Aggregate {
    album: Option<OwnedAlbum>,
    collectors: Vec<String>,
}

/// Aggregate collector evidence into ranked recommendation rows.
pub fn rank_candidates(
    seed: &SeedAlbum,
    collector_albums: Vec<(String, Vec<OwnedAlbum>)>,
    options: &ScoreOptions,
) -> Vec<CandidateRecord> {
    let mut aggregates: HashMap<String, Aggregate> = HashMap::new();
    let scanned = collector_albums.len().max(1);

    for (collector, albums) in collector_albums {
        for album in albums {
            if album.url == seed.url {
                continue;
            }
            if options.exclude_artist && album.artist.eq_ignore_ascii_case(&seed.artist) {
                continue;
            }
            if options.exclude_label
                && seed.label.is_some()
                && album.label.is_some()
                && seed.label == album.label
            {
                continue;
            }
            if !options.required_tags.is_empty()
                && !options.required_tags.iter().all(|tag| {
                    album
                        .tags
                        .iter()
                        .any(|value| value.eq_ignore_ascii_case(tag))
                })
            {
                continue;
            }

            let entry = aggregates.entry(album.url.clone()).or_default();
            if entry.album.is_none() {
                entry.album = Some(album);
            }
            entry.collectors.push(collector.clone());
        }
    }

    let mut ranked = Vec::new();
    for aggregate in aggregates.into_values() {
        let Some(album) = aggregate.album else {
            continue;
        };

        let overlap_count = aggregate.collectors.len();
        if overlap_count < options.min_overlap {
            continue;
        }

        let overlap_ratio = overlap_count as f64 / scanned as f64;
        let same_artist_penalty = if album.artist.eq_ignore_ascii_case(&seed.artist) {
            4.0
        } else {
            0.0
        };
        let score = (overlap_count as f64 * 1.5) + (overlap_ratio * 10.0) - same_artist_penalty;

        ranked.push(CandidateRecord {
            rank: 0,
            title: album.title.clone(),
            artist: album.artist.clone(),
            url: album.url,
            overlap_count,
            overlap_ratio,
            score,
            reason: format!(
                "Seen in {overlap_count} of {scanned} sampled {}",
                options.source_label_plural
            ),
            collectors: aggregate.collectors,
        });
    }

    match options.sort {
        SortMode::Score => ranked.sort_by(|a, b| b.score.total_cmp(&a.score)),
        SortMode::Overlap => ranked.sort_by(|a, b| b.overlap_count.cmp(&a.overlap_count)),
    }

    for (index, item) in ranked.iter_mut().take(options.limit).enumerate() {
        item.rank = index + 1;
    }

    ranked.truncate(options.limit);
    ranked
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ItemKind, Platform};

    #[test]
    fn ranks_candidates_by_overlap() {
        let seed = SeedAlbum {
            platform: Platform::Bandcamp,
            kind: ItemKind::Album,
            title: "Seed".to_string(),
            artist: "Seed Artist".to_string(),
            url: "https://seed.bandcamp.com/album/seed".to_string(),
            artist_url: None,
            tags: vec![],
            label: None,
            release_id: None,
        };

        let albums = vec![
            (
                "fan_a".to_string(),
                vec![OwnedAlbum {
                    platform: Platform::Bandcamp,
                    kind: ItemKind::Album,
                    title: "A".to_string(),
                    artist: "Other".to_string(),
                    url: "https://x.bandcamp.com/album/a".to_string(),
                    tags: vec![],
                    label: None,
                }],
            ),
            (
                "fan_b".to_string(),
                vec![OwnedAlbum {
                    platform: Platform::Bandcamp,
                    kind: ItemKind::Album,
                    title: "A".to_string(),
                    artist: "Other".to_string(),
                    url: "https://x.bandcamp.com/album/a".to_string(),
                    tags: vec![],
                    label: None,
                }],
            ),
        ];

        let ranked = rank_candidates(
            &seed,
            albums,
            &ScoreOptions {
                min_overlap: 1,
                exclude_artist: false,
                exclude_label: false,
                required_tags: vec![],
                source_label_plural: "collectors",
                sort: SortMode::Score,
                limit: 10,
            },
        );

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].overlap_count, 2);
    }
}
