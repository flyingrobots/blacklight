# Blacklight — Technical Plan

## What Is This?

A local-first app that cracks open `~/.claude/` — the hidden directory where Claude Code
stores every conversation, tool call, file edit, and session artifact — and makes it
visible, searchable, and understandable.

Anthropic stores 4.5GB+ of data in this directory with no UI to browse it. This tool
gives users transparency into what's been recorded about their interactions.

---

## Use Cases

### Privacy & Transparency
- **"What has Claude stored about me?"** — Browse everything in `~/.claude/`, classified and searchable.
- **"How much disk space is this eating?"** — Storage analysis with per-project breakdown and cleanup recommendations.
- **"Why is there 3.5GB of data for one project?"** — Redundancy analysis exposing how progress messages re-serialize entire conversation contexts.

### Productivity
- **"Find that conversation where I debugged the auth issue"** — Fuzzy full-text search across all sessions.
- **"What did I work on last week?"** — Activity timeline and session browser.
- **"Show me the plan Claude made for the refactor"** — Plan browser with markdown rendering.
- **"What tasks are still open?"** — Task/todo aggregation across sessions.

### Analytics
- **"How much am I using Claude?"** — Token spend, model breakdown, daily/hourly patterns.
- **"Where does Claude struggle?"** — Friction analysis from session outcome data.
- **"Which projects lean on Claude most?"** — Per-project usage breakdown.

### Session Replay
- **"Walk me through that session"** — Full conversation replay with tool calls, thinking blocks, file diffs — the complete picture of what Claude *did*.

---

## Data Sources

### Primary (The Beef)

| Source | Format | Size | Contents |
|--------|--------|------|----------|
| `projects/` | JSONL per session | 4.5 GB | Full conversation transcripts: every message, tool call, tool result, thinking block, file edit |
| `projects/*/sessions-index.json` | JSON | ~1 KB each | Session metadata: first prompt, summary, message count, timestamps, git branch |
| `stats-cache.json` | JSON | 9 KB | Pre-computed analytics: daily activity, model tokens, hourly distribution, session counts |
| `usage-data/facets/` | JSON per session | ~50 KB total | Session outcomes: goals, friction, satisfaction, helpfulness ratings |
| `history.jsonl` | JSONL | 1.2 MB | Flat prompt log: every user message with timestamp, project, session ID |
| `file-history/` | Text snapshots | 85 MB | Versioned file snapshots (`{hash}@v{N}`), full contents per version |

### Secondary

| Source | Format | Size | Contents |
|--------|--------|------|----------|
| `debug/` | Text logs | 278 MB | Per-session debug logs: startup traces, MCP connections, errors, performance |
| `tasks/` | JSON per task | ~50 KB | Structured task records with status and dependency graphs |
| `todos/` | JSON arrays | ~60 KB | Lightweight checklists (90% empty) |
| `plans/` | Markdown | 380 KB | Implementation plans with auto-generated names |
| `paste-cache/` | Mixed (text/JSONL/diff) | 4.4 MB | Cached clipboard contents |
| `shell-snapshots/` | Shell scripts | 480 KB | zsh environment snapshots |

### Config (Index for Context, Not Search)

| Source | Contents |
|--------|----------|
| `settings.json` | User preferences (181 bytes) |
| `statsig/` | Feature flags and A/B test assignments |
| `cache/changelog.md` | Claude Code version history |

---

## Data Shape & Gotchas

### Session JSONL — 7 Message Types

| Type | ~% | What It Is | Index? |
|------|-----|-----------|--------|
| `assistant` | 44% | Claude's responses: `text`, `tool_use`, `thinking` content blocks | **Full** |
| `user` | 21% | User messages + `tool_result` blocks from tool execution | **Full** |
| `progress` | 27% | Real-time tool execution status with `normalizedMessages` context blob | **Metadata only** |
| `file-history-snapshot` | 4% | File state tracking | **Metadata only** |
| `system` | 2% | Session events: `turn_duration`, `local_command` | **Light** |
| `summary` | <1% | Session conclusion metadata | **Full** |
| `queue-operation` | 1% | Message queue management | **Skip** |

### The 1MB+ Line Problem

Progress messages contain a `normalizedMessages` field that re-serializes the entire
conversation context (including inlined file contents) on every tool execution tick.

- 372 lines exceed 1MB (largest: 1.82MB)
- 1,624 lines exceed 500KB
- 96% of each progress line is `normalizedMessages`
- The same 74KB source file can appear hundreds of times

**Solution:** Skip `normalizedMessages` entirely. The content is already captured in
the individual `user`/`assistant` messages.

### Content Block Nesting

Assistant messages contain arrays of typed content blocks:
```
message.content[]: [
  { type: "thinking", thinking: "..." },
  { type: "text", text: "..." },
  { type: "tool_use", id: "toolu_...", name: "Read", input: { file_path: "..." } }
]
```

