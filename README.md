# Blacklight

<p align="center">
  <img src="./frontend/src/assets/BLACKLIGHT.svg" alt="Blacklight" />
</p>

Blacklight is a local-first observability and memory layer for AI coding sessions.
It ingests session artifacts from Claude, Gemini, and Codex directories, indexes them into SQLite, and serves an embedded web UI for search, replay, analytics, provenance, and enrichment.

Core runtime is one Rust binary plus one SQLite database.

## What Blacklight Does

- Indexes multi-source session history (`claude`, `gemini`, `codex`) into a single local database.
- Preserves source files in a backup CAS (Git CAS mode or simple hash-addressed mode).
- Stores cryptographic fingerprints for messages, tool calls, and sessions.
- Deduplicates large content blobs and exposes full-text search with SQLite FTS5.
- Serves a web dashboard with session drilldown, project analytics, file provenance, and storage stats.
- Runs optional AI enrichment (title/summary/tags) with approval workflow.
- Supports background scheduling for periodic indexing and enrichment.

## Screenshots

| Dashboard | Sessions | Analytics |
|-----------|----------|-----------|
| ![Dashboard](screenshots/dashboard-slate.png) | ![Sessions](screenshots/sessions-slate.png) | ![Analytics](screenshots/analytics-slate.png) |

| Session Detail | Search | Projects |
|----------------|--------|----------|
| ![Session Detail](screenshots/session-detail-slate.png) | ![Search](screenshots/search-slate.png) | ![Projects](screenshots/projects-slate.png) |

## How It Works

1. Source discovery: reads configured `[[sources]]` and auto-discovers common local paths (`~/.gemini`, `~/.codex`, Claude Desktop session directories on macOS).
2. File scan and classification: classifies known artifacts (session JSONL/JSON, indexes, tasks, facets, stats, plans, history).
3. Incremental change detection: compares files against `indexed_files` (mtime, size, byte offset) and skips unchanged data unless `--full`.
4. Parsing and normalization: routes each source format into unified relational tables (`sessions`, `messages`, `content_blocks`, `tool_calls`, `file_references`).
5. Backup and provenance: writes source backups, records CAS hashes, and updates per-turn/per-session fingerprints.
6. Search and analytics: indexes text in FTS5 and exposes query endpoints for UI pages.
7. Runtime control: exposes `/api` endpoints plus WebSocket notifications for HUD progress and alerts.

## Supported Sources

| Kind | Typical Path | Parsed Artifacts |
|------|--------------|------------------|
| `claude` | `~/.claude/` | `projects/**/sessions-index.json`, `projects/**/*.jsonl`, tasks, facets, plans, history, stats |
| `gemini` | `~/.gemini/` | Gemini session JSON (`session-*.json` in chats paths) |
| `codex` | `~/.codex/` | Codex rollout JSONL (`sessions/**/rollout-*.jsonl`) |

Auto-discovery is additive: discovered paths are included unless already configured.

## Quick Start (Local)

### Prerequisites

- Rust toolchain
- Node.js (for frontend build)
- Git
- Optional for enrichment:
  - Ollama running locally
  - or `GOOGLE_API_KEY`
  - or `claude` CLI on `PATH`

### Build and Run

```bash
# from repo root
npm install
npm run build

# optional: write a default config file
./target/debug/blacklight init

# index your local session data
./target/debug/blacklight index

# serve dashboard at http://127.0.0.1:3141
./target/debug/blacklight serve
```

`npm run build` builds frontend assets and backend binary. The Rust server embeds `frontend/dist`, so frontend assets must exist before building the backend.

## Docker

```bash
docker compose up
```

The compose setup maps:

- `${HOME}/.claude -> /root/.claude`
- `${HOME}/.blacklight -> /root/.blacklight`

and exposes `3141`.

## Configuration

Default config path: `~/.blacklight/blacklight.toml`

Create it with:

```bash
blacklight init
```

Example:

```toml
db = "~/.blacklight/blacklight.db"
backup_dir = "~/.blacklight/backups/"
backup_mode = "gitcas" # gitcas | simple
log_level = "info"

[[sources]]
name = "claude"
path = "~/.claude/"
kind = "claude"
cas_prefix = "claude"

[[sources]]
name = "personal-gemini"
path = "~/.gemini/"
kind = "gemini"
cas_prefix = "gemini"

[[sources]]
name = "codex"
path = "~/.codex/"
kind = "codex"
cas_prefix = "codex"

[indexer]
verbose = false
skip_dirs = ["cache", "statsig", "shell-snapshots", "session-env", "ide", "paste-cache", "debug", "telemetry"]

[enrichment]
concurrency = 5
auto_approve_threshold = 0.80
ollama_url = "http://localhost:11434"
ollama_model = ""
google_api_key = ""
preferred_backend = "auto" # auto | ollama | gemini | claude-cli
```

