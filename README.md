# Blacklight

A local-first tool that indexes `~/.claude/` — the hidden directory where Claude Code stores every conversation, tool call, file edit, and session artifact — and makes it visible, searchable, and understandable.

## Why

Anthropic stores 4.5GB+ of structured data in `~/.claude/` with no UI to browse it. Blacklight gives you:

- **Full-text search** across every conversation you've had with Claude
- **Session replay** with rendered markdown, tool call cards, and thinking blocks
- **Analytics** — token usage, model breakdown, daily/hourly patterns, session outcomes
- **Storage analysis** — where disk space is going, deduplication stats
- **File impact** — which files Claude has touched most across sessions

## Quick Start

```bash
cargo build --release
./target/release/blacklight index     # index ~/.claude/ data
./target/release/blacklight serve     # start web UI on localhost:3141
./target/release/blacklight search "auth bug"   # search from terminal
./target/release/blacklight stats     # usage overview
```

## Architecture

Single Rust binary. Single SQLite file. No daemon, no external services.

```
blacklight
├── index     — crawl ~/.claude/, build SQLite DB
├── serve     — REST API + embedded web frontend
├── search    — full-text search from the terminal
└── stats     — usage statistics from the terminal
```

**Key design decisions:**

- **BLAKE3 content-addressable blob store** — identical content stored once, referenced many times. That 74KB file appearing 300 times? Stored once.
- **FTS5 with porter stemming** — search "running" matches "run". BM25 ranking. Highlighted snippets.
- **Progress message optimization** — `normalizedMessages` (96% of each progress line) is skipped entirely, cutting parseable data from ~4.5GB to ~1GB.
- **WAL mode SQLite** — web server reads while indexer writes.

## Tech Stack

| Component | Choice |
|-----------|--------|
| Language | Rust |
| Database | SQLite (bundled via rusqlite) |
| Full-text search | FTS5 with porter unicode61 tokenizer |
| Content hashing | BLAKE3 |
| CLI | clap |
| Web server | axum + tower-http |
| Frontend | Svelte (planned) |

## Development

```bash
cargo build       # build
cargo test        # run tests (52 tests)
cargo clippy      # lint
cargo run -- --help
```

## License

MIT
