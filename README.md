# wax

`wax` is a Rust CLI for digging through public Bandcamp collector overlap.

Given a Bandcamp album URL, `wax` resolves the seed album, finds public collectors who own it, inspects their public libraries, and ranks related albums worth exploring.

## What It Does

- Resolves a Bandcamp album URL into normalized seed metadata
- Finds public collectors for the seed album
- Crawls public libraries for a bounded set of collectors
- Ranks related albums by collector overlap
- Prints human-readable tables or machine-readable JSON and CSV
- Caches fetched pages locally to make repeated runs faster

## Install

Build and install the binary into Cargo's bin directory:

```bash
cargo install --path .
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

Run the main recommendation flow with a Bandcamp album URL:

```bash
wax dig https://artist.bandcamp.com/album/example-record
```

Inspect the available commands:

```bash
wax --help
wax dig --help
```

## Commands

### `wax dig <album-url>`

Runs the full discovery flow and prints ranked recommendations.

```bash
wax dig https://artist.bandcamp.com/album/example-record
wax dig https://artist.bandcamp.com/album/example-record --max-collectors 50 --limit 20
wax dig https://artist.bandcamp.com/album/example-record --exclude-artist --sort overlap
wax dig https://artist.bandcamp.com/album/example-record --json
```

Useful flags:

- `--max-collectors <n>`: maximum collectors to sample from the seed album
- `--max-depth <n>`: reserved crawl-depth flag, currently defaults to `1`
- `--limit <n>`: maximum recommendations to print
- `--sample top|random`: keep the first collectors found or shuffle before truncating
- `--min-overlap <n>`: minimum overlap count required for a recommendation
- `--exclude-artist`: drop records by the same artist as the seed
- `--exclude-label`: drop records on the same label as the seed
- `--tag <tag>`: require matching tags on candidate records
- `--sort score|overlap`: sort recommendations by score or raw overlap

### `wax resolve <album-url>`

Resolves an album URL and prints seed metadata.

```bash
wax resolve https://artist.bandcamp.com/album/example-record
```

### `wax collectors <album-url>`

Lists public collectors discovered for a seed album.

```bash
wax collectors https://artist.bandcamp.com/album/example-record
wax collectors https://artist.bandcamp.com/album/example-record --max-collectors 100
```

### `wax library <fan-url>`

Prints albums from a public Bandcamp fan page.

```bash
wax library https://bandcamp.com/fanname
wax library https://bandcamp.com/fanname --limit 50
```

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
```

Use `--csv` for CSV output:

```bash
wax dig https://artist.bandcamp.com/album/example-record --csv
wax collectors https://artist.bandcamp.com/album/example-record --csv
wax library https://bandcamp.com/fanname --csv
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

## Notes

- `wax` only uses publicly visible Bandcamp pages
- private or unavailable fan libraries are skipped
- if no usable public data is available, the command exits with a non-zero status
- repeated runs are faster when the cache is warm

## Development

Run the test suite:

```bash
cargo test
```

Run the CLI without installing:

```bash
cargo run -- dig https://artist.bandcamp.com/album/example-record
```

The current implementation is in [Cargo.toml](Cargo.toml), [src/cli.rs](src/cli.rs), and [src/app.rs](src/app.rs). The broader product notes live in [SPEC.md](SPEC.md).
