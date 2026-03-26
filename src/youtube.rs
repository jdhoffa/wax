//! YouTube-specific URL handling, API helpers, and playlist parsing.

use std::collections::HashMap;

use serde::Deserialize;
use url::Url;

use crate::error::{AppError, Result};
use crate::model::{ItemKind, OwnedAlbum, Platform, SeedAlbum};

const WATCH_BASE: &str = "https://www.youtube.com/watch";

/// Nearby candidate evidence extracted from one public playlist.
#[derive(Debug, Clone)]
pub struct PlaylistSource {
    pub id: String,
    pub title: String,
    pub url: String,
    pub tracks: Vec<OwnedAlbum>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistTrackEntry {
    pub video_id: String,
    pub position: usize,
    pub title: String,
    pub artist: String,
}

#[derive(Debug, Deserialize)]
struct VideoListResponse {
    #[serde(default)]
    items: Vec<ApiVideo>,
}

#[derive(Debug, Deserialize)]
struct ApiVideo {
    id: String,
    snippet: Option<ApiVideoSnippet>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiVideoSnippet {
    title: String,
    channel_title: Option<String>,
    channel_id: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PlaylistListResponse {
    #[serde(default)]
    items: Vec<ApiPlaylist>,
}

#[derive(Debug, Deserialize)]
struct ApiPlaylist {
    id: String,
    snippet: Option<ApiPlaylistSnippet>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiPlaylistSnippet {
    title: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlaylistItemListResponse {
    next_page_token: Option<String>,
    #[serde(default)]
    items: Vec<ApiPlaylistItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiPlaylistItem {
    snippet: Option<ApiPlaylistItemSnippet>,
    content_details: Option<ApiPlaylistItemContentDetails>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiPlaylistItemSnippet {
    title: Option<String>,
    channel_title: Option<String>,
    position: Option<usize>,
    resource_id: Option<ApiResourceId>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiResourceId {
    video_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiPlaylistItemContentDetails {
    video_id: Option<String>,
}

/// Normalize a YouTube or YouTube Music video URL to a canonical watch URL.
pub fn normalize_url(url: &str) -> Result<String> {
    let video_id = extract_video_id(url)?;
    let mut canonical = Url::parse(WATCH_BASE)?;
    canonical.query_pairs_mut().append_pair("v", &video_id);
    Ok(canonical.to_string())
}

/// Extract the canonical YouTube video id from a supported URL.
pub fn extract_video_id(url: &str) -> Result<String> {
    let parsed = Url::parse(url)?;
    let Some(host) = parsed.host_str() else {
        return Err(AppError::InvalidInput(format!(
            "unsupported YouTube URL: {url}"
        )));
    };

    let host = host.to_ascii_lowercase();
    let video_id = match host.as_str() {
        "youtu.be" | "www.youtu.be" => parsed
            .path_segments()
            .and_then(|mut segments| segments.next())
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        "youtube.com" | "www.youtube.com" | "m.youtube.com" | "music.youtube.com" => {
            let path = parsed.path().trim_end_matches('/');
            if path == "/watch" {
                parsed
                    .query_pairs()
                    .find(|(key, _)| key == "v")
                    .map(|(_, value)| value.to_string())
            } else {
                None
            }
        }
        _ => None,
    };

    let Some(video_id) = video_id.filter(|value| !value.trim().is_empty()) else {
        return Err(AppError::InvalidInput(format!(
            "expected a YouTube video URL: {url}"
        )));
    };

    Ok(video_id)
}

/// Extract a playlist id from a supported YouTube URL.
pub fn extract_playlist_id(url: &str) -> Result<String> {
    let parsed = Url::parse(url)?;
    let Some(host) = parsed.host_str() else {
        return Err(AppError::InvalidInput(format!(
            "unsupported YouTube URL: {url}"
        )));
    };

    let host = host.to_ascii_lowercase();
    let playlist_id = match host.as_str() {
        "youtube.com" | "www.youtube.com" | "m.youtube.com" | "music.youtube.com" => {
            let path = parsed.path().trim_end_matches('/');
            if path == "/watch" || path == "/playlist" {
                parsed
                    .query_pairs()
                    .find(|(key, _)| key == "list")
                    .map(|(_, value)| value.to_string())
            } else {
                None
            }
        }
        "youtu.be" | "www.youtu.be" => parsed
            .query_pairs()
            .find(|(key, _)| key == "list")
            .map(|(_, value)| value.to_string()),
        _ => None,
    };

    let Some(playlist_id) = playlist_id.filter(|value| !value.trim().is_empty()) else {
        return Err(AppError::InvalidInput(
            "YouTube dig requires playlist context. Pass a YouTube watch URL with both `v=` and `list=` parameters.".to_string(),
        ));
    };

    Ok(playlist_id)
}

/// Build a `videos.list` request URL.
pub fn videos_url(api_key: &str, video_ids: &[String]) -> Result<String> {
    let mut url = Url::parse("https://www.googleapis.com/youtube/v3/videos")?;
    url.query_pairs_mut()
        .append_pair("part", "snippet")
        .append_pair("id", &video_ids.join(","))
        .append_pair("key", api_key);
    Ok(url.to_string())
}

/// Build a playlist contents URL.
pub fn playlist_items_url(
    api_key: &str,
    playlist_id: &str,
    page_token: Option<&str>,
    max_results: usize,
) -> Result<String> {
    let mut url = Url::parse("https://www.googleapis.com/youtube/v3/playlistItems")?;
    url.query_pairs_mut()
        .append_pair("part", "snippet,contentDetails")
        .append_pair("playlistId", playlist_id)
        .append_pair("maxResults", &max_results.min(50).to_string())
        .append_pair("key", api_key);
    if let Some(page_token) = page_token {
        url.query_pairs_mut().append_pair("pageToken", page_token);
    }
    Ok(url.to_string())
}

/// Build a `playlists.list` request URL.
pub fn playlists_url(api_key: &str, playlist_ids: &[String]) -> Result<String> {
    let mut url = Url::parse("https://www.googleapis.com/youtube/v3/playlists")?;
    url.query_pairs_mut()
        .append_pair("part", "snippet")
        .append_pair("id", &playlist_ids.join(","))
        .append_pair("maxResults", &playlist_ids.len().min(50).to_string())
        .append_pair("key", api_key);
    Ok(url.to_string())
}

/// Parse a canonical seed video from a `videos.list` response body.
pub fn parse_seed(json: &str) -> Result<SeedAlbum> {
    let response: VideoListResponse = serde_json::from_str(json)?;
    let Some(video) = response.items.into_iter().next() else {
        return Err(AppError::NoPublicData);
    };
    let Some(snippet) = video.snippet else {
        return Err(AppError::Parse(
            "missing snippet in YouTube video response".to_string(),
        ));
    };

    let url = normalize_url(&format!("https://www.youtube.com/watch?v={}", video.id))?;
    let artist_url = snippet
        .channel_id
        .as_deref()
        .map(|channel_id| format!("https://www.youtube.com/channel/{channel_id}"));

    Ok(SeedAlbum {
        platform: Platform::Youtube,
        kind: ItemKind::Track,
        title: snippet.title,
        artist: snippet
            .channel_title
            .unwrap_or_else(|| "Unknown Channel".to_string()),
        url,
        artist_url,
        tags: snippet.tags,
        label: None,
        release_id: Some(video.id),
    })
}

/// Parse playlist titles from a `playlists.list` response body.
pub fn parse_playlist_titles(json: &str) -> Result<HashMap<String, String>> {
    let response: PlaylistListResponse = serde_json::from_str(json)?;
    Ok(response
        .items
        .into_iter()
        .filter_map(|playlist| Some((playlist.id, playlist.snippet?.title)))
        .collect())
}

/// Extract the next page token from a playlist-items response body.
pub fn parse_next_page_token(json: &str) -> Result<Option<String>> {
    let response: PlaylistItemListResponse = serde_json::from_str(json)?;
    Ok(response.next_page_token)
}

/// Parse playlist page entries from one `playlistItems.list` response body.
pub fn parse_playlist_page(json: &str) -> Result<Vec<PlaylistTrackEntry>> {
    let response: PlaylistItemListResponse = serde_json::from_str(json)?;
    Ok(response
        .items
        .into_iter()
        .filter_map(|item| {
            let snippet = item.snippet?;
            let position = snippet.position?;
            let video_id = item
                .content_details
                .as_ref()
                .and_then(|details| details.video_id.clone())
                .or_else(|| {
                    snippet
                        .resource_id
                        .as_ref()
                        .and_then(|id| id.video_id.clone())
                })?;
            Some(PlaylistTrackEntry {
                video_id,
                position,
                title: snippet.title.unwrap_or_else(|| "Unknown Video".to_string()),
                artist: snippet
                    .channel_title
                    .unwrap_or_else(|| "Unknown Channel".to_string()),
            })
        })
        .collect())
}

/// Parse a playlist into candidate evidence with per-track distance from the seed.
pub fn parse_playlist_source(
    json: &str,
    playlist_id: &str,
    playlist_title: Option<&str>,
    seed_video_id: &str,
) -> Result<Option<PlaylistSource>> {
    let entries = parse_playlist_page(json)?;
    build_playlist_source(entries, playlist_id, playlist_title, seed_video_id)
}

/// Build a playlist source from accumulated playlist entries.
pub fn build_playlist_source(
    entries: Vec<PlaylistTrackEntry>,
    playlist_id: &str,
    playlist_title: Option<&str>,
    seed_video_id: &str,
) -> Result<Option<PlaylistSource>> {
    let mut items = Vec::new();
    let mut seed_positions = Vec::new();

    for entry in entries {
        let video_id = entry.video_id;
        let position = entry.position;
        if video_id == seed_video_id {
            seed_positions.push(position);
        }

        items.push((video_id, position, entry.title, entry.artist));
    }

    if seed_positions.is_empty() {
        return Ok(None);
    }

    let mut deduped: HashMap<String, OwnedAlbum> = HashMap::new();
    for (video_id, position, title, artist) in items {
        if video_id == seed_video_id {
            continue;
        }

        let distance = seed_positions
            .iter()
            .map(|seed_position| seed_position.abs_diff(position))
            .min()
            .unwrap_or(usize::MAX);
        if distance == 0 || distance == usize::MAX {
            continue;
        }

        let url = normalize_url(&format!("https://www.youtube.com/watch?v={video_id}"))?;
        deduped
            .entry(url.clone())
            .and_modify(|existing| {
                let current = existing
                    .tags
                    .iter()
                    .find_map(|tag| tag.strip_prefix("distance:"))
                    .and_then(|value| value.parse::<usize>().ok())
                    .unwrap_or(distance);
                if distance < current {
                    existing.tags = vec![format!("distance:{distance}")];
                }
            })
            .or_insert_with(|| OwnedAlbum {
                platform: Platform::Youtube,
                kind: ItemKind::Track,
                title,
                artist,
                url,
                tags: vec![format!("distance:{distance}")],
                label: None,
            });
    }

    if deduped.is_empty() {
        return Ok(None);
    }

    let title = playlist_title
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| playlist_id.to_string());

    let mut tracks: Vec<_> = deduped.into_values().collect();
    tracks.sort_by(|a, b| {
        let a_distance = a
            .tags
            .iter()
            .find_map(|tag| tag.strip_prefix("distance:"))
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(usize::MAX);
        let b_distance = b
            .tags
            .iter()
            .find_map(|tag| tag.strip_prefix("distance:"))
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(usize::MAX);
        a_distance
            .cmp(&b_distance)
            .then_with(|| a.artist.cmp(&b.artist))
            .then_with(|| a.title.cmp(&b.title))
    });

    Ok(Some(PlaylistSource {
        id: playlist_id.to_string(),
        title,
        url: format!("https://www.youtube.com/playlist?list={playlist_id}"),
        tracks,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_watch_url() {
        let actual = normalize_url("https://music.youtube.com/watch?v=abc123&si=xyz").unwrap();
        assert_eq!(actual, "https://www.youtube.com/watch?v=abc123");
    }

    #[test]
    fn normalizes_short_url() {
        let actual = normalize_url("https://youtu.be/abc123?t=30").unwrap();
        assert_eq!(actual, "https://www.youtube.com/watch?v=abc123");
    }

    #[test]
    fn rejects_channel_urls() {
        let err = normalize_url("https://www.youtube.com/@artist").unwrap_err();
        assert!(err.to_string().contains("expected a YouTube video URL"));
    }

    #[test]
    fn extracts_playlist_id_from_watch_url() {
        let actual =
            extract_playlist_id("https://www.youtube.com/watch?v=seed123&list=PL123&index=4")
                .unwrap();
        assert_eq!(actual, "PL123");
    }

    #[test]
    fn rejects_watch_url_without_playlist_context() {
        let err = extract_playlist_id("https://www.youtube.com/watch?v=seed123").unwrap_err();
        assert!(err.to_string().contains("requires playlist context"));
    }

    #[test]
    fn parses_seed_video() {
        let json = include_str!("../tests/fixtures/youtube_video_seed.json");
        let seed = parse_seed(json).unwrap();

        assert_eq!(seed.platform, Platform::Youtube);
        assert_eq!(seed.kind, ItemKind::Track);
        assert_eq!(seed.title, "Seed Song");
        assert_eq!(seed.artist, "Seed Artist");
        assert_eq!(seed.url, "https://www.youtube.com/watch?v=seed123");
        assert_eq!(
            seed.artist_url.as_deref(),
            Some("https://www.youtube.com/channel/channel123")
        );
        assert_eq!(seed.release_id.as_deref(), Some("seed123"));
    }

    #[test]
    fn parses_playlist_titles() {
        let json = include_str!("../tests/fixtures/youtube_playlists.json");
        let titles = parse_playlist_titles(json).unwrap();
        assert_eq!(
            titles.get("PL_ONE").map(String::as_str),
            Some("Late Night Finds")
        );
    }

    #[test]
    fn parses_playlist_source_with_distance_tags() {
        let json = include_str!("../tests/fixtures/youtube_playlist_items_pl_one.json");
        let source = parse_playlist_source(json, "PL_ONE", Some("Late Night Finds"), "seed123")
            .unwrap()
            .unwrap();

        assert_eq!(source.title, "Late Night Finds");
        assert_eq!(source.tracks.len(), 3);
        assert_eq!(source.tracks[0].title, "Related One");
        assert!(source.tracks[0].tags.iter().any(|tag| tag == "distance:1"));
    }

    #[test]
    fn parses_playlist_page_entries() {
        let json = include_str!("../tests/fixtures/youtube_playlist_items_pl_one.json");
        let entries = parse_playlist_page(json).unwrap();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].video_id, "rel1");
    }
}
