# HACKING

## Overview

`wax` is a Rust CLI with provider-based dispatch for Bandcamp, SoundCloud, and YouTube.

Current provider split:

- Bandcamp:
  - `resolve` supported
  - `collectors` supported
  - `library` supported
  - `dig` supported
- SoundCloud:
  - `resolve` supported
  - `dig` supported for track URLs as a prototype
  - `collectors` unsupported
  - `library` unsupported
  - playlist `dig` unsupported
- YouTube:
  - `resolve` supported for video URLs
  - `dig` supported for watch URLs with playlist context using public playlist overlap
  - `collectors` unsupported
  - `library` unsupported

Core implementation lives in:

- `src/app.rs`
- `src/provider.rs`
- `src/parser.rs`
- `src/soundcloud.rs`
- `src/youtube.rs`
- `src/score.rs`
- `src/output.rs`

Execution path for a typical command:

1. `src/main.rs` parses the CLI and calls `wax::app::run`.
2. `src/app.rs` initializes logging, loads settings, chooses an output format, and dispatches the subcommand.
3. `src/provider.rs` detects the upstream platform and runs the command-specific workflow.
4. `src/fetch.rs` fetches remote pages or fixture files and uses `src/cache.rs` for response caching.
5. `src/parser.rs`, `src/soundcloud.rs`, or `src/youtube.rs` normalize URLs and extract provider-specific data.
6. `src/score.rs` aggregates overlap evidence into ranked recommendations.
7. `src/output.rs` renders table, JSON, or CSV output.

## Module Guide

- `src/app.rs`: top-level application wiring from parsed CLI to provider commands and output rendering
- `src/cache.rs`: filesystem-backed response cache keyed by URL hash
- `src/cli.rs`: Clap definitions for commands, flags, and help text
- `src/config.rs`: config-file loading and precedence with CLI flags
- `src/error.rs`: shared error type and process exit-code mapping
- `src/fetch.rs`: HTTP client setup, cache lookups, rate limiting, and fixture loading via `file://`
- `src/model.rs`: shared structs used across provider code, ranking, and output
- `src/output.rs`: human-readable table rendering plus JSON/CSV serializers
- `src/parser.rs`: Bandcamp HTML normalization and parsing
- `src/provider.rs`: provider detection and command orchestration
- `src/score.rs`: candidate filtering, aggregation, and ranking
- `src/soundcloud.rs`: SoundCloud URL handling, API helpers, and likes-feed parsing
- `src/youtube.rs`: YouTube URL handling, API helpers, and playlist overlap parsing

If you are adding a new provider, the cleanest path is to keep provider-specific normalization/parsing in a dedicated module and extend `src/provider.rs` only with orchestration and dispatch.

## Development

Run the test suite:

```bash
cargo test
```

Run the CLI locally without installing:

```bash
cargo run -- dig https://artist.bandcamp.com/album/example-record
cargo run -- resolve https://soundcloud.com/chvrches/the-mother-we-share
cargo run -- --help
```

Build rustdoc locally:

```bash
cargo doc --no-deps
```

## Releasing

`wax` uses `cargo-release` for local release orchestration and GitHub Actions for the actual crates.io publish.

Install `cargo-release`:

```bash
cargo install cargo-release
```

The release config lives in `release.toml`.

Normal release flow:

1. Ensure the branch is up to date and CI is green.
2. Run a dry run:

```bash
cargo release patch
```

3. Execute the release locally:

```bash
cargo release patch --execute
```

4. Push the release commit and tag:

```bash
git push --follow-tags
```

That push triggers `.github/workflows/publish.yml`, which:

- checks formatting
- runs clippy
- runs tests
- runs `cargo publish --dry-run`
- publishes to crates.io

Before the first release, configure the GitHub repo secret:

- `CRATES_IO_TOKEN`

Recommended release policy:

- publish intentional releases only
- do not publish every merge to `main`
- use patch releases for fixes, minor releases for additive features, and major releases for breaking changes

## Configuration

Pass a TOML file with `--config`:

```bash
wax dig https://artist.bandcamp.com/album/example-record --config ./wax.toml
```

Example config:

```toml
cache_dir = "/tmp/wax-cache"
request_delay_ms = 1000
concurrency = 4
timeout_ms = 10000
user_agent = "wax/0.1"
youtube_api_key = "your-api-key"
max_collectors = 75
max_depth = 1
```

Note: the current CLI reads the config file for shared request settings. Command-specific defaults such as `max_collectors` and `max_depth` are present in the config schema but are not yet wired into command execution.

YouTube support requires a YouTube Data API key. The runtime looks for it in this order:

1. `--youtube-api-key`
2. `youtube_api_key` in the config file
3. `YOUTUBE_API_KEY` in the environment

## SoundCloud Notes

SoundCloud resolve currently uses the public `api-v2 /resolve` endpoint after deriving a client id from the page or falling back to a known public client id.

SoundCloud `dig` currently works like this:

1. Resolve the seed track
2. Fetch public likers for the seed
3. Fetch each liker's public likes feed
4. Find the seed-like event in that feed
5. Use the two previous and two next likes as candidate evidence
6. Rank repeated co-likes

The live SoundCloud `dig` path is still expensive and may need more work on:

- crawl limits
- fallback behavior
- sparse public data
- pagination strategy

When editing the SoundCloud flow, keep in mind that `src/provider.rs` owns the crawl orchestration while `src/soundcloud.rs` owns URL construction and response parsing. Keeping that split clean makes it easier to test and document.

## YouTube Notes

YouTube resolve uses the YouTube Data API `videos.list` endpoint.

YouTube `dig` currently works like this:

1. Resolve the seed video
2. Extract the `list=` playlist context from the input URL
3. Resolve playlist metadata
4. Fetch playlist contents until the seed page is covered
5. Score nearby co-occurrence within that playlist, with a small bonus for closer playlist neighbors

Current limitations:

- requires a YouTube Data API key
- requires a watch URL where the seed video is inside the referenced playlist
- does not support `collectors` or `library`

When editing the YouTube flow, keep the same split as SoundCloud: `src/provider.rs` owns crawl orchestration and `src/youtube.rs` owns URL construction and response parsing.

## Testing Notes

Fixture coverage currently includes:

- Bandcamp overlap flow in `tests/dig_flow.rs`
- SoundCloud likes-based flow in `tests/soundcloud_likes_dig_flow.rs`
- YouTube playlist-overlap flow in `tests/youtube_playlist_dig_flow.rs`

Relevant fixtures:

- `tests/fixtures/fan_a.html`
- `tests/fixtures/fan_b.html`
- `tests/fixtures/seed.html`
- `tests/fixtures/soundcloud_likers.json`
- `tests/fixtures/soundcloud_user_likes_a.json`
- `tests/fixtures/soundcloud_user_likes_b.json`
- `tests/fixtures/youtube_video_seed.json`
- `tests/fixtures/youtube_playlists.json`
- `tests/fixtures/youtube_seed_playlist_hits.json`
- `tests/fixtures/youtube_playlist_items_pl_one.json`
- `tests/fixtures/youtube_playlist_items_pl_two.json`
