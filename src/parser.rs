use std::collections::{HashMap, HashSet};

use regex::Regex;
use scraper::{Html, Selector};
use url::Url;

use crate::error::{AppError, Result};
use crate::model::{Collector, ItemKind, OwnedAlbum, Platform, SeedAlbum};

pub fn normalize_url(url: &str) -> Result<String> {
    let mut parsed = Url::parse(url)?;
    parsed.set_fragment(None);
    parsed.set_query(None);

    if parsed.path() != "/" {
        let trimmed = parsed.path().trim_end_matches('/').to_string();
        parsed.set_path(&trimmed);
    }

    Ok(parsed.to_string())
}

pub fn resolve_seed(url: &str, html: &str) -> Result<SeedAlbum> {
    let document = Html::parse_document(html);
    let canonical_url = meta_content(&document, r#"meta[property="og:url"]"#)
        .or_else(|| Some(url.to_string()))
        .map(|value| normalize_url(&value))
        .transpose()?
        .ok_or_else(|| AppError::Parse("unable to determine canonical URL".to_string()))?;

    let og_title = meta_content(&document, r#"meta[property="og:title"]"#)
        .or_else(|| title_text(&document))
        .unwrap_or_else(|| "Unknown Album".to_string());

    let (title, artist) = split_album_artist(&og_title);
    let artist_name = artist.unwrap_or_else(|| {
        infer_artist_from_html(html).unwrap_or_else(|| "Unknown Artist".to_string())
    });
    let tags = collect_tag_text(&document);

    Ok(SeedAlbum {
        platform: Platform::Bandcamp,
        kind: ItemKind::Album,
        title,
        artist: artist_name,
        url: canonical_url,
        artist_url: None,
        tags,
        label: None,
        release_id: infer_release_id(html),
    })
}

pub fn parse_collectors(html: &str) -> Vec<Collector> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("a[href]").expect("valid selector");
    let mut seen = HashSet::new();
    let mut collectors = Vec::new();

    for anchor in document.select(&selector) {
        let Some(href) = anchor.value().attr("href") else {
            continue;
        };
        let Some(url) = normalize_collector_url(href) else {
            continue;
        };
        if !seen.insert(url.clone()) {
            continue;
        }

        let handle = url
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("unknown")
            .to_string();
        let text = anchor
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();

        collectors.push(Collector {
            handle,
            url,
            display_name: if text.is_empty() { None } else { Some(text) },
            visible: true,
        });
    }

    collectors
}

pub fn parse_owned_albums(html: &str) -> Vec<OwnedAlbum> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("a[href]").expect("valid selector");
    let mut albums = HashMap::<String, OwnedAlbum>::new();

    for anchor in document.select(&selector) {
        let Some(href) = anchor.value().attr("href") else {
            continue;
        };
        let Some(url) = normalize_album_url(href) else {
            continue;
        };

        let text = anchor.text().collect::<Vec<_>>().join(" ");
        let trimmed = collapse_ws(&text);
        let (title, artist) = split_album_artist(&trimmed);

        albums.entry(url.clone()).or_insert_with(|| OwnedAlbum {
            platform: Platform::Bandcamp,
            kind: ItemKind::Album,
            title: if title.is_empty() {
                "Unknown Album".to_string()
            } else {
                title
            },
            artist: artist.unwrap_or_else(|| "Unknown Artist".to_string()),
            url,
            tags: Vec::new(),
            label: None,
        });
    }

    albums.into_values().collect()
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

fn split_album_artist(raw: &str) -> (String, Option<String>) {
    let cleaned = collapse_ws(raw);
    if let Some((title, artist)) = cleaned.split_once(", by ") {
        return (title.trim().to_string(), Some(artist.trim().to_string()));
    }
    if let Some((title, artist)) = cleaned.split_once(" | ") {
        return (title.trim().to_string(), Some(artist.trim().to_string()));
    }
    (cleaned, None)
}

fn collect_tag_text(document: &Html) -> Vec<String> {
    let selector = Selector::parse(r#"a[href*="/tag/"]"#).expect("valid selector");
    let mut seen = HashSet::new();
    let mut tags = Vec::new();
    for tag in document.select(&selector) {
        let text = collapse_ws(&tag.text().collect::<Vec<_>>().join(" "));
        if !text.is_empty() && seen.insert(text.clone()) {
            tags.push(text);
        }
    }
    tags
}

fn infer_artist_from_html(html: &str) -> Option<String> {
    let patterns = [
        Regex::new(r#""artist"\s*:\s*"([^"]+)""#).ok()?,
        Regex::new(r#""byArtist"\s*:\s*"([^"]+)""#).ok()?,
    ];

    for pattern in patterns {
        if let Some(capture) = pattern.captures(html) {
            return Some(capture.get(1)?.as_str().to_string());
        }
    }

    None
}

fn infer_release_id(html: &str) -> Option<String> {
    let pattern = Regex::new(r#""id"\s*:\s*([0-9]+)"#).ok()?;
    pattern
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|id| id.as_str().to_string())
}

fn normalize_collector_url(href: &str) -> Option<String> {
    let url = if href.starts_with("http://") || href.starts_with("https://") {
        Url::parse(href).ok()?
    } else {
        Url::parse(&format!(
            "https://bandcamp.com{}",
            ensure_leading_slash(href)
        ))
        .ok()?
    };

    let host = url.host_str()?;
    let path = url.path().trim_end_matches('/');
    let first_segment = path
        .trim_start_matches('/')
        .split('/')
        .next()
        .unwrap_or_default();

    if host == "bandcamp.com"
        && !first_segment.is_empty()
        && !matches!(
            first_segment,
            "album" | "track" | "music" | "discover" | "tag" | "about" | "help" | "search"
        )
    {
        return Some(format!("https://bandcamp.com/{first_segment}"));
    }

    None
}

fn normalize_album_url(href: &str) -> Option<String> {
    let candidate = if href.starts_with("http://") || href.starts_with("https://") {
        Url::parse(href).ok()?
    } else {
        return None;
    };

    if candidate.path().contains("/album/") {
        let mut normalized = candidate;
        normalized.set_query(None);
        normalized.set_fragment(None);
        return Some(normalized.to_string().trim_end_matches('/').to_string());
    }

    None
}

fn ensure_leading_slash(path: &str) -> String {
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}

fn collapse_ws(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_album_url() {
        let actual = normalize_url("https://artist.bandcamp.com/album/test?x=1#frag").unwrap();
        assert_eq!(actual, "https://artist.bandcamp.com/album/test");
    }

    #[test]
    fn parses_collectors_from_bandcamp_links() {
        let html = r#"
            <html><body>
                <a href="https://bandcamp.com/fan_a">Fan A</a>
                <a href="/fan_b">Fan B</a>
                <a href="https://bandcamp.com/discover">Discover</a>
            </body></html>
        "#;
        let collectors = parse_collectors(html);
        assert_eq!(collectors.len(), 2);
        assert_eq!(collectors[0].url, "https://bandcamp.com/fan_a");
    }

    #[test]
    fn parses_owned_album_links() {
        let html = r#"
            <html><body>
                <a href="https://artist.bandcamp.com/album/record-a">Record A, by Artist A</a>
                <a href="https://artist.bandcamp.com/track/song-a">Song A</a>
            </body></html>
        "#;
        let albums = parse_owned_albums(html);
        assert_eq!(albums.len(), 1);
        assert_eq!(albums[0].title, "Record A");
        assert_eq!(albums[0].artist, "Artist A");
        assert_eq!(albums[0].platform, Platform::Bandcamp);
        assert_eq!(albums[0].kind, ItemKind::Album);
    }

    #[test]
    fn resolves_seed_from_og_title() {
        let html = r#"
            <html><head>
                <meta property="og:url" content="https://artist.bandcamp.com/album/seed">
                <meta property="og:title" content="Seed Record, by Seed Artist">
            </head></html>
        "#;
        let seed = resolve_seed("https://artist.bandcamp.com/album/seed", html).unwrap();
        assert_eq!(seed.title, "Seed Record");
        assert_eq!(seed.artist, "Seed Artist");
        assert_eq!(seed.platform, Platform::Bandcamp);
        assert_eq!(seed.kind, ItemKind::Album);
    }
}
