# wax

[![CI](https://github.com/jdhoffa/wax/actions/workflows/ci.yml/badge.svg)](https://github.com/jdhoffa/wax/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

`wax` is a Rust CLI for digging through public music discovery signals from Bandcamp and SoundCloud.

Give it a URL and it will detect the platform automatically:

- Bandcamp album URLs use collector overlap
- SoundCloud track URLs use public likers and nearby likes in each liker feed

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

Inspect the available commands:

```bash
wax --help
wax dig --help
```

## Commands

### `wax dig <url>`

Runs the recommendation flow for a supported URL.

Examples:

```bash
wax dig https://artist.bandcamp.com/album/example-record
wax dig https://artist.bandcamp.com/album/example-record --max-collectors 50 --limit 20
wax dig https://soundcloud.com/chvrches/the-mother-we-share --max-collectors 25 --limit 10
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
- SoundCloud playlist URLs are not supported for `dig` yet

### `wax resolve <url>`

Resolves a supported URL and prints canonical seed metadata.

Examples:

```bash
wax resolve https://artist.bandcamp.com/album/example-record
wax resolve https://soundcloud.com/chvrches/the-mother-we-share
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
- `--rate-limit-ms <n>`: delay between requests in milliseconds
- `--concurrency <n>`: request concurrency setting
- `--timeout-ms <n>`: per-request timeout
- `-v`, `--verbose`: enable debug logging
- `--quiet`: only print errors

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

## Notes

- `wax` only uses publicly visible Bandcamp and SoundCloud data
- SoundCloud support currently focuses on track resolution and a prototype `dig` flow
- private or unavailable public data is skipped when possible
- repeated runs are faster when the cache is warm

## Hacking

Developer-facing notes live in [HACKING.md](HACKING.md).
