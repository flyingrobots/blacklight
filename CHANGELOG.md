# Changelog

## [0.3.0] - 2026-02-11

### Added — M3 Serve

- Axum web server with embedded static file serving (`blacklight serve`)
- SQLite connection pool with `spawn_blocking` for async-safe DB access
- REST API endpoints:
  - `GET /api/sessions` — paginated session list with project/date filtering
  - `GET /api/sessions/:id` — session detail with outcomes
  - `GET /api/sessions/:id/messages` — paginated message thread with content blocks
  - `GET /api/sessions/:id/tools` — tool call history
  - `GET /api/sessions/:id/files` — file references
  - `GET /api/search` — FTS5 full-text search with project/kind filtering
  - `GET /api/analytics/*` — overview, daily stats, models, tools, projects, outcomes, coverage
  - `GET /api/projects` — rich per-project detail with tool fingerprints, file counts, date ranges
  - `GET /api/content/:hash` — blob retrieval
  - `GET /api/files` — file provenance with operation history
  - `GET /api/storage` — storage overview with dedup ratio
- CORS and compression middleware
- Vue 3 + TypeScript frontend (Vite, vue-router)
  - Dashboard with overview stats and daily activity chart
  - Sessions list with filtering, session detail with message thread
  - Full-text search with kind/project filters and highlighted snippets
  - Analytics page with daily chart, model usage, tool frequency, outcomes
  - Projects page with treemap-like grid and stacked tool-frequency fingerprint bars
  - Files page with provenance tracking
  - Storage page with dedup stats
  - BLACKLIGHT.svg logo in sidebar (CSS-filtered for dark theme)
- Reusable Vue components: ContentBlock, DailyChart, MessageThread, SearchResult, SessionCard, ThinkingBlock, ToolCallCard

## [0.2.0] - 2026-02-11

### Added — M2 Excavation

- Full indexer pipeline: `blacklight index` now crawls `~/.claude/` and populates the database
- File scanner with 9 file classifications (session JSONL, tasks, facets, plans, history, etc.)
- Change detection for incremental indexing — second run skips unchanged files
- Streaming JSONL reader with byte offset tracking and seek support
- Progress message optimization — raw string scan skips ~28% of lines without any JSON parsing
- Message handlers for all types: assistant (text, tool_use, thinking), user (text, tool_result), system, summary
- Batch DB operations — accumulate inserts per line, flush every 500 messages in a single transaction
- ToolUseTracker — maps Read/Write/Edit tool calls to file paths for the file_references table
- Session metadata parser (sessions-index.json) with FK-safe ordering
- Auto-creation of minimal session rows for subagent files
- Structured data parsers: tasks + dependencies, facets (outcomes/categories/friction), stats-cache, plans, history
- `--full` flag to force complete re-index
- `--verbose` flag for per-file logging
- 91 tests total (39 unit + 52 integration)
- Realistic integration tests using public domain text (Shakespeare, Whitman, Dickinson, Twain, Poe)

## [0.1.0] - 2026-02-11

### Added — M1 Foundation

- Project scaffold with Cargo.toml and all core dependencies
- CLI skeleton with `index`, `serve`, `search`, and `stats` subcommands (clap)
- SQLite schema: 15 tables, 11 indexes, FTS5 virtual table
- Migration runner with version tracking (`PRAGMA user_version`)
- Connection manager with WAL mode and performance PRAGMAs
- Core data types for all `~/.claude/` JSON formats (serde)
  - Session JSONL messages (user, assistant, progress, system, summary, file-history-snapshot, queue-operation)
  - Session index entries
  - Task records
  - Session facets (outcomes, friction, categories)
  - Stats cache with daily activity and model usage
- Content-addressable blob store with BLAKE3 hashing
  - Insert, retrieve, existence check, batch insert
  - Blob reference tracking (hash -> message -> context)
  - Dedup threshold (256 bytes)
- FTS5 full-text search operations
  - Content indexing with deduplication
  - BM25-ranked search with highlighted snippets
  - Porter stemming and unicode61 tokenizer
- 52 tests covering DB, models, and content modules
