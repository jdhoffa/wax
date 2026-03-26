//! Provider detection and command dispatch.
//!
//! This module contains the top-level workflows for each command. It decides
//! which provider implementation to use, runs the relevant fetch/parse steps,
//! and returns provider-neutral output structs from [`crate::model`].

use std::collections::HashMap;

use rand::seq::SliceRandom;
use url::Url;

use crate::cli::{DigArgs, SampleMode};
use crate::error::{AppError, Result};
use crate::fetch::Fetcher;
use crate::model::{
    CollectorsOutput, CrawlSummary, DigOutput, LibraryOutput, Platform, ResolveOutput,
};
use crate::parser;
use crate::progress::ProgressReporter;
use crate::score::{rank_candidates, ScoreOptions};
use crate::soundcloud;
use crate::youtube;

/// Detect the provider for a user-supplied URL.
pub fn detect_platform(url: &str) -> Result<Platform> {
    let parsed = Url::parse(url)?;
    let Some(host) = parsed.host_str() else {
        return Err(AppError::InvalidInput(format!(
            "unsupported platform for URL: {url}"
        )));
    };

    let host = host.to_ascii_lowercase();
    if host == "bandcamp.com" || host.ends_with(".bandcamp.com") {
        return Ok(Platform::Bandcamp);
    }
    if host == "soundcloud.com" || host.ends_with(".soundcloud.com") || host == "on.soundcloud.com"
    {
        return Ok(Platform::Soundcloud);
    }
    if host == "youtube.com"
        || host.ends_with(".youtube.com")
        || host == "youtu.be"
        || host.ends_with(".youtu.be")
    {
        return Ok(Platform::Youtube);
    }

    Err(AppError::InvalidInput(format!(
        "unsupported platform for URL: {url}"
    )))
}

/// Resolve a supported URL into canonical seed metadata.
pub async fn resolve_command(fetcher: &mut Fetcher, item_url: &str) -> Result<ResolveOutput> {
    match detect_platform(item_url)? {
        Platform::Bandcamp => resolve_bandcamp(fetcher, item_url).await,
        Platform::Soundcloud => resolve_soundcloud(fetcher, item_url).await,
        Platform::Youtube => resolve_youtube(fetcher, item_url).await,
    }
}

/// List collectors for a Bandcamp album.
pub async fn collectors_command(fetcher: &mut Fetcher, args: &DigArgs) -> Result<CollectorsOutput> {
    match detect_platform(&args.album_url)? {
        Platform::Bandcamp => collectors_bandcamp(fetcher, args).await,
        Platform::Soundcloud => Err(AppError::UnsupportedPlatformFeature {
            platform: Platform::Soundcloud.as_str().to_string(),
            feature: "collectors".to_string(),
        }),
        Platform::Youtube => Err(AppError::UnsupportedPlatformFeature {
            platform: Platform::Youtube.as_str().to_string(),
            feature: "collectors".to_string(),
        }),
    }
}

/// List a public Bandcamp fan library.
pub async fn library_command(
    fetcher: &mut Fetcher,
    fan_url: &str,
    limit: usize,
) -> Result<LibraryOutput> {
    match detect_platform(fan_url)? {
        Platform::Bandcamp => library_bandcamp(fetcher, fan_url, limit).await,
        Platform::Soundcloud => Err(AppError::UnsupportedPlatformFeature {
            platform: Platform::Soundcloud.as_str().to_string(),
            feature: "library".to_string(),
        }),
        Platform::Youtube => Err(AppError::UnsupportedPlatformFeature {
            platform: Platform::Youtube.as_str().to_string(),
            feature: "library".to_string(),
        }),
    }
}

/// Rank recommendations for a supported seed URL.
pub async fn dig_command(
    fetcher: &mut Fetcher,
    args: &DigArgs,
    progress: ProgressReporter,
) -> Result<DigOutput> {
    match detect_platform(&args.album_url)? {
        Platform::Bandcamp => dig_bandcamp(fetcher, args, progress).await,
        Platform::Soundcloud => dig_soundcloud(fetcher, args, progress).await,
        Platform::Youtube => dig_youtube(fetcher, args, progress).await,
    }
}