### Effective precedence

- CLI flags override config values.
- For log level: `RUST_LOG` overrides config `log_level`.
- For enrichment secrets/model selection: environment variables (for example `GOOGLE_API_KEY`, `OLLAMA_MODEL`) override config fields.

## CLI Reference

```bash
blacklight init
blacklight index [--full] [--source <path>] [--verbose]
blacklight serve [--port <n>] [--no-open]
blacklight enrich [--limit <n>] [--concurrency <n>] [--force]
```

Global flags:

- `--db <path>` custom SQLite path
- `--config <path>` custom config file
- `--claude-dir <path>` override the configured `claude` source path for the run

Current status:

- `blacklight search` and `blacklight stats` are declared but not yet implemented in CLI.
- Search and stats are available in the web UI/API.

## Web Dashboard

Primary routes:

- `/` Dashboard
- `/sessions` Sessions list
- `/sessions/:id` Session detail (messages/tools/files/raw)
- `/projects` Project breakdown
- `/search` Full-text search
- `/analytics` Activity/model/tool/outcome analytics
- `/files` File provenance view
- `/storage` Blob and dedup storage stats
- `/review` Enrichment approval queue

Global controls:

- Indexer HUD (start/pause/resume/stop indexer)
- Enrichment controls
- Migration controls (V4 provenance migration)
- Scheduler controls (interval, enrichment toggle, concurrency)
- Live notifications over WebSocket (`/api/ws`)

## API Surface (High Level)

Base path: `/api`

- Sessions: `/sessions`, `/sessions/{id}`, `/sessions/{id}/messages`, `/sessions/{id}/tools`, `/sessions/{id}/files`, `/sessions/{id}/raw`
- Search: `/search`
- Analytics: `/analytics/overview`, `/analytics/coverage`, `/analytics/daily`, `/analytics/daily-projects`, `/analytics/models`, `/analytics/tools`, `/analytics/projects`, `/analytics/llms`, `/analytics/outcomes`
- Files and storage: `/files`, `/storage`, `/content/{hash}`
- Background control: `/indexer/*`, `/enrichment/*`, `/schedule`, `/migration/*`
- Review workflow: `/review`, `/review/{session_id}/approve`, `/review/{session_id}/reject`, `/review/approve-all`
- Notifications: `/ws`

## Enrichment Backends

Enrichment generates title, summary, and tags per session.

Backend selection behavior:

- `preferred_backend = "auto"`: Ollama (if available) -> Gemini (if key configured) -> Claude CLI
- `preferred_backend = "ollama"`: Ollama (auto-detect model if not set)
- `preferred_backend = "gemini"`: Gemini when key exists, otherwise fallback path may resolve to Claude CLI
- `preferred_backend = "claude-cli"`: force Claude CLI

Low-confidence tags are marked `pending_review` and surfaced on the `/review` page.

## Storage and Provenance Model

- SQLite in WAL mode with migrations (`user_version` currently 5).
- FTS5 table `fts_content` with porter tokenizer for content search.
- Dedup threshold: content >= 256 bytes is stored in `content_store` and referenced by hash.
- Backup table `session_backups` links sessions to CAS content hashes.
- Fingerprinting fields on sessions/messages/tool calls support tamper-evident provenance checks.

## Development

```bash
# backend tests
cargo test

# full dev loop (backend + Vite with /api proxy)
npm install
npm run dev

# frontend-only loop
cd frontend
npm install
npm run dev
```

Vite dev server runs on `5173` and proxies `/api` + WebSocket traffic to `127.0.0.1:3141`.

## Repository Layout

```text
src/
  main.rs                CLI entrypoint
  config.rs              Config model and loader
  db.rs                  SQLite open + migrations
  content.rs             Hashing, blob store, FTS helpers
  enrich.rs              Enrichment pipeline/backends
  indexer/               Scanner, parsers, router, DB batch ops
  server/                Axum API, scheduler, embedded frontend, queries
frontend/
  src/                   Vue app, routes, views, components
  vite.config.ts         Dev proxy/build config
tests/                   Integration-style backend tests
scripts/                 Utility scripts (for example screenshot capture)
```

## Known Limitations

- CLI `search` and `stats` commands are placeholders.
- Scheduler settings are persisted in the database (`schedule_config`) and controlled from API/HUD at runtime.
- Embedded frontend requires built assets in `frontend/dist` when compiling backend from source.

## Additional Documentation

- Architecture deep dive: [`ARCHITECTURE.md`](ARCHITECTURE.md)
- Product and delivery roadmap: [`ROADMAP.md`](ROADMAP.md)
- Technical planning notes: [`TECH-PLAN.md`](TECH-PLAN.md)
- Release history: [`CHANGELOG.md`](CHANGELOG.md)

## License

Apache-2.0