User messages with tool results:
```
message.content[]: [
  { type: "tool_result", tool_use_id: "toolu_...", content: "..." }
]
```

### Debug Log Format

```
ISO8601_TIMESTAMP [LEVEL] [optional_context] message
```
Multi-line entries for stack traces. Embedded JSON in some log messages.

### Edge Cases

- **Paste-cache is mixed format** — JSONL, plain text, and unified diffs share the same directory with no distinguishing extension. Need content-type sniffing.
- **File-history has no metadata sidecar** — versions inferred from `@v{N}` filename suffix.
- **Tool result files** stored separately in `projects/{uuid}/tool-results/toolu_*.txt` for large outputs.
- **All files currently valid** (zero corruption found across 2,789 files), but the indexer should handle truncated/malformed JSON gracefully since writes could be interrupted.

---

## Deduplication Strategy

### Layer 0: Don't Parse It

Skip `normalizedMessages` in progress messages entirely. This alone cuts parseable
data from ~4.5GB to ~1GB.

### Layer 1: Content-Addressable Blob Store

Every text blob above a size threshold (e.g., 512 bytes) gets BLAKE3-hashed and stored
once in a `content_store` table. Everything else references blobs by hash.

That 74KB Rust file appearing 300 times? Stored once, referenced 300 times via a
32-byte hash.

### Layer 2: FTS5 Indexes Unique Blobs

The full-text search index points at unique content blobs, not individual messages.
A separate `blob_references` table maps blobs back to every message that produced them.

Search finds the blob once → join reveals every session that touched it.

### Layer 3: File Reference Table

Since Claude reads files from actual repos, we build a `file_references` index mapping
file paths → content hashes → sessions/messages. Answers "every session that touched
this file" without storing the content more than once per unique version.

### Projected Impact

| Category | Raw | After Dedup | Reduction |
|----------|-----|-------------|-----------|
| Progress normalizedMessages | ~3.5 GB | ~0 | ~100% |
| Repeated file contents | ~500 MB | ~50 MB | ~90% |
| FTS5 index size | ~2 GB | ~200 MB | ~90% |
| Actual conversations | ~500 MB | ~500 MB | 0% |
| **Total DB** | **~4.5 GB** | **~750 MB** | **~83%** |

---

## Tech Stack — Feature-by-Feature Review

### 1. JSONL Streaming Parser

**Feature:** Parse 4.5GB of session JSONL files with 1MB+ lines, constant memory.

**Choice:** Rust + `serde_json` (line-by-line `BufReader` + `serde_json::from_str` per line)

| Alternative | Why Not |
|-------------|---------|
| Node.js streams | V8 allocates full strings for each JSON line. A 1.8MB line becomes a 1.8MB JS string, triggers GC pressure. Streaming helps throughput but not per-line memory. |
| Python `json` | CPython is 10-20x slower at JSON parsing than serde. 4.5GB would take 10+ minutes vs under a minute in Rust. GIL prevents true parallelism. |
| `simd-json` (Rust) | Faster than `serde_json` for parsing, but adds complexity and we're I/O-bound anyway. Could swap in later if CPU becomes the bottleneck. |
| `jq` / shell pipeline | Can't do structured dedup, content hashing, or database writes. |

**Rust crate:** `serde_json` + `serde` with `#[derive(Deserialize)]` structs.
Use `serde_json::from_str` per line (not `StreamDeserializer`, which is for multiple
values on one line). Read lines with `BufReader::read_line` with a large buffer.

For the `normalizedMessages` skip optimization: use `serde_json::Value` to partially
parse progress messages, or use `serde::Deserializer::ignore` to skip the field
without allocating.

---

### 2. Content Hashing / Deduplication

**Feature:** Hash all text blobs for content-addressable storage. Same content stored once regardless of how many times it appears.

**Choice:** BLAKE3

| Alternative | Why Not |
|-------------|---------|
| SHA-256 | 5-10x slower than BLAKE3. No practical benefit — we don't need cryptographic resistance, just collision avoidance. |
| xxHash / xxh3 | Non-cryptographic. Fine for hash tables, but collision probability is too high for a content-addressed store at this scale (millions of blobs). |
| MD5 | Collision-broken and slower than BLAKE3. No reason to use it. |
| CRC32 | Way too small (32-bit), guaranteed collisions at this volume. |

**Rust crate:** `blake3` — single dependency, SIMD-accelerated, 128 bytes of state.
Output: 32-byte hash, stored as 64-char hex string in SQLite.

**Threshold:** Only hash+dedup blobs >= 256 bytes. Smaller text (short user messages,
status fields) stored inline — the overhead of a hash lookup exceeds the storage savings.

---

### 3. Database

**Feature:** Structured storage for sessions, messages, tool calls, tasks, stats. Relational queries. Persistent single-file storage. Zero infrastructure.

**Choice:** SQLite (via `rusqlite`)

