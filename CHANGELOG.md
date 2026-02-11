# Changelog

## [0.1.0] - 2026-02-11

### Added

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
