//! SoundCloud-specific URL handling, API helpers, and likes parsing.
//!
//! Unlike the Bandcamp path, most useful discovery data comes from SoundCloud's
//! public API responses rather than from HTML alone. This module keeps the
//! provider-specific pieces contained so the higher-level command flow in
//! [`crate::provider`] can remain provider-neutral.

use regex::Regex;
use scraper::{Html, Selector};
use serde::Deserialize;
use url::Url;

use crate::error::{AppError, Result};
use crate::model::{ItemKind, Platform, SeedAlbum};

const FALLBACK_CLIENT_ID: &str = "WU4bVxk5Df0g5JC8ULzW77Ry7OM10Lyj";

/// Normalize a SoundCloud track or playlist URL.
pub fn normalize_url(url: &str) -> Result<String> {
    let mut parsed = Url::parse(url)?;
    parsed.set_fragment(None);
    parsed.set_query(None);

    let Some(host) = parsed.host_str() else {
        return Err(AppError::InvalidInput(format!(
            "unsupported SoundCloud URL: {url}"
        )));
    };

    let host = host.to_ascii_lowercase();
    if host != "soundcloud.com" && host != "www.soundcloud.com" && host != "m.soundcloud.com" {
        if host == "on.soundcloud.com" {
            return Ok(parsed.to_string().trim_end_matches('/').to_string());
        }

        return Err(AppError::InvalidInput(format!(
            "unsupported SoundCloud URL: {url}"
        )));
    }

    parsed
        .set_host(Some("soundcloud.com"))
        .map_err(|_| AppError::InvalidInput(format!("unsupported SoundCloud URL: {url}")))?;

    let trimmed = parsed.path().trim_end_matches('/');
    let segments: Vec<_> = trimmed
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.len() < 2 {
        return Err(AppError::InvalidInput(format!(
            "expected a SoundCloud track or playlist URL: {url}"
        )));
    }

    parsed.set_path(&format!("/{}", segments.join("/")));
    Ok(parsed.to_string())
}