| Alternative | Why Not |
|-------------|---------|
| PostgreSQL | Requires a running server. Overkill for a local tool. Users would need to install and configure it. |
| DuckDB | Excellent for analytical/columnar queries but weaker for point lookups, inserts, and FTS. No built-in FTS equivalent to FTS5. Better as a secondary analytical layer, not the primary store. |
| Redis | In-memory, ephemeral by default. Not suited for 750MB of persistent structured data. No relational queries. |
| Embedded RocksDB | Key-value only. Would need to build our own query layer on top. |
| Flat files + grep | Doesn't scale. No joins, no indexes, no transactions. |

**Rust crate:** `rusqlite` with `bundled` feature (compiles SQLite from source,
guarantees FTS5 availability). WAL mode for concurrent reads during indexing.

**Why SQLite specifically wins here:**
- Single file = the database IS the index. `cp blacklight.db backup.db` and you're done.
- WAL mode = the web server can read while the indexer writes.
- PRAGMA optimizations: `journal_mode=WAL`, `synchronous=NORMAL`, `cache_size=-64000` (64MB cache), `mmap_size=268435456` (256MB mmap).
- Battle-tested at this scale — SQLite handles databases up to 281TB. 750MB is trivial.

---

### 4. Full-Text Search

**Feature:** Fuzzy search across all conversations, tool outputs, plans. Ranked results with snippets.

**Choice:** SQLite FTS5 with `porter unicode61` tokenizer + BM25 ranking

