# wax

[![CI](https://github.com/jdhoffa/wax/actions/workflows/ci.yml/badge.svg)](https://github.com/jdhoffa/wax/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/wax-dig.svg)](https://crates.io/crates/wax-dig)
[![docs.rs](https://img.shields.io/docsrs/wax-dig)](https://docs.rs/wax-dig)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

`wax` is a Rust CLI for digging through public music discovery signals from Bandcamp, SoundCloud, and YouTube.

Give it a URL and it will detect the platform automatically:

- Bandcamp album URLs use collector overlap
- SoundCloud track URLs use public likers and nearby likes in each liker feed
- YouTube watch URLs with playlist context use public playlist overlap

`wax` keeps the same entrypoint across providers:

```bash
wax dig <url>
```

## Install

Build and install the binary into Cargo's bin directory:

```bash
cargo install --path .
```

To install from crates.io:

```bash
cargo install wax-dig
```

That usually installs `wax` to `~/.cargo/bin/wax`.

If `~/.cargo/bin` is not on your `PATH`, add:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

For a local build without installing:

```bash
cargo build --release
```

The binary will be at `target/release/wax`.

## Quick Start

Bandcamp:

```bash
wax dig https://artist.bandcamp.com/album/example-record
```

SoundCloud:

```bash
wax resolve https://soundcloud.com/chvrches/the-mother-we-share
wax dig https://soundcloud.com/chvrches/the-mother-we-share
```

YouTube:

```bash
export YOUTUBE_API_KEY=your-api-key
wax resolve https://www.youtube.com/watch?v=dQw4w9WgXcQ
wax dig 'https://music.youtube.com/watch?v=dQw4w9WgXcQ&list=PL123'
```

Inspect the available commands:

```bash
wax --help
wax dig --help
wax resolve --help
```

Typical flow:

1. Run `wax resolve <url>` to confirm the seed metadata and platform detection.
2. Run `wax dig <url>` to generate ranked recommendations.
3. Add output flags like `--json` or `--csv` when you want to script against the results.

## Commands

### `wax dig <url>`

Runs the recommendation flow for a supported URL.

What it does:

- Bandcamp: resolves the seed album, collects public collectors, reads each public library, and ranks repeated overlap
- SoundCloud: resolves the seed track, fetches public likers, inspects each liker feed around the seed like event, and ranks repeated nearby co-likes
- YouTube: resolves the seed video, loads the playlist from the URL, and ranks nearby co-occurrence within that playlist

Examples:

```bash
wax dig https://artist.bandcamp.com/album/example-record
wax dig https://artist.bandcamp.com/album/example-record --max-collectors 50 --limit 20
wax dig https://soundcloud.com/chvrches/the-mother-we-share --max-collectors 25 --limit 10
wax dig 'https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=PL123' --limit 10
wax dig https://soundcloud.com/chvrches/the-mother-we-share --exclude-artist --sort overlap
wax dig https://artist.bandcamp.com/album/example-record --json
```

Useful flags:

- `--max-collectors <n>`: maximum source nodes to sample during discovery
- `--max-depth <n>`: reserved crawl-depth flag, currently defaults to `1`
- `--limit <n>`: maximum recommendations to print
- `--sample top|random`: keep sources in discovery order or shuffle before truncating
- `--min-overlap <n>`: minimum overlap count required for a recommendation
- `--exclude-artist`: drop records by the same artist as the seed
- `--exclude-label`: drop records on the same label as the seed
- `--tag <tag>`: require matching tags on candidate records
- `--sort score|overlap`: sort recommendations by score or raw overlap

Provider notes:

- Bandcamp `dig` uses public collectors and their public libraries
- SoundCloud `dig` currently supports track URLs
- YouTube `dig` requires a YouTube Data API key and a watch URL with playlist context
- SoundCloud playlist URLs are not supported for `dig` yet
- YouTube recommendations are based on public playlist overlap rather than user likes
- `--tag` filtering is most useful on Bandcamp; SoundCloud tags are currently limited to what the public API exposes
- `--csv` writes only ranked result rows for `dig`

### `wax resolve <url>`

Resolves a supported URL and prints canonical seed metadata.

Examples:

```bash
wax resolve https://artist.bandcamp.com/album/example-record
wax resolve https://soundcloud.com/chvrches/the-mother-we-share
wax resolve https://www.youtube.com/watch?v=dQw4w9WgXcQ
```

### `wax collectors <album-url>`

Lists public collectors discovered for a Bandcamp seed album.

```bash
wax collectors https://artist.bandcamp.com/album/example-record
wax collectors https://artist.bandcamp.com/album/example-record --max-collectors 100
```

This command is Bandcamp-only.

### `wax library <fan-url>`

Prints albums from a public Bandcamp fan page.

```bash
wax library https://bandcamp.com/fanname
wax library https://bandcamp.com/fanname --limit 50
```

This command is Bandcamp-only.

### `wax cache stats`

Prints the cache directory, entry count, and cache size in bytes.

```bash
wax cache stats
```

### `wax cache clear`

Clears the local fetch cache.

```bash
wax cache clear
```

## Global Flags

These flags are available across commands:

- `--config <path>`: load settings from a TOML config file
- `--cache-dir <path>`: override the cache directory
- `--json`: print JSON output
- `--csv`: print CSV output where supported
- `--user-agent <value>`: override the HTTP user agent
- `--youtube-api-key <value>`: YouTube Data API key, alternatively set `YOUTUBE_API_KEY`
- `--rate-limit-ms <n>`: delay between requests in milliseconds
- `--concurrency <n>`: request concurrency setting
- `--timeout-ms <n>`: per-request timeout
- `-v`, `--verbose`: enable debug logging
- `--quiet`: only print errors

CLI precedence:

- command-line flags override config file values
- config file values override built-in defaults

## Output Modes

By default, `wax` prints readable table output.

Use `--json` for structured output:

```bash
wax dig https://artist.bandcamp.com/album/example-record --json
wax resolve https://soundcloud.com/chvrches/the-mother-we-share --json
```

Use `--csv` for CSV output:

```bash
wax dig https://artist.bandcamp.com/album/example-record --csv
wax collectors https://artist.bandcamp.com/album/example-record --csv
wax library https://bandcamp.com/fanname --csv
```

Command support:

- `resolve`: table or JSON
- `dig`: table, JSON, or CSV
- `collectors`: table, JSON, or CSV
- `library`: table, JSON, or CSV
- `cache`: plain text

If both `--json` and `--csv` are passed, CSV wins for commands that support it.

## Configuration

Pass a TOML file with `--config`:

```bash
wax dig https://artist.bandcamp.com/album/example-record --config ./wax.toml
```

Example:

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

Currently wired settings:

- `cache_dir`
- `request_delay_ms`
- `concurrency`
- `timeout_ms`
- `user_agent`
- `youtube_api_key`

Schema fields such as `max_collectors` and `max_depth` exist in the config type, but command execution still takes those values from CLI arguments and built-in defaults.

## Supported URLs

- Bandcamp album URLs for `resolve`, `collectors`, and `dig`
- Bandcamp fan URLs for `library`
- SoundCloud track URLs for `resolve` and `dig`
- SoundCloud playlist URLs for `resolve`
- YouTube watch URLs for `resolve`
- YouTube watch URLs with `v=` and `list=` for `dig`
- `youtu.be` short URLs for `resolve`, and for `dig` when they also include `list=`
- `music.youtube.com` watch URLs for `resolve`, and for `dig` when they also include `list=`

Unsupported today:

- SoundCloud `collectors`
- SoundCloud `library`
- SoundCloud playlist `dig`
- YouTube `collectors`
- YouTube `library`

## Notes

- `wax` only uses publicly visible Bandcamp, SoundCloud, and YouTube data
- SoundCloud support currently focuses on track resolution and a prototype `dig` flow
- YouTube support requires a YouTube Data API key and currently focuses on video resolution plus playlist-overlap `dig`
- private or unavailable public data is skipped when possible
- repeated runs are faster when the cache is warm
- some commands may return `no usable public data found` when a page is valid but the needed public signals are missing

## Hacking

Developer-facing notes live in [HACKING.md](HACKING.md).