/// Resolve a SoundCloud page directly from HTML metadata.
pub fn resolve_seed(url: &str, html: &str) -> Result<SeedAlbum> {
    let document = Html::parse_document(html);
    let canonical_url = meta_content(&document, r#"meta[property="og:url"]"#)
        .or_else(|| Some(url.to_string()))
        .map(|value| normalize_url(&value))
        .transpose()?
        .ok_or_else(|| AppError::Parse("unable to determine canonical URL".to_string()))?;

    let kind = infer_kind(&canonical_url);
    let title = meta_content(&document, r#"meta[property="og:title"]"#)
        .or_else(|| json_field(html, "title"))
        .or_else(|| title_text(&document))
        .unwrap_or_else(|| "Unknown SoundCloud Item".to_string());
    let artist = json_field(html, "username")
        .or_else(|| meta_content(&document, r#"meta[name="twitter:audio:artist_name"]"#))
        .or_else(|| meta_content(&document, r#"meta[property="soundcloud:creator"]"#))
        .or_else(|| extract_artist_from_title(&title))
        .unwrap_or_else(|| "Unknown Artist".to_string());
    let genre = meta_content(&document, r#"meta[property="music:genre"]"#)
        .or_else(|| json_field(html, "genre"));
    let mut tags = Vec::new();
    if let Some(genre) = genre.filter(|value| !value.trim().is_empty()) {
        tags.push(genre);
    }

    Ok(SeedAlbum {
        platform: Platform::Soundcloud,
        kind,
        title: clean_title(&title),
        artist,
        url: canonical_url.clone(),
        artist_url: infer_artist_url(&canonical_url),
        tags,
        label: None,
        release_id: infer_track_id(&document, html).or_else(|| json_numeric_field(html, "id")),
    })
}

/// Public liker plus the nearby liked tracks used as recommendation evidence.
#[derive(Debug, Clone)]
pub struct LikeSource {
    pub id: String,
    pub title: String,
    pub url: String,
    pub tracks: Vec<crate::model::OwnedAlbum>,
}

/// One page of a user's likes feed plus an optional extracted evidence window.
pub struct UserLikesPage {
    pub source: Option<LikeSource>,
    pub next_href: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LikersResponse {
    #[serde(default)]
    collection: Vec<ApiLiker>,
}

#[derive(Debug, Deserialize)]
struct ApiLiker {
    id: u64,
    permalink_url: Option<String>,
    username: String,
}

#[derive(Debug, Deserialize)]
struct ApiTrack {
    id: u64,
    title: String,
    permalink_url: String,
    #[serde(default)]
    kind: String,
    genre: Option<String>,
    label_name: Option<String>,
    user: Option<ApiUser>,
}

#[derive(Debug, Deserialize)]
struct ApiUser {
    username: String,
    permalink_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UserLikesResponse {
    #[serde(default)]
    collection: Vec<ApiLike>,
    next_href: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiLike {
    created_at: String,
    track: Option<ApiTrack>,
}

/// Extract the public client id used by SoundCloud's web application.
pub fn extract_client_id(html: &str) -> Result<String> {
    let pattern =
        Regex::new(r#""hydratable":"apiClient","data":\{"id":"([^"]+)""#).expect("valid regex");
    let client_id = pattern
        .captures(html)
        .and_then(|captures| captures.get(1).map(|value| value.as_str().to_string()))
        .unwrap_or_else(|| FALLBACK_CLIENT_ID.to_string());

    Ok(client_id)
}

/// Build the public likers endpoint for a track.
pub fn likers_url(client_id: &str, track_id: &str, limit: usize) -> Result<String> {
    let mut url = Url::parse(&format!(
        "https://api-v2.soundcloud.com/tracks/{track_id}/likers"
    ))?;
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("limit", &limit.to_string());
    Ok(url.to_string())
}

/// Build the likes-feed endpoint for a user.
pub fn user_likes_url(client_id: &str, user_id: &str, limit: usize) -> Result<String> {
    let mut url = Url::parse(&format!(
        "https://api-v2.soundcloud.com/users/{user_id}/likes"
    ))?;
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("limit", &limit.to_string());
    Ok(url.to_string())
}

/// Build the SoundCloud resolve endpoint for a canonical URL.
pub fn resolve_api_url(client_id: &str, soundcloud_url: &str) -> Result<String> {
    let mut url = Url::parse("https://api-v2.soundcloud.com/resolve")?;
    url.query_pairs_mut()
        .append_pair("url", soundcloud_url)
        .append_pair("client_id", client_id);
    Ok(url.to_string())
}

/// Ensure a SoundCloud API URL includes the given client id.
pub fn with_client_id(url: &str, client_id: &str) -> Result<String> {
    let mut parsed = Url::parse(url)?;
    let has_client_id = parsed.query_pairs().any(|(key, _)| key == "client_id");
    if !has_client_id {
        parsed.query_pairs_mut().append_pair("client_id", client_id);
    }
    Ok(parsed.to_string())
}

/// Resolve a canonical SoundCloud seed from the public API response body.
pub fn resolve_api_seed(json: &str) -> Result<SeedAlbum> {
    let track: ApiTrack = serde_json::from_str(json)?;
    let kind = match track.kind.as_str() {
        "playlist" => ItemKind::Playlist,
        _ => ItemKind::Track,
    };
    let artist = track
        .user
        .as_ref()
        .map(|user| user.username.clone())
        .unwrap_or_else(|| "Unknown Artist".to_string());
    let artist_url = track.user.and_then(|user| user.permalink_url);

    Ok(SeedAlbum {
        platform: Platform::Soundcloud,
        kind,
        title: track.title,
        artist,
        url: track.permalink_url,
        artist_url,
        tags: track.genre.into_iter().collect(),
        label: track.label_name,
        release_id: Some(track.id.to_string()),
    })
}

/// Parse a list of public likers for a track.
pub fn parse_likers(json: &str) -> Result<Vec<LikeSource>> {
    let response: LikersResponse = serde_json::from_str(json)?;
    Ok(response
        .collection
        .into_iter()
        .map(|user| LikeSource {
            id: user.id.to_string(),
            title: user.username,
            url: user.permalink_url.unwrap_or_default(),
            tracks: Vec::new(),
        })
        .collect())
}

/// Parse one page of a user's likes feed and extract nearby recommendation evidence.
pub fn parse_user_likes_page(
    json: &str,
    user: &LikeSource,
    seed_track_id: &str,
    max_neighbors: usize,
) -> Result<UserLikesPage> {
    let response: UserLikesResponse = serde_json::from_str(json)?;
    let seed_track_id = seed_track_id
        .parse::<u64>()
        .map_err(|_| AppError::Parse("invalid SoundCloud track id".to_string()))?;

    let seed_index = response.collection.iter().position(|entry| {
        entry
            .track
            .as_ref()
            .map(|track| track.id == seed_track_id)
            .unwrap_or(false)
    });

    let Some(seed_index) = seed_index else {
        return Ok(UserLikesPage {
            source: None,
            next_href: response.next_href,
        });
    };

    let seed_timestamp = response.collection[seed_index].created_at.clone();
    let mut deduped = std::collections::HashMap::new();
    for (index, entry) in response.collection.into_iter().enumerate() {
        let distance = index.abs_diff(seed_index);
        if distance == 0 || distance > max_neighbors {
            continue;
        }
        let Some(track) = entry.track else {
            continue;
        };
        if track.id == seed_track_id || track.kind != "track" {
            continue;
        }

        deduped
            .entry(track.permalink_url.clone())
            .or_insert_with(|| {
                let mut tags: Vec<String> = track.genre.into_iter().collect();
                if !entry.created_at.is_empty() {
                    tags.push(format!("liked_at:{}", entry.created_at));
                }
                tags.push(format!("seed_liked_at:{seed_timestamp}"));

                crate::model::OwnedAlbum {
                    platform: Platform::Soundcloud,
                    kind: ItemKind::Track,
                    title: track.title,
                    artist: track
                        .user
                        .map(|user| user.username)
                        .unwrap_or_else(|| "Unknown Artist".to_string()),
                    url: track.permalink_url,
                    tags,
                    label: track.label_name,
                }
            });
    }

    if deduped.is_empty() {
        return Ok(UserLikesPage {
            source: None,
            next_href: response.next_href,
        });
    }

    Ok(UserLikesPage {
        source: Some(LikeSource {
            id: user.id.clone(),
            title: user.title.clone(),
            url: user.url.clone(),
            tracks: deduped.into_values().collect(),
        }),
        next_href: response.next_href,
    })
}

fn infer_kind(url: &str) -> ItemKind {
    if url.contains("/sets/") {
        ItemKind::Playlist
    } else {
        ItemKind::Track
    }
}

fn infer_artist_url(canonical_url: &str) -> Option<String> {
    let parsed = Url::parse(canonical_url).ok()?;
    let segments: Vec<_> = parsed
        .path_segments()?
        .filter(|segment| !segment.is_empty())
        .collect();
    let first = segments.first()?;
    Some(format!("https://soundcloud.com/{first}"))
}

fn meta_content(document: &Html, selector: &str) -> Option<String> {
    let selector = Selector::parse(selector).ok()?;
    document
        .select(&selector)
        .next()
        .and_then(|node| node.value().attr("content"))
        .map(|value| value.trim().to_string())
}

fn title_text(document: &Html) -> Option<String> {
    let selector = Selector::parse("title").ok()?;
    document
        .select(&selector)
        .next()
        .map(|node| collapse_ws(&node.text().collect::<Vec<_>>().join(" ")))
}

fn json_field(html: &str, field: &str) -> Option<String> {
    let pattern = Regex::new(&format!(r#""{}"\s*:\s*"([^"]+)""#, regex::escape(field))).ok()?;
    let captures = pattern.captures(html)?;
    let value = captures.get(1)?.as_str();
    Some(html_escape(value))
}

fn json_numeric_field(html: &str, field: &str) -> Option<String> {
    let pattern = Regex::new(&format!(r#""{}"\s*:\s*([0-9]+)"#, regex::escape(field))).ok()?;
    let captures = pattern.captures(html)?;
    Some(captures.get(1)?.as_str().to_string())
}

fn infer_track_id(document: &Html, html: &str) -> Option<String> {
    let meta_keys = [
        r#"meta[property="twitter:app:url:iphone"]"#,
        r#"meta[property="twitter:app:url:ipad"]"#,
        r#"meta[property="twitter:app:url:googleplay"]"#,
        r#"meta[property="al:ios:url"]"#,
        r#"meta[property="al:android:url"]"#,
    ];

    for key in meta_keys {
        if let Some(value) = meta_content(document, key) {
            if let Some(id) = extract_sound_id(&value) {
                return Some(id);
            }
        }
    }

    let patterns = [
        Regex::new(r#"soundcloud://sounds:([0-9]+)"#).ok()?,
        Regex::new(r#""urn"\s*:\s*"soundcloud:tracks:([0-9]+)""#).ok()?,
        Regex::new(r#""station_urn"\s*:\s*"soundcloud:system-playlists:track-stations:([0-9]+)""#)
            .ok()?,
    ];

    for pattern in patterns {
        if let Some(captures) = pattern.captures(html) {
            return captures.get(1).map(|value| value.as_str().to_string());
        }
    }

    None
}

fn extract_sound_id(value: &str) -> Option<String> {
    let pattern = Regex::new(r#"sounds:([0-9]+)"#).ok()?;
    let captures = pattern.captures(value)?;
    captures.get(1).map(|value| value.as_str().to_string())
}

fn extract_artist_from_title(title: &str) -> Option<String> {
    let collapsed = collapse_ws(title);
    if let Some((artist, _)) = collapsed.split_once(" - ") {
        return Some(artist.trim().to_string());
    }
    None
}

fn clean_title(title: &str) -> String {
    let collapsed = collapse_ws(title);
    if let Some((artist, track)) = collapsed.split_once(" - ") {
        if !artist.trim().is_empty() && !track.trim().is_empty() {
            return track.trim().to_string();
        }
    }
    collapsed
}

fn html_escape(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
}

fn collapse_ws(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_soundcloud_track_url() {
        let actual =
            normalize_url("https://m.soundcloud.com/test-user/test-track/?si=abc#frag").unwrap();
        assert_eq!(actual, "https://soundcloud.com/test-user/test-track");
    }

    #[test]
    fn rejects_soundcloud_profile_url_for_seed_resolution() {
        let err = normalize_url("https://soundcloud.com/test-user").unwrap_err();
        assert!(err
            .to_string()
            .contains("expected a SoundCloud track or playlist URL"));
    }

    #[test]
    fn resolves_soundcloud_track_seed_from_meta_and_json() {
        let html = r#"
            <html>
                <head>
                    <meta property="og:url" content="https://soundcloud.com/test-user/test-track?si=123">
                    <meta property="og:title" content="Test User - Test Track">
                    <meta property="music:genre" content="ambient">
                    <script type="application/ld+json">
                        {"username":"Test User","title":"Test Track","id":12345}
                    </script>
                </head>
            </html>
        "#;

        let seed = resolve_seed("https://soundcloud.com/test-user/test-track", html).unwrap();
        assert_eq!(seed.platform, Platform::Soundcloud);
        assert_eq!(seed.kind, ItemKind::Track);
        assert_eq!(seed.title, "Test Track");
        assert_eq!(seed.artist, "Test User");
        assert_eq!(seed.url, "https://soundcloud.com/test-user/test-track");
        assert_eq!(
            seed.artist_url.as_deref(),
            Some("https://soundcloud.com/test-user")
        );
        assert_eq!(seed.tags, vec!["ambient"]);
        assert_eq!(seed.release_id.as_deref(), Some("12345"));
    }

    #[test]
    fn resolves_soundcloud_playlist_kind() {
        let html = r#"
            <html>
                <head>
                    <meta property="og:url" content="https://soundcloud.com/test-user/sets/test-set">
                    <meta property="og:title" content="Test User - Test Set">
                </head>
            </html>
        "#;

        let seed = resolve_seed("https://soundcloud.com/test-user/sets/test-set", html).unwrap();
        assert_eq!(seed.kind, ItemKind::Playlist);
    }

    #[test]
    fn extracts_client_id_from_hydration_blob() {
        let html = r#"
            <script>
                window.__sc_hydration = [{"hydratable":"apiClient","data":{"id":"abc123","isExpiring":false}}];
            </script>
        "#;

        assert_eq!(extract_client_id(html).unwrap(), "abc123");
    }

    #[test]
    fn parses_public_likers() {
        let json = include_str!("../tests/fixtures/soundcloud_likers.json");
        let likers = parse_likers(json).unwrap();

        assert_eq!(likers.len(), 2);
        assert_eq!(likers[0].id, "501");
        assert_eq!(likers[0].title, "listener-a");
    }

    #[test]
    fn parses_user_likes_near_seed_event() {
        let user = LikeSource {
            id: "501".to_string(),
            title: "listener-a".to_string(),
            url: "https://soundcloud.com/listener-a".to_string(),
            tracks: Vec::new(),
        };
        let json = include_str!("../tests/fixtures/soundcloud_user_likes_a.json");
        let source = parse_user_likes_page(json, &user, "100", 2)
            .unwrap()
            .source
            .unwrap();

        assert_eq!(source.tracks.len(), 3);
        assert_eq!(source.tracks[0].platform, Platform::Soundcloud);
    }
}