async fn resolve_bandcamp(fetcher: &mut Fetcher, album_url: &str) -> Result<ResolveOutput> {
    let normalized = parser::normalize_url(album_url)?;
    let html = fetcher.fetch_text(&normalized).await?;
    let seed = parser::resolve_seed(&normalized, &html)?;
    Ok(ResolveOutput { seed })
}

async fn resolve_soundcloud(fetcher: &mut Fetcher, track_url: &str) -> Result<ResolveOutput> {
    let normalized = soundcloud::normalize_url(track_url)?;
    let html = fetcher.fetch_text(&normalized).await?;
    let client_id = soundcloud::extract_client_id(&html)?;
    let resolve_url = soundcloud::resolve_api_url(&client_id, &normalized)?;
    let json = fetcher.fetch_text(&resolve_url).await?;
    let seed = soundcloud::resolve_api_seed(&json)?;
    Ok(ResolveOutput { seed })
}

async fn resolve_youtube(fetcher: &mut Fetcher, video_url: &str) -> Result<ResolveOutput> {
    let api_key = youtube_api_key(fetcher)?;
    let normalized = youtube::normalize_url(video_url)?;
    let video_id = youtube::extract_video_id(&normalized)?;
    let json = fetcher
        .fetch_text(&youtube::videos_url(&api_key, &[video_id])?)
        .await?;
    let seed = youtube::parse_seed(&json)?;
    Ok(ResolveOutput { seed })
}

async fn collectors_bandcamp(fetcher: &mut Fetcher, args: &DigArgs) -> Result<CollectorsOutput> {
    let resolved = resolve_bandcamp(fetcher, &args.album_url).await?;
    let html = fetcher.fetch_text(&resolved.seed.url).await?;
    let collectors = sample_collectors(
        parser::parse_collectors(&html),
        args.max_collectors,
        args.sample,
    );
    if collectors.is_empty() {
        return Err(AppError::NoPublicData);
    }

    Ok(CollectorsOutput {
        seed: resolved.seed,
        collectors_discovered: collectors.len(),
        collectors,
    })
}

