# HACKING

## Overview

`wax` is a Rust CLI with provider-based dispatch for Bandcamp and SoundCloud.

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

Core implementation lives in:

- `src/app.rs`
- `src/provider.rs`
- `src/parser.rs`
- `src/soundcloud.rs`
- `src/score.rs`
- `src/output.rs`

## Development

Run the test suite:

```bash
cargo test
```

Run the CLI locally without installing:

```bash
cargo run -- dig https://artist.bandcamp.com/album/example-record
cargo run -- resolve https://soundcloud.com/chvrches/the-mother-we-share
```

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
max_collectors = 75
max_depth = 1
```

Note: the current CLI reads the config file for shared request settings. Command-specific defaults such as `max_collectors` and `max_depth` are present in the config schema but are not yet wired into command execution.

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

## Testing Notes

Fixture coverage currently includes:

- Bandcamp overlap flow in `tests/dig_flow.rs`
- SoundCloud likes-based flow in `tests/soundcloud_likes_dig_flow.rs`

Relevant fixtures:

- `tests/fixtures/fan_a.html`
- `tests/fixtures/fan_b.html`
- `tests/fixtures/seed.html`
- `tests/fixtures/soundcloud_likers.json`
- `tests/fixtures/soundcloud_user_likes_a.json`
- `tests/fixtures/soundcloud_user_likes_b.json`