| Alternative | Why Not |
|-------------|---------|
| Tantivy (Rust) | Full Lucene-equivalent. Supports true fuzzy queries (edit distance), faceted search, better ranking. But: separate index file to manage, another dependency, more complexity. **Best candidate for a future upgrade if FTS5 search quality isn't enough.** |
| Meilisearch | Best-in-class typo-tolerant search. But: separate service to run (it's a standalone binary). Defeats the "single binary, zero infrastructure" goal. |
| Elasticsearch | Massively overkill. JVM, cluster management, 500MB+ RAM. No. |
| Typesense | Similar to Meilisearch — great search, but another service. |
| SQLite trigram tokenizer | Better for substring/typo matching than porter. But slower to build, larger index. Could layer on top of FTS5 as a secondary index. |

**FTS5 capabilities we use:**
- `porter` stemmer: "running" matches "run", "searched" matches "search"
- `unicode61` tokenizer: handles non-ASCII (file paths, identifiers, comments)
- `BM25` ranking: relevance scoring out of the box
- `snippet()` function: context snippets with highlights for search results
- `highlight()` function: in-document hit highlighting

**FTS5 limitations we accept:**
- No typo tolerance ("seach" won't match "search"). Mitigation: client-side
  suggestions using `LIKE '%seach%'` fallback or a trigram index later.
- No semantic/vector search. Mitigation: out of scope for v1. Could add later
  with SQLite `vec0` extension or external embeddings.

**Architecture note:** FTS5 indexes the `content_store` table (unique blobs), not
individual messages. This means a 74KB file that appears 300 times gets indexed once.
Join through `blob_references` to find which sessions/messages contain the match.

---

### 5. Web Server

**Feature:** Serve the frontend, provide REST/JSON API for the UI, handle search queries.

**Choice:** axum (Rust)

| Alternative | Why Not |
|-------------|---------|
| actix-web | Slightly faster raw throughput, but axum has better ergonomics, tower middleware ecosystem, and is maintained by the Tokio team. |
| Rocket | Slower compile times, more macro magic, less flexible middleware. |
| warp | Lower-level filter-based API. More boilerplate than axum for standard REST routes. |
| Express/Fastify (Node.js) | Would mean shipping two runtimes (Rust for indexer + Node for server). Single binary is cleaner. |
| Python Flask/FastAPI | Same problem — second runtime. Also slower for serving static assets and handling concurrent requests. |
| No server (static files) | SQLite can't be queried from the browser directly (without sql.js/WASM, which has size and memory limitations for a 750MB DB). |

**Rust crates:**
- `axum` — routes, handlers, JSON responses
- `tower-http` — CORS, static file serving, compression
- `tokio` — async runtime (axum's foundation)

**API shape:** REST + JSON. No GraphQL (overkill for a single-user local app).
Endpoints like:
- `GET /api/sessions?project=echo&limit=50`
- `GET /api/sessions/:id/messages`
- `GET /api/search?q=auth+bug&project=echo`
- `GET /api/stats/daily`
- `GET /api/storage`

Static frontend assets served from an embedded directory (`include_dir!` or
`rust-embed`) so the entire app is a single binary.

---

### 6. Frontend Framework

**Feature:** Interactive UI with session lists, conversation replay, search, charts.

**Choice:** Svelte (compiled, no virtual DOM)

| Alternative | Why Not |
|-------------|---------|
| React | Larger runtime (~40KB gzipped), more boilerplate, virtual DOM overhead unnecessary for this app's interaction patterns. |
| Vue | Capable, but Svelte produces smaller bundles and has simpler reactivity for this scale of app. |
| Vanilla JS + htmx | htmx is great for server-rendered partials, but conversation replay and search need client-side state management. Would end up reinventing a framework. |
| SolidJS | Smaller community, fewer component libraries. Similar perf to Svelte. |
| Preact | Lighter React, but still virtual DOM. Svelte's compiled approach wins. |

**Why Svelte:**
- Compiled to vanilla JS — tiny bundle, no runtime overhead
- Reactive by default — `$:` syntax is perfect for search/filter state
- Built-in transitions/animations — useful for collapsible tool cards
- SvelteKit not needed — this is an SPA, not SSR. Just `vite` + `svelte` plugin.

**Build:** Vite dev server for development, `vite build` produces static assets
that get embedded into the Rust binary for distribution.

---

### 7. Data Visualization

**Feature:** Activity heatmaps, token usage charts, model breakdowns, storage treemaps, hourly distribution.

**Choices:**

| Feature | Library | Why |
|---------|---------|-----|
| Activity heatmap (GitHub-style) | `cal-heatmap` or custom SVG | cal-heatmap is purpose-built for calendar heatmaps. Lightweight. D3-based custom is also viable. |
| Line/bar/pie charts | `Chart.js` | Simple API, good defaults, small bundle (~60KB). Covers 80% of charting needs. |
| Storage treemap | `D3 treemap` | Chart.js doesn't do treemaps. D3's treemap layout is the standard. |
| Hourly distribution | `Chart.js` (polar area or bar) | Standard chart type, no special library needed. |

| Alternative | Why Not |
|-------------|---------|
| Recharts | React-only. |
| Plotly | 3.5MB bundle. Way too heavy for a local tool. |
| ECharts | 1MB+ bundle. Powerful but oversized for our needs. |
| Observable Plot | Interesting but young, smaller ecosystem. |
| D3 for everything | D3 is low-level — great for custom viz (treemaps, heatmaps) but overkill for a bar chart. Use Chart.js for standard charts, D3 only where needed. |

---

### 8. Conversation Replay

**Feature:** Render full conversation threads with tool calls, thinking blocks, file diffs, message threading.

**Choices:**

| Sub-feature | Library | Why |
|-------------|---------|-----|
| Markdown rendering | `marked` | Fast, small (32KB), GFM support. Renders Claude's markdown responses. |
| Syntax highlighting | `Prism` or `highlight.js` | Prism is smaller (lighter core + per-language bundles). highlight.js has broader language support. Either works. |
| Diff rendering | `diff2html` | Standard library for rendering unified diffs with syntax highlighting. Handles the diff output from Edit tool calls. |
| Conversation layout | Custom Svelte components | No existing library matches our needs (message threading + tool cards + thinking toggles). Simple enough to build. |

| Alternative | Why Not |
|-------------|---------|
| `markdown-it` | Heavier than `marked`, plugin system adds complexity we don't need. |
| `rehype`/`remark` | AST-based pipeline. More flexible but more complex. Overkill for rendering markdown responses. |
| Monaco Editor (for code blocks) | 5MB+ bundle. We just need read-only syntax highlighting, not an editor. |

---

### 9. Incremental Indexing

**Feature:** After first full index, only process new/changed files on subsequent runs.

**Choice:** File mtime tracking in SQLite `indexed_files` table + byte offset for JSONL append detection.

| Alternative | Why Not |
|-------------|---------|
| Filesystem watcher (inotify/FSEvents) | Only catches changes while the watcher is running. Doesn't help if the app was closed when changes happened. Still need mtime comparison as a fallback. Could layer on top for live updates in v2. |
| Git-style content hashing of source files | Expensive — would need to hash every file on every run to detect changes. Mtime comparison is O(stat) per file, content hashing is O(read) per file. |
| Polling interval | Wasteful. Mtime check on-demand is better — only runs when the user opens the app or triggers a re-index. |

**Mechanism:**
1. On startup (or manual re-index), stat every file in `~/.claude/`
2. Compare `(path, mtime, size)` against `indexed_files` table
3. New file → full parse
4. Changed mtime/size → re-parse (for JSONL: seek to `last_byte_offset`, parse new lines)
5. Missing file → mark stale (soft delete references, keep content_store blobs for other refs)

---

### 10. Binary Distribution

**Feature:** Ship as a single binary. No runtime dependencies. `./blacklight` and go.

**Choice:** Rust single binary with embedded static assets

| Component | How It's Embedded |
|-----------|-------------------|
| SQLite | `rusqlite` with `bundled` feature (compiles SQLite C source into the binary) |
| Frontend assets | `rust-embed` or `include_dir!` macro (embeds HTML/JS/CSS at compile time) |
| Migrations | Embedded SQL strings (run on first startup or DB version mismatch) |

| Alternative | Why Not |
|-------------|---------|
| Electron | 150MB+ binary, ships an entire Chromium. Absurd for a local tool. |
| Tauri | Closer to what we want (Rust backend + webview), but adds webview dependency and IPC complexity. We don't need native window chrome — a browser tab is fine. |
| Docker container | Adds Docker as a dependency. Defeats "zero infrastructure." |
| Python package (pip install) | Runtime dependency, version conflicts, virtualenv management. |
| npm package | Same issues. Plus Node.js required. |

**Result:** `cargo build --release` → single binary (~10-15MB estimated). User runs
`./blacklight` → opens `http://localhost:PORT` in their browser.

---

### Summary: The Full Stack

```
┌─────────────────────────────────────────────────┐
│                   Browser Tab                    │
│  ┌─────────────────────────────────────────────┐ │
│  │  Svelte SPA                                 │ │
│  │  ├── Chart.js    (standard charts)          │ │
│  │  ├── D3          (heatmaps, treemaps)       │ │
│  │  ├── marked      (markdown rendering)       │ │
│  │  ├── Prism       (syntax highlighting)      │ │
│  │  └── diff2html   (diff rendering)           │ │
│  └─────────────────────────────────────────────┘ │
└──────────────────────┬──────────────────────────┘
                       │ HTTP/JSON API
┌──────────────────────┴──────────────────────────┐
│              Rust Binary (single)                │
│  ┌──────────────┐  ┌───────────────────────────┐ │
│  │  axum        │  │  Indexer                   │ │
│  │  (web server)│  │  ├── serde_json (parsing)  │ │
│  │  + tower-http│  │  ├── blake3 (hashing)      │ │
│  │  + rust-embed│  │  └── rusqlite (DB writes)  │ │
│  └──────┬───────┘  └──────────┬────────────────┘ │
│         │                     │                   │
│  ┌──────┴─────────────────────┴────────────────┐ │
│  │  SQLite (bundled, single file)              │ │
│  │  ├── Structured tables (sessions, messages) │ │
│  │  ├── content_store (dedup blob store)       │ │
│  │  └── FTS5 index (full-text search)          │ │
│  └─────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────┘
                       │
          reads from   │
                       ▼
              ~/.claude/ (4.5GB)
```

### Rust Crate Summary

| Crate | Purpose | Why This One |
|-------|---------|-------------|
| `serde` + `serde_json` | JSON deserialization | Industry standard, derive macros, zero-copy where possible |
| `blake3` | Content hashing | Fastest secure hash, SIMD-accelerated, Rust-native |
| `rusqlite` (bundled) | SQLite interface | Mature, `bundled` guarantees FTS5, good ergonomics |
| `axum` | HTTP server | Tokio-native, tower middleware, best ergonomics in Rust web |
| `tower-http` | Middleware (CORS, compression, static files) | Pairs with axum, modular |
| `tokio` | Async runtime | Required by axum, battle-tested |
| `rust-embed` | Embed frontend assets in binary | Simple, compile-time embedding |
| `clap` | CLI argument parsing | Standard for Rust CLIs |
| `tracing` | Structured logging | Async-aware, spans, levels |
| `anyhow` | Error handling | Ergonomic error chains for application code |

### Frontend Dependencies

| Package | Purpose | Size (gzipped) |
|---------|---------|----------------|
| `svelte` | UI framework | ~8KB runtime |
| `chart.js` | Standard charts | ~60KB |
| `d3` (subset) | Heatmaps + treemaps only | ~30KB (tree-shaken) |
| `marked` | Markdown → HTML | ~32KB |
| `prismjs` | Syntax highlighting | ~15KB core + language packs |
| `diff2html` | Diff rendering | ~40KB |

---

## System Architecture

### One Binary, Four Roles

```
blacklight
├── index     (Indexer)     — crawl ~/.claude/, build SQLite DB
├── serve     (Web Server)  — REST API + embedded frontend
├── search    (CLI Search)  — quick terminal queries
└── stats     (CLI Stats)   — quick terminal summary
```

Single Rust binary. Single SQLite file. No daemon, no external services.

### Component 1: Indexer (`blacklight index`)

The crawler. Scans `~/.claude/`, parses everything, populates the SQLite DB.

```
blacklight index                    # full index (first run), incremental (subsequent)
blacklight index --full             # force full re-index
blacklight index --source ~/other   # index a different .claude directory
```

Runs once, exits. Can also be triggered from the web UI via `POST /api/reindex`.

**Owns:** All parsing logic — JSONL streaming, JSON parsing, markdown extraction,
debug log parsing, content hashing, FTS5 population, deduplication.

### Component 2: Web Server + REST API (`blacklight serve`)

Starts a local HTTP server, opens the browser.

```
blacklight serve                    # index if stale, then serve on localhost:3141
blacklight serve --port 8080        # custom port
blacklight serve --no-open          # don't auto-open browser
```

**REST API** (consumed by the Svelte frontend, also usable standalone):

```
Sessions & Messages
  GET  /api/sessions                    ?project=&branch=&from=&to=&limit=&offset=
  GET  /api/sessions/:id
  GET  /api/sessions/:id/messages       ?type=user,assistant
  GET  /api/sessions/:id/tool-calls

Search
  GET  /api/search                      ?q=&project=&kind=&from=&to=&limit=

Analytics
  GET  /api/stats/overview              total sessions, messages, tokens, disk usage
  GET  /api/stats/daily                 daily activity timeseries
  GET  /api/stats/hourly                hourly distribution
  GET  /api/stats/models                per-model token breakdown
  GET  /api/stats/tools                 tool usage frequency
  GET  /api/stats/outcomes              session outcomes from facets

Storage
  GET  /api/storage                     disk usage by project, redundancy analysis
  GET  /api/storage/blobs               largest blobs, dedup stats

Files
  GET  /api/files                       files Claude has touched, ranked by frequency
  GET  /api/files/:path/sessions        sessions that touched this file
  GET  /api/files/:path/versions        file-history versions

Content
  GET  /api/content/:hash               fetch a blob from content_store by hash

Tasks & Plans
  GET  /api/tasks                       ?status=&session=
  GET  /api/plans                       list plan markdown files
  GET  /api/plans/:name                 rendered plan content

System
  POST /api/reindex                     trigger re-index from the UI
  GET  /api/status                      index health, last indexed, file counts
```

**Static assets:** Svelte SPA embedded in the binary via `rust-embed`. Every route
that isn't `/api/*` returns `index.html` for client-side routing.

### Component 3: CLI Search (`blacklight search`)

Quick terminal access without opening a browser.

```
blacklight search "auth bug"                    # fuzzy search across everything
blacklight search "engine_impl" --project echo  # scoped to a project
blacklight search "auth" --kind tool_output     # only tool outputs
blacklight search "refactor" --limit 5          # top 5 results
```

Output: ranked results with context snippets, session IDs, timestamps.

### Component 4: CLI Stats (`blacklight stats`)

Quick terminal summary without a browser.

```
blacklight stats                # overview: sessions, messages, tokens, disk
blacklight stats --daily        # daily activity table
blacklight stats --models       # per-model breakdown
blacklight stats --projects     # per-project breakdown
```

Output: formatted tables to stdout.

### Component Diagram

```
┌──────────────────────────────────────────────────────┐
│                  blacklight binary                    │
│                                                       │
│  ┌─────────┐  ┌──────────────────────────────────┐   │
│  │ Indexer  │  │ Web Server (axum)                │   │
│  │          │  │  ┌────────────┐ ┌─────────────┐  │   │
│  │ • scan   │  │  │ REST API   │ │ Static File │  │   │
│  │ • parse  │  │  │ /api/*     │ │ Server      │  │   │
│  │ • hash   │  │  │            │ │ (embedded   │  │   │
│  │ • index  │  │  │            │ │  Svelte SPA)│  │   │
│  │          │  │  └─────┬──────┘ └──────┬──────┘  │   │
│  └────┬─────┘  └────────┼──────────────┼─────────┘   │
│       │                 │              │              │
│  ┌────┴────┐  ┌────────┴┐      ┌──────┴───────┐     │
│  │ CLI     │  │ CLI     │      │   Browser    │     │
│  │ search  │  │ stats   │      │   Tab        │     │
│  └────┬────┘  └────┬────┘      └──────┬───────┘     │
│       │            │                   │              │
│  ┌────┴────────────┴───────────────────┴──────────┐  │
│  │              SQLite DB (single file)            │  │
│  │  • structured tables  • content_store  • FTS5   │  │
│  └────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────┘
                         │
            reads from   │
                         ▼
                ~/.claude/ (4.5GB)
```

### What's NOT a Component

- **No daemon/watcher** (v1) — index on demand. `--watch` mode is a future enhancement.
- **No separate frontend build server in prod** — Vite for development only, assets
  embedded in release binary.
- **No auth** — localhost only, single user.
- **No database server** — SQLite is embedded.

---

## Database Schema

### Core Tables

```sql
-- Session metadata (from sessions-index.json)
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,                    -- UUID
    project_path TEXT NOT NULL,             -- e.g., "/home/user/projects/echo"
    project_slug TEXT NOT NULL,             -- e.g., "echo"
    first_prompt TEXT,
    summary TEXT,
    message_count INTEGER,
    created_at TEXT NOT NULL,               -- ISO 8601
    modified_at TEXT NOT NULL,
    git_branch TEXT,
    claude_version TEXT,
    is_sidechain INTEGER DEFAULT 0,
    source_file TEXT NOT NULL               -- path to the JSONL file
);

-- Individual messages (from session JSONL)
CREATE TABLE messages (
    id TEXT PRIMARY KEY,                    -- uuid from the message
    session_id TEXT NOT NULL REFERENCES sessions(id),
    parent_id TEXT,                         -- parentUuid (nullable for root)
    type TEXT NOT NULL,                     -- "user", "assistant", "system", "summary"
    timestamp TEXT NOT NULL,
    model TEXT,                             -- only for assistant messages
    stop_reason TEXT,                       -- "end_turn", "tool_use"
    cwd TEXT,
    git_branch TEXT,
    duration_ms INTEGER                     -- for system/turn_duration messages
);

-- Content blocks within messages (flattened from message.content[])
CREATE TABLE content_blocks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    message_id TEXT NOT NULL REFERENCES messages(id),
    block_index INTEGER NOT NULL,           -- position in content array
    block_type TEXT NOT NULL,               -- "text", "tool_use", "tool_result", "thinking"
    content_hash TEXT REFERENCES content_store(hash),  -- for the text/thinking content
    tool_name TEXT,                         -- for tool_use blocks
    tool_use_id TEXT,                       -- "toolu_..." identifier
    tool_input_hash TEXT REFERENCES content_store(hash) -- for tool_use input JSON
);

-- Tool calls (denormalized view for fast queries)
CREATE TABLE tool_calls (
    id TEXT PRIMARY KEY,                    -- tool_use_id
    message_id TEXT NOT NULL REFERENCES messages(id),
    session_id TEXT NOT NULL REFERENCES sessions(id),
    tool_name TEXT NOT NULL,
    input_hash TEXT REFERENCES content_store(hash),
    output_hash TEXT REFERENCES content_store(hash),
    timestamp TEXT NOT NULL
);
```

### Deduplication Tables

```sql
-- Content-addressable blob store
CREATE TABLE content_store (
    hash TEXT PRIMARY KEY,                  -- BLAKE3 hex
    content TEXT NOT NULL,
    size INTEGER NOT NULL,
    kind TEXT                               -- "text", "tool_output", "thinking", "file", "plan"
);

-- Maps blobs to every message that produced/consumed them
CREATE TABLE blob_references (
    hash TEXT NOT NULL REFERENCES content_store(hash),
    message_id TEXT NOT NULL REFERENCES messages(id),
    context TEXT NOT NULL,                  -- "response_text", "tool_input", "tool_output", "thinking"
    PRIMARY KEY (hash, message_id, context)
);

-- File path → content hash → sessions
CREATE TABLE file_references (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,
    content_hash TEXT NOT NULL REFERENCES content_store(hash),
    session_id TEXT NOT NULL REFERENCES sessions(id),
    message_id TEXT NOT NULL REFERENCES messages(id),
    operation TEXT NOT NULL                  -- "read", "write", "edit", "grep_match"
);
```

### Structured Data Tables

```sql
-- Tasks (from tasks/*.json)
CREATE TABLE tasks (
    id TEXT NOT NULL,
    session_id TEXT NOT NULL,               -- parent directory UUID
    subject TEXT NOT NULL,
    description TEXT NOT NULL,
    active_form TEXT,
    status TEXT NOT NULL,                   -- "pending", "in_progress", "completed"
    PRIMARY KEY (session_id, id)
);

CREATE TABLE task_dependencies (
    session_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    depends_on TEXT NOT NULL,
    PRIMARY KEY (session_id, task_id, depends_on)
);

-- Session outcomes (from usage-data/facets/*.json)
CREATE TABLE session_outcomes (
    session_id TEXT PRIMARY KEY,
    underlying_goal TEXT,
    outcome TEXT,                            -- "fully_achieved", "mostly_achieved", etc.
    helpfulness TEXT,                        -- "essential", "very_helpful", etc.
    session_type TEXT,                       -- "single_task", "multi_task", etc.
    primary_success TEXT,
    friction_detail TEXT,
    brief_summary TEXT
);

CREATE TABLE outcome_categories (
    session_id TEXT NOT NULL REFERENCES session_outcomes(session_id),
    category TEXT NOT NULL,
    count INTEGER DEFAULT 1,
    PRIMARY KEY (session_id, category)
);

CREATE TABLE outcome_friction (
    session_id TEXT NOT NULL REFERENCES session_outcomes(session_id),
    friction_type TEXT NOT NULL,
    count INTEGER DEFAULT 1,
    PRIMARY KEY (session_id, friction_type)
);

-- Daily stats (from stats-cache.json)
CREATE TABLE daily_stats (
    date TEXT PRIMARY KEY,
    message_count INTEGER,
    session_count INTEGER,
    tool_call_count INTEGER
);

CREATE TABLE model_usage (
    model TEXT PRIMARY KEY,
    input_tokens INTEGER,
    output_tokens INTEGER,
    cache_read_tokens INTEGER,
    cache_creation_tokens INTEGER
);
```

### Full-Text Search

```sql
-- FTS5 index over unique content blobs
CREATE VIRTUAL TABLE fts_content USING fts5(
    hash UNINDEXED,
    kind,
    content,
    tokenize='porter unicode61'
);

-- Trigger: insert into FTS when content_store gets a new blob
```

### Indexer State

```sql
-- Track what's been indexed for incremental updates
CREATE TABLE indexed_files (
    file_path TEXT PRIMARY KEY,
    mtime_ms INTEGER NOT NULL,
    size_bytes INTEGER NOT NULL,
    last_byte_offset INTEGER DEFAULT 0,     -- for appended JSONL files
    indexed_at TEXT NOT NULL
);
```

---

## Indexer Pipeline

### Phase 1: Discovery

Scan `~/.claude/` directory structure. Build a manifest of all files with their
types, sizes, and mtimes. Compare against `indexed_files` table to find new/changed files.

### Phase 2: Session Metadata

Parse all `sessions-index.json` files → populate `sessions` table.
Fast pass, small files.

### Phase 3: Conversation Indexing (The Big One)

Stream each session JSONL file line by line:

```
for each line:
    parse JSON
    match type:
        "progress"  → extract metadata (timestamp, tool status), SKIP normalizedMessages
        "assistant" → extract content blocks:
            "text"     → hash content → content_store + fts_content
            "tool_use" → hash input → content_store, record in tool_calls
            "thinking" → hash content → content_store (optional FTS)
        "user"      → extract content blocks:
            "text"        → hash → content_store + fts_content
            "tool_result" → hash output → content_store + fts_content
        "system"    → extract duration_ms or local_command
        "summary"   → store as-is
        "queue-operation" → skip

    insert message metadata into messages table
    update indexed_files with current byte offset
```

### Phase 4: Structured Data

- Parse `tasks/*.json` → `tasks` + `task_dependencies`
- Parse `usage-data/facets/*.json` → `session_outcomes` + related tables
- Parse `stats-cache.json` → `daily_stats` + `model_usage`
- Parse `plans/*.md` → `content_store` + FTS (treat whole file as a blob)
- Parse `history.jsonl` → cross-reference with sessions

### Phase 5: Debug Logs (Optional)

Parse `debug/*.txt` line by line, extract structured fields.
Large volume (278MB), lower value for search. Could defer to a `--full` flag.

### Incremental Mode

On subsequent runs:
- Check `indexed_files` for mtime changes
- For JSONL files that grew: seek to `last_byte_offset`, parse only new lines
- For new files: full parse
- For deleted files: remove from index (or mark stale)

---

## App Views

### 1. Home Dashboard
- GitHub-style activity heatmap (messages per day)
- Aggregate stats: sessions, messages, tokens, estimated cost, disk usage
- Hourly usage distribution chart
- Model usage breakdown (pie/donut chart)
- Recent sessions quick list
- Top projects by usage

### 2. Session Explorer
- Filterable list: by project, date range, git branch, outcome, model
- Sort by: date, message count, duration
- Search within session list
- Each row: first prompt, summary, message count, model, outcome badge

### 3. Conversation Replay
- Full rendered conversation thread
- User messages and assistant responses
- Tool calls as collapsible cards (tool name, inputs, outputs)
- Thinking blocks (toggle visibility)
- File edits shown as syntax-highlighted diffs
- Session metadata sidebar (project, branch, model, duration)
- Timeline scrubber / message navigator

### 4. Search
- Full-text fuzzy search bar (FTS5 + BM25 ranking)
- Filters: project, date range, message type, tool name, content kind
- Results with context snippets and highlights
- Click through to conversation replay at the matched message

### 5. Analytics
- Token usage over time (stacked area chart by model)
- Cost estimates (apply published per-token pricing)
- Tool usage breakdown (bar chart: which tools, how often)
- Session outcomes (from facets: stacked bar of fully/mostly/partially achieved)
- Friction analysis (what types of friction, how often)
- Per-project breakdowns of all the above

### 6. Storage
- Treemap: disk usage by project
- Breakdown: raw JSONL vs actual unique content
- Redundancy score per project
- Largest sessions / largest blobs
- Cleanup recommendations

### 7. File Impact
- Files Claude has touched, ranked by frequency
- File-history timeline with version diffs
- Per-file: which sessions, what operations (read/write/edit)

### 8. Plans & Tasks
- Plan browser with rendered markdown
- Task board: grouped by session, filterable by status
- Dependency graph visualization

---

## Open Questions

- [x] Project name: **Blacklight** — reveals what's hidden in `~/.claude/`
- [ ] License?
- [ ] Should we watch `~/.claude/` for live changes (filesystem watcher) or index on-demand?
- [ ] Should debug logs be indexed by default or behind a flag?
- [ ] Do we want to support multiple `~/.claude/` directories (e.g., for different machines)?
- [ ] Thinking blocks: index for search or just store for replay?
- [ ] Should the web UI support dark mode? (yes)
- [ ] Auth for the local web server? (probably unnecessary for localhost-only)