async fn library_bandcamp(
    fetcher: &mut Fetcher,
    fan_url: &str,
    limit: usize,
) -> Result<LibraryOutput> {
    let normalized = parser::normalize_url(fan_url)?;
    let html = fetcher.fetch_text(&normalized).await?;
    let mut albums = parser::parse_owned_albums(&html);
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

async fn dig_bandcamp(
    fetcher: &mut Fetcher,
    args: &DigArgs,
    progress: ProgressReporter,
) -> Result<DigOutput> {
    progress.stage("Resolving Bandcamp seed...");
    let resolved = resolve_bandcamp(fetcher, &args.album_url).await?;
    let seed_html = fetcher.fetch_text(&resolved.seed.url).await?;
    let discovered_collectors = parser::parse_collectors(&seed_html);
    let sampled_collectors = sample_collectors(
        discovered_collectors.clone(),
        args.max_collectors,
        args.sample,
    );

    if sampled_collectors.is_empty() {
        return Err(AppError::NoPublicData);
    }

    progress.stage(&format!(
        "Found {} collectors, sampling {}...",
        discovered_collectors.len(),
        sampled_collectors.len()
    ));

    let mut collector_albums = Vec::new();
    let mut collectors_scanned = 0usize;
    let mut collectors_skipped = 0usize;

    for (index, collector) in sampled_collectors.iter().enumerate() {
        progress.item_progress("Scanning collectors", index + 1, sampled_collectors.len());
        match library_bandcamp(fetcher, &collector.url, usize::MAX).await {
            Ok(library) => {
                collectors_scanned += 1;
                collector_albums.push((collector.handle.clone(), library.albums));
            }
            Err(AppError::NoPublicData) => collectors_skipped += 1,
            Err(_) => collectors_skipped += 1,
        }
    }

    progress.stage("Ranking candidates...");
    let results = rank_candidates(
        &resolved.seed,
        collector_albums,
        &ScoreOptions {
            min_overlap: args.min_overlap,
            exclude_artist: args.exclude_artist,
            exclude_label: args.exclude_label,
            required_tags: args.tag.clone(),
            source_label_plural: "collectors",
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

async fn dig_soundcloud(
    fetcher: &mut Fetcher,
    args: &DigArgs,
    progress: ProgressReporter,
) -> Result<DigOutput> {
    progress.stage("Resolving SoundCloud seed...");
    let normalized = soundcloud::normalize_url(&args.album_url)?;
    let seed_html = fetcher.fetch_text(&normalized).await?;
    let client_id = soundcloud::extract_client_id(&seed_html)?;
    let resolve_url = soundcloud::resolve_api_url(&client_id, &normalized)?;
    let resolve_json = fetcher.fetch_text(&resolve_url).await?;
    let seed = soundcloud::resolve_api_seed(&resolve_json)?;

    if seed.kind != crate::model::ItemKind::Track {
        return Err(AppError::UnsupportedPlatformFeature {
            platform: Platform::Soundcloud.as_str().to_string(),
            feature: "playlist dig".to_string(),
        });
    }

    let Some(seed_track_id) = seed.release_id.clone() else {
        return Err(AppError::Parse(
            "unable to determine SoundCloud track id".to_string(),
        ));
    };
    let liker_limit = (args.max_collectors.saturating_mul(20))
        .max(args.min_overlap)
        .min(200);
    let likers_url = soundcloud::likers_url(&client_id, &seed_track_id, liker_limit)?;
    let likers_json = fetcher.fetch_text(&likers_url).await?;
    let discovered_likers = soundcloud::parse_likers(&likers_json)?;
    let discovered_count = discovered_likers.len();
    let mut sampled_sources = discovered_likers;
    if let SampleMode::Random = args.sample {
        let mut rng = rand::thread_rng();
        sampled_sources.shuffle(&mut rng);
    }
    if sampled_sources.is_empty() {
        return Err(AppError::NoPublicData);
    }

    progress.stage(&format!(
        "Found {} public likers, scanning nearby likes...",
        discovered_count
    ));

    let mut source_tracks = Vec::new();
    let mut likers_scanned = 0usize;
    let mut likers_skipped = 0usize;
    let mut likers_attempted = 0usize;

    for (index, liker) in sampled_sources.iter().enumerate() {
        if source_tracks.len() >= args.max_collectors {
            break;
        }

        progress.item_progress("Scanning likers", index + 1, sampled_sources.len());
        likers_attempted += 1;
        let mut next_url = Some(soundcloud::user_likes_url(&client_id, &liker.id, 100)?);
        let mut page_count = 0usize;
        let mut found_source = false;

        while let Some(url) = next_url.take() {
            page_count += 1;
            if page_count > 4 {
                break;
            }

            let page_url = soundcloud::with_client_id(&url, &client_id)?;
            let likes_json = fetcher.fetch_text(&page_url).await?;
            match soundcloud::parse_user_likes_page(&likes_json, liker, &seed_track_id, 2) {
                Ok(page) => {
                    if let Some(source) = page.source {
                        likers_scanned += 1;
                        source_tracks.push((source.title, source.tracks));
                        found_source = true;
                        break;
                    }
                    next_url = page.next_href;
                }
                Err(_) => {
                    next_url = None;
                }
            }
        }

        if !found_source {
            likers_skipped += 1;
        }
    }

    if source_tracks.is_empty() {
        return Err(AppError::NoPublicData);
    }

    progress.stage("Ranking candidates...");
    let results = rank_candidates(
        &seed,
        source_tracks,
        &ScoreOptions {
            min_overlap: args.min_overlap,
            exclude_artist: args.exclude_artist,
            exclude_label: args.exclude_label,
            required_tags: args.tag.clone(),
            source_label_plural: "likers",
            sort: args.sort,
            limit: args.limit,
        },
    );

    if results.is_empty() {
        return Err(AppError::NoPublicData);
    }

    let summary = CrawlSummary {
        collectors_discovered: discovered_count,
        collectors_sampled: likers_attempted,
        collectors_scanned: likers_scanned,
        collectors_skipped: likers_skipped,
        candidates_ranked: results.len(),
        cache_hits: fetcher.stats.hits,
        cache_misses: fetcher.stats.misses,
    };

    Ok(DigOutput {
        seed,
        summary,
        results,
    })
}

async fn dig_youtube(
    fetcher: &mut Fetcher,
    args: &DigArgs,
    progress: ProgressReporter,
) -> Result<DigOutput> {
    progress.stage("Resolving YouTube seed...");
    let api_key = youtube_api_key(fetcher)?;
    let normalized = youtube::normalize_url(&args.album_url).map_err(|_| {
        AppError::InvalidInput(
            "YouTube dig expects a watch URL with playlist context, for example `https://www.youtube.com/watch?v=VIDEO_ID&list=PLAYLIST_ID`.".to_string(),
        )
    })?;
    let video_id = youtube::extract_video_id(&normalized)?;
    let playlist_id = youtube::extract_playlist_id(&args.album_url)?;
    let seed_json = fetcher
        .fetch_text(&youtube::videos_url(
            &api_key,
            std::slice::from_ref(&video_id),
        )?)
        .await?;
    let seed = youtube::parse_seed(&seed_json)?;

    progress.stage("Loading playlist context...");
    let playlist_titles =
        fetch_playlist_titles(fetcher, &api_key, std::slice::from_ref(&playlist_id)).await?;
    let mut entries = Vec::new();
    let mut next_page_token = None;
    let mut page_count = 0usize;

    loop {
        page_count += 1;
        let url =
            youtube::playlist_items_url(&api_key, &playlist_id, next_page_token.as_deref(), 50)?;
        let json = fetcher.fetch_text(&url).await?;
        entries.extend(youtube::parse_playlist_page(&json)?);
        let covered_seed = entries.iter().any(|entry| entry.video_id == video_id);
        next_page_token = youtube::parse_next_page_token(&json)?;
        if next_page_token.is_none() || covered_seed || page_count >= 6 {
            break;
        }
    }

    if !entries.iter().any(|entry| entry.video_id == video_id) {
        return Err(AppError::InvalidInput(
            "The provided YouTube URL includes a playlist, but the scanned playlist pages did not include the seed video. Pass a watch URL where the `v=` video actually belongs to the `list=` playlist.".to_string(),
        ));
    }

    let title = playlist_titles.get(&playlist_id).map(String::as_str);
    let source = youtube::build_playlist_source(entries, &playlist_id, title, &video_id)?
        .ok_or(AppError::NoPublicData)?;

    progress.stage("Ranking candidates...");
    let effective_min_overlap = args.min_overlap.min(1);
    let results = rank_candidates(
        &seed,
        vec![(source.title, source.tracks)],
        &ScoreOptions {
            min_overlap: effective_min_overlap,
            exclude_artist: args.exclude_artist,
            exclude_label: args.exclude_label,
            required_tags: args.tag.clone(),
            source_label_plural: "playlists",
            sort: args.sort,
            limit: args.limit,
        },
    );

    if results.is_empty() {
        return Err(AppError::NoPublicData);
    }

    let summary = CrawlSummary {
        collectors_discovered: 1,
        collectors_sampled: 1,
        collectors_scanned: 1,
        collectors_skipped: 0,
        candidates_ranked: results.len(),
        cache_hits: fetcher.stats.hits,
        cache_misses: fetcher.stats.misses,
    };

    Ok(DigOutput {
        seed,
        summary,
        results,
    })
}

async fn fetch_playlist_titles(
    fetcher: &mut Fetcher,
    api_key: &str,
    playlist_ids: &[String],
) -> Result<HashMap<String, String>> {
    let mut titles = HashMap::new();
    for chunk in playlist_ids.chunks(50) {
        let url = youtube::playlists_url(api_key, chunk)?;
        let json = fetcher.fetch_text(&url).await?;
        titles.extend(youtube::parse_playlist_titles(&json)?);
    }
    Ok(titles)
}

fn youtube_api_key(fetcher: &Fetcher) -> Result<String> {
    fetcher.youtube_api_key().map(str::to_string).ok_or_else(|| {
        AppError::InvalidInput(
            "YouTube support requires a YouTube Data API key; pass --youtube-api-key, set YOUTUBE_API_KEY, or add youtube_api_key to config".to_string(),
        )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_bandcamp_platform() {
        assert_eq!(
            detect_platform("https://artist.bandcamp.com/album/test").unwrap(),
            Platform::Bandcamp
        );
    }

    #[test]
    fn detects_soundcloud_platform() {
        assert_eq!(
            detect_platform("https://soundcloud.com/test-user/test-track").unwrap(),
            Platform::Soundcloud
        );
    }

    #[test]
    fn detects_youtube_platform() {
        assert_eq!(
            detect_platform("https://music.youtube.com/watch?v=abc123").unwrap(),
            Platform::Youtube
        );
    }
}
