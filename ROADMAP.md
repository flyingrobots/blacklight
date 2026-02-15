# Blacklight — Roadmap

## Milestones Overview

| # | Milestone | Theme | Depends On |
|---|-----------|-------|------------|
| 1 | **FOUNDATION** | Project scaffold, SQLite schema, core types | — |
| 2 | **EXCAVATION** | Indexer: crawl `~/.claude/`, populate DB | FOUNDATION |
| 3 | **LENS** | CLI: search + stats from the terminal | EXCAVATION |
| 4 | **SIGNAL** | Web server + REST API | EXCAVATION |
| 5 | **MIRROR** | Frontend: dashboard, session explorer, search | SIGNAL |
| 6 | **REPLAY** | Conversation replay view | MIRROR |
| 7 | **SPECTRUM** | Analytics, storage analysis, file impact | MIRROR |

```
FOUNDATION ──→ EXCAVATION ──→ LENS
                    │
                    ├──→ SIGNAL ──→ MIRROR ──→ REPLAY
                    │                  │
                    │                  └──→ SPECTRUM
                    │
                    └── (LENS and SIGNAL are independent after EXCAVATION)
```

---

---

# M1: FOUNDATION

> Project scaffolding, SQLite schema, core Rust types, content-addressable store.

## F1.1: Project Scaffolding

### Description
Initialize the Rust project with workspace structure, dependencies, and build configuration.

### Tasks

#### T1.1.1: Initialize Cargo workspace

**Requirements:**
- `cargo init` with binary target
- Workspace-ready `Cargo.toml` (even if single crate for now)
- `.gitignore` for Rust (`/target`, `*.db`, etc.)
- `CLAUDE.md` with project conventions

**User Story:**
> As a developer, I can `cargo build` from a fresh clone and get a compiling binary.

**Test Plan:**
- [ ] `cargo build` succeeds with zero warnings
- [ ] `cargo test` runs (even if no tests yet)
- [ ] `cargo clippy` passes
- [ ] `.gitignore` excludes `target/`, `*.db`, `*.db-wal`, `*.db-shm`

#### T1.1.2: Add core dependencies

**Requirements:**
Add to `Cargo.toml`:
- `serde`, `serde_json` — JSON parsing
- `rusqlite` with `bundled` and `vtab` features — SQLite + FTS5
- `blake3` — content hashing
- `clap` with `derive` feature — CLI argument parsing
- `anyhow` — error handling
- `tracing`, `tracing-subscriber` — structured logging
- `tokio` with `full` feature — async runtime
- `axum`, `tower-http` — web server (can be added later, but declare now)

**User Story:**
> As a developer, all dependencies are declared and compile together without conflicts.

**Test Plan:**
- [ ] `cargo build` succeeds with all deps
- [ ] `cargo tree` shows expected dependency graph
- [ ] No duplicate major versions of any crate

---

## F1.2: SQLite Schema & Migrations

### Description
Define the complete database schema and implement a migration system that runs on first startup or version mismatch.

### Tasks

#### T1.2.1: Implement migration runner

**Requirements:**
- Embed SQL migration files as strings in the binary
- On DB open: check `PRAGMA user_version` against expected version
- If version mismatch or new DB: run migrations sequentially
- Each migration increments `user_version`
- Wrap all migrations in a transaction (all-or-nothing)
- Log migration progress via `tracing`

**User Story:**
> As a user, the database is created and migrated automatically on first run — no manual setup.

**Test Plan:**
- [ ] Fresh DB: all migrations run, `user_version` set correctly
- [ ] Existing DB at current version: no migrations run
- [ ] Existing DB at older version: only new migrations run
- [ ] Failed migration: transaction rolls back, DB unchanged
- [ ] Migration runner is idempotent (safe to call multiple times)

#### T1.2.2: Create V1 migration — core tables

**Requirements:**
Implement the full schema from TECH-PLAN.md as migration 001:

**Sessions table:**
- `id TEXT PRIMARY KEY` (UUID)
- `project_path TEXT NOT NULL`
- `project_slug TEXT NOT NULL`
- `first_prompt TEXT`
- `summary TEXT`
- `message_count INTEGER`
- `created_at TEXT NOT NULL` (ISO 8601)
- `modified_at TEXT NOT NULL`
- `git_branch TEXT`
- `claude_version TEXT`
- `is_sidechain INTEGER DEFAULT 0`
- `source_file TEXT NOT NULL`

**Messages table:**
- `id TEXT PRIMARY KEY` (UUID)
- `session_id TEXT NOT NULL REFERENCES sessions(id)`
- `parent_id TEXT`
- `type TEXT NOT NULL` (user, assistant, system, summary)
- `timestamp TEXT NOT NULL`
- `model TEXT`
- `stop_reason TEXT`
- `cwd TEXT`
- `git_branch TEXT`
- `duration_ms INTEGER`

**Content blocks table:**
- `id INTEGER PRIMARY KEY AUTOINCREMENT`
- `message_id TEXT NOT NULL REFERENCES messages(id)`
- `block_index INTEGER NOT NULL`
- `block_type TEXT NOT NULL` (text, tool_use, tool_result, thinking)
- `content_hash TEXT REFERENCES content_store(hash)`
- `tool_name TEXT`
- `tool_use_id TEXT`
- `tool_input_hash TEXT REFERENCES content_store(hash)`

**Tool calls table (denormalized):**
- `id TEXT PRIMARY KEY` (tool_use_id)
- `message_id TEXT NOT NULL REFERENCES messages(id)`
- `session_id TEXT NOT NULL REFERENCES sessions(id)`
- `tool_name TEXT NOT NULL`
- `input_hash TEXT REFERENCES content_store(hash)`
- `output_hash TEXT REFERENCES content_store(hash)`
- `timestamp TEXT NOT NULL`

**Content store:**
- `hash TEXT PRIMARY KEY` (BLAKE3 hex)
- `content TEXT NOT NULL`
- `size INTEGER NOT NULL`
- `kind TEXT` (text, tool_output, thinking, file, plan)

**Blob references:**
- `hash TEXT NOT NULL REFERENCES content_store(hash)`
- `message_id TEXT NOT NULL REFERENCES messages(id)`
- `context TEXT NOT NULL`
- `PRIMARY KEY (hash, message_id, context)`

**File references:**
- `id INTEGER PRIMARY KEY AUTOINCREMENT`
- `file_path TEXT NOT NULL`
- `content_hash TEXT NOT NULL REFERENCES content_store(hash)`
- `session_id TEXT NOT NULL REFERENCES sessions(id)`
- `message_id TEXT NOT NULL REFERENCES messages(id)`
- `operation TEXT NOT NULL`

**Tasks:**
- `id TEXT NOT NULL`
- `session_id TEXT NOT NULL`
- `subject TEXT NOT NULL`
- `description TEXT NOT NULL`
- `active_form TEXT`
- `status TEXT NOT NULL`
- `PRIMARY KEY (session_id, id)`

**Task dependencies:**
- `session_id TEXT NOT NULL`
- `task_id TEXT NOT NULL`
- `depends_on TEXT NOT NULL`
- `PRIMARY KEY (session_id, task_id, depends_on)`

**Session outcomes:**
- `session_id TEXT PRIMARY KEY`
- `underlying_goal TEXT`
- `outcome TEXT`
- `helpfulness TEXT`
- `session_type TEXT`
- `primary_success TEXT`
- `friction_detail TEXT`
- `brief_summary TEXT`

**Outcome categories:**
- `session_id TEXT NOT NULL REFERENCES session_outcomes(session_id)`
- `category TEXT NOT NULL`
- `count INTEGER DEFAULT 1`
- `PRIMARY KEY (session_id, category)`

**Outcome friction:**
- `session_id TEXT NOT NULL REFERENCES session_outcomes(session_id)`
- `friction_type TEXT NOT NULL`
- `count INTEGER DEFAULT 1`
- `PRIMARY KEY (session_id, friction_type)`

**Daily stats:**
- `date TEXT PRIMARY KEY`
- `message_count INTEGER`
- `session_count INTEGER`
- `tool_call_count INTEGER`

**Model usage:**
- `model TEXT PRIMARY KEY`
- `input_tokens INTEGER`
- `output_tokens INTEGER`
- `cache_read_tokens INTEGER`
- `cache_creation_tokens INTEGER`

**Indexed files (indexer state):**
- `file_path TEXT PRIMARY KEY`
- `mtime_ms INTEGER NOT NULL`
- `size_bytes INTEGER NOT NULL`
- `last_byte_offset INTEGER DEFAULT 0`
- `indexed_at TEXT NOT NULL`

**Indexes:**
- `CREATE INDEX idx_messages_session ON messages(session_id, timestamp)`
- `CREATE INDEX idx_messages_type ON messages(type)`
- `CREATE INDEX idx_tool_calls_session ON tool_calls(session_id)`
- `CREATE INDEX idx_tool_calls_name ON tool_calls(tool_name)`
- `CREATE INDEX idx_content_blocks_message ON content_blocks(message_id)`
- `CREATE INDEX idx_blob_refs_hash ON blob_references(hash)`
- `CREATE INDEX idx_blob_refs_message ON blob_references(message_id)`
- `CREATE INDEX idx_file_refs_path ON file_references(file_path)`
- `CREATE INDEX idx_file_refs_session ON file_references(session_id)`
- `CREATE INDEX idx_sessions_project ON sessions(project_slug)`
- `CREATE INDEX idx_sessions_created ON sessions(created_at)`

**User Story:**
> As a developer, all tables exist after migration and match the TECH-PLAN schema exactly.

**Test Plan:**
- [ ] Migration creates all 17 tables
- [ ] All indexes exist
- [ ] Foreign keys are enforced (`PRAGMA foreign_keys = ON`)
- [ ] Insert + select roundtrip for each table with sample data
- [ ] Schema matches TECH-PLAN.md specification

#### T1.2.3: Create V1 migration — FTS5 virtual table

**Requirements:**
- Create FTS5 virtual table for full-text search:
  ```sql
  CREATE VIRTUAL TABLE fts_content USING fts5(
      hash UNINDEXED,
      kind,
      content,
      tokenize='porter unicode61'
  );
  ```
- Test that FTS5 is available (bundled SQLite guarantees this)

**User Story:**
> As a user, I can perform full-text search queries against indexed content.

**Test Plan:**
- [ ] FTS5 table created successfully
- [ ] Insert text content, query with `MATCH`, results returned
- [ ] `snippet()` function returns highlighted context
- [ ] `bm25()` ranking works
- [ ] Porter stemming works: "running" matches "run"
- [ ] Unicode content is searchable

#### T1.2.4: Implement SQLite connection manager

**Requirements:**
- Open or create DB at `~/.blacklight/blacklight.db` (or configurable path)
- Set PRAGMAs on connection:
  - `journal_mode = WAL`
  - `synchronous = NORMAL`
  - `foreign_keys = ON`
  - `cache_size = -64000` (64MB)
  - `mmap_size = 268435456` (256MB)
- Run migration check on open
- Provide both sync (for indexer) and async-compatible (for web server) access
- Create parent directory if it doesn't exist

**User Story:**
> As the app, I get a properly configured SQLite connection with all optimizations applied.

**Test Plan:**
- [ ] DB file created at expected path
- [ ] Parent directory created if missing
- [ ] All PRAGMAs verified after connection
- [ ] WAL mode active (check `PRAGMA journal_mode`)
- [ ] Multiple concurrent readers work (WAL)
- [ ] Connection works from both sync and async contexts

---

## F1.3: Core Data Types

### Description
Define Rust structs and enums that represent the domain model.

### Tasks

#### T1.3.1: Define message types and enums

**Requirements:**
Rust types with `serde::Deserialize` for parsing JSONL:

```rust
enum MessageType { User, Assistant, Progress, System, Summary, FileHistorySnapshot, QueueOperation }
enum ContentBlockType { Text, ToolUse, ToolResult, Thinking }
enum StopReason { EndTurn, ToolUse }
enum TaskStatus { Pending, InProgress, Completed }
enum SessionOutcome { FullyAchieved, MostlyAchieved, PartiallyAchieved, NotAchieved }
enum Helpfulness { Essential, VeryHelpful, ModeratelyHelpful, SlightlyHelpful }
enum SessionType { SingleTask, MultiTask, IterativeRefinement, Exploration }
```

Structs for:
- `SessionMessage` — top-level JSONL line (with `type` discriminator)
- `AssistantMessage`, `UserMessage` — type-specific fields
- `ContentBlock` — union of text/tool_use/tool_result/thinking
- `ToolUseBlock` — tool name, id, input
- `ToolResultBlock` — tool_use_id, content
- `SessionIndex` — from `sessions-index.json`
- `SessionIndexEntry` — individual session metadata
- `TaskRecord` — from `tasks/*.json`
- `TodoItem` — from `todos/*.json`
- `SessionFacet` — from `usage-data/facets/*.json`
- `StatsCache` — from `stats-cache.json`

All types should handle missing/optional fields gracefully with `Option<T>` and
`#[serde(default)]`. Unknown fields should be ignored (`#[serde(deny_unknown_fields)]`
NOT used — schema may evolve).

**User Story:**
> As the indexer, I can deserialize any JSONL line into a typed Rust struct without panicking, even if the schema has new fields I don't know about.

**Test Plan:**
- [ ] Deserialize sample `user` message JSON → `SessionMessage::User`
- [ ] Deserialize sample `assistant` message with text + tool_use + thinking blocks
- [ ] Deserialize sample `progress` message (verify `normalizedMessages` field exists but can be skipped)
- [ ] Deserialize sample `system` message (both `turn_duration` and `local_command` subtypes)
- [ ] Deserialize with unknown fields → no error
- [ ] Deserialize with missing optional fields → defaults to `None`
- [ ] Deserialize `sessions-index.json` entry
- [ ] Deserialize `tasks/*.json` record
- [ ] Deserialize `usage-data/facets/*.json` record
- [ ] Deserialize `stats-cache.json`
- [ ] Roundtrip: deserialize → serialize → deserialize produces same result

---

## F1.4: Content Store Module

### Description
Implement the content-addressable blob store: hash, store, deduplicate, retrieve.

### Tasks

#### T1.4.1: Implement BLAKE3 hashing utility

**Requirements:**
- Function: `hash_content(content: &str) -> String` → 64-char hex BLAKE3 digest
- Function: `hash_content_bytes(content: &[u8]) -> String` → same for byte slices
- Threshold constant: `DEDUP_THRESHOLD = 256` bytes
- Function: `should_dedup(content: &str) -> bool` → true if len >= threshold

**User Story:**
> As the indexer, I can hash any text blob to a deterministic key for deduplication.

**Test Plan:**
- [ ] Same input always produces same hash
- [ ] Different input produces different hash
- [ ] Hash is exactly 64 hex characters
- [ ] Empty string hashes without error
- [ ] 1MB string hashes in < 10ms
- [ ] `should_dedup("")` → false
- [ ] `should_dedup("x".repeat(256))` → true
- [ ] `should_dedup("x".repeat(255))` → false

#### T1.4.2: Implement content_store DB operations

**Requirements:**
- `insert_blob(conn, hash, content, size, kind) -> Result<bool>` — returns true if new, false if already existed
- `get_blob(conn, hash) -> Result<Option<ContentBlob>>`
- `blob_exists(conn, hash) -> Result<bool>`
- `insert_blob_reference(conn, hash, message_id, context) -> Result<()>`
- `get_blob_references(conn, hash) -> Result<Vec<BlobReference>>`
- Use `INSERT OR IGNORE` for idempotent blob insertion
- Batch insert support: `insert_blobs_batch(conn, blobs: &[ContentBlob]) -> Result<usize>` — returns count of newly inserted

**User Story:**
> As the indexer, I can store content blobs with automatic deduplication — inserting the same content twice is a no-op.

**Test Plan:**
- [ ] Insert blob → retrieve by hash → content matches
- [ ] Insert same hash twice → no error, no duplicate, returns false on second insert
- [ ] Insert different content with different hash → both retrievable
- [ ] `blob_exists` returns true for inserted, false for unknown
- [ ] Batch insert of 1000 blobs completes in < 100ms
- [ ] Blob references correctly link hash → message_id → context
- [ ] Multiple references to same hash: all retrievable

#### T1.4.3: Implement FTS5 indexing operations

**Requirements:**
- `index_content(conn, hash, kind, content) -> Result<()>` — insert into FTS5
- `search(conn, query, limit, offset) -> Result<Vec<SearchResult>>` — BM25-ranked search
- `SearchResult` struct: `{ hash, kind, snippet, rank }`
- Use `snippet(fts_content, 2, '<mark>', '</mark>', '...', 64)` for context snippets
- Use `bm25(fts_content)` for ranking
- Skip FTS insert if hash already in FTS (dedup at FTS level too)
- Handle FTS5 query syntax errors gracefully (return user-friendly error, don't panic)

**User Story:**
> As a user, I can search for text across all indexed content and get ranked results with highlighted snippets.

**Test Plan:**
- [ ] Index 3 blobs with different content → search finds correct one
- [ ] BM25 ranking: more relevant result scores higher
- [ ] Snippet contains `<mark>` tags around matched terms
- [ ] Search for non-existent term → empty results
- [ ] Search with FTS5 syntax (quotes, AND, OR, NOT) works
- [ ] Invalid FTS5 syntax → graceful error, not panic
- [ ] Duplicate hash insertion → no FTS5 duplicate
- [ ] Porter stemming: search "running" matches content "run"
- [ ] Search across 10,000 blobs completes in < 100ms

---

---

# M2: EXCAVATION

> The indexer: crawl `~/.claude/`, parse all data sources, populate the database.

## F2.1: File Discovery & Manifest

### Description
Scan `~/.claude/` and build a manifest of all files to process, compared against
what's already been indexed.

### Tasks

#### T2.1.1: Implement directory scanner

**Requirements:**
- Recursively walk `~/.claude/` (configurable root path)
- Classify each file by type:
  - `SessionJsonl` — `projects/**/*.jsonl`
  - `SessionIndex` — `projects/**/sessions-index.json`
  - `TaskJson` — `tasks/**/*.json` (excluding `.lock`, `.highwatermark`)
  - `TodoJson` — `todos/**/*.json`
  - `FacetJson` — `usage-data/facets/**/*.json`
  - `StatsCache` — `stats-cache.json`
  - `HistoryJsonl` — `history.jsonl`
  - `PlanMarkdown` — `plans/**/*.md`
  - `DebugLog` — `debug/**/*.txt`
  - `FileHistorySnapshot` — `file-history/**/*`
  - `ToolResultTxt` — `projects/**/tool-results/toolu_*.txt`
  - `Skip` — everything else (`.lock`, `.highwatermark`, `.DS_Store`, `settings.json`, `statsig/`, `cache/`, `shell-snapshots/`, `session-env/`, `ide/`, `paste-cache/`)
- Collect `(path, file_type, mtime_ms, size_bytes)` for each file
- Sort by type then by path for deterministic processing order
- Log summary: file counts by type, total size

**User Story:**
> As the indexer, I know exactly which files need processing and what type each one is.

**Test Plan:**
- [ ] Scanner finds all JSONL files in projects/
- [ ] Scanner correctly classifies each file type
- [ ] `.lock` and `.highwatermark` files are skipped
- [ ] `.DS_Store` files are skipped
- [ ] Symlinks are followed (or explicitly not followed — document choice)
- [ ] Non-existent root path → clear error message
- [ ] Empty root path → zero files, no error
- [ ] Summary log shows correct counts
- [ ] Scan of real `~/.claude/` completes in < 1 second

#### T2.1.2: Implement change detection

**Requirements:**
- Compare scanned manifest against `indexed_files` table
- Classify each file as:
  - `New` — not in `indexed_files`
  - `Modified` — mtime or size changed
  - `Unchanged` — mtime and size match
  - `Deleted` — in `indexed_files` but not on disk
- For `Modified` JSONL files: note `last_byte_offset` for append detection
  (if size grew but mtime changed, assume append — seek to last offset)
- Return processing plan: list of files to (re)index, list to mark stale

**User Story:**
> As the indexer on a second run, I only process new and changed files, completing in seconds instead of minutes.

**Test Plan:**
- [ ] First run: all files classified as `New`
- [ ] Second run (no changes): all files classified as `Unchanged`
- [ ] Touch a file (update mtime): classified as `Modified`
- [ ] Delete a file: classified as `Deleted`
- [ ] Add a new file: classified as `New`
- [ ] JSONL file that grew: `last_byte_offset` carried forward from `indexed_files`
- [ ] Processing plan includes only `New` + `Modified` files

---

## F2.2: Session Metadata Parser

### Description
Parse `sessions-index.json` files to populate the `sessions` table.

### Tasks

#### T2.2.1: Parse sessions-index.json files

**Requirements:**
- Deserialize each `sessions-index.json` into `SessionIndex` struct
- Extract `project_path` from the index's `originalPath` field
- Derive `project_slug` from the last path component (e.g., `/home/user/projects/echo` → `echo`)
- For each entry: insert or update `sessions` table
- Handle missing/null fields gracefully (summary, gitBranch may be empty)
- Extract `claude_version` from session JSONL if not in index (defer to F2.3)

**User Story:**
> As a user, I can see all my sessions listed with project, date, summary, and message count — before the full conversation is even indexed.

**Test Plan:**
- [ ] Parse real `sessions-index.json` → correct session count
- [ ] `project_slug` correctly derived from path
- [ ] Sessions with empty summary → stored as NULL
- [ ] Sessions with empty git branch → stored as NULL
- [ ] Duplicate session ID → upsert (update, not duplicate)
- [ ] Timestamps parsed correctly (ISO 8601)
- [ ] `is_sidechain` flag preserved
- [ ] `source_file` points to correct JSONL path

---

## F2.3: JSONL Streaming Parser

### Description
The core indexer: stream session JSONL files line-by-line, extract messages, tool calls,
content blocks, and populate all related tables with deduplication.

### Tasks

#### T2.3.1: Implement line-by-line JSONL reader

**Requirements:**
- Open JSONL file with `BufReader` (8KB buffer minimum; consider 64KB for large files)
- Read one line at a time — never load entire file into memory
- Support seeking to byte offset (for incremental indexing of appended files)
- Parse each line as `serde_json::Value` first for type discrimination
- Track byte offset after each line for `indexed_files` update
- Handle malformed lines: log warning with file path + line number, skip line, continue
- Handle empty lines: skip silently
- Report progress: log every 1000 lines processed

**User Story:**
> As the indexer, I can process a 500MB JSONL file in constant memory, resuming from where I left off.

**Test Plan:**
- [ ] Read a real session JSONL file → all lines parsed
- [ ] Memory usage stays constant regardless of file size (test with 100MB+ file)
- [ ] Seek to byte offset → starts reading from correct line
- [ ] Malformed line → warning logged, processing continues
- [ ] Empty line → skipped silently
- [ ] File with 1MB+ lines → parsed without error
- [ ] Byte offset tracking is accurate (seek to offset → next line is correct)
- [ ] Progress logging at correct intervals

#### T2.3.2: Implement message type router

**Requirements:**
- Read `type` field from parsed JSON
- Route to appropriate handler:
  - `"assistant"` → `handle_assistant_message()`
  - `"user"` → `handle_user_message()`
  - `"progress"` → `handle_progress_message()` (metadata only)
  - `"system"` → `handle_system_message()`
  - `"summary"` → `handle_summary_message()`
  - `"file-history-snapshot"` → `handle_file_history()` (metadata only)
  - `"queue-operation"` → skip entirely
  - Unknown type → log warning, skip
- Each handler returns `Vec<DbOperation>` (inserts to batch)
- Batch DB writes every 500 messages (single transaction)

**User Story:**
> As the indexer, each message type is handled by the correct parser, and unknown types don't crash the system.

**Test Plan:**
- [ ] Each known type routes to correct handler
- [ ] Unknown type → warning logged, no crash
- [ ] Missing `type` field → warning logged, skip
- [ ] Batch transaction commits every 500 messages
- [ ] Transaction failure on one message → entire batch retried individually
- [ ] All 7 message types from real data parse successfully

#### T2.3.3: Implement assistant message handler

**Requirements:**
- Extract from assistant message:
  - `uuid` → `messages.id`
  - `parentUuid` → `messages.parent_id`
  - `timestamp` → `messages.timestamp`
  - `sessionId` → `messages.session_id`
  - `message.model` → `messages.model`
  - `message.stop_reason` → `messages.stop_reason`
  - `cwd`, `gitBranch`
- Iterate `message.content[]` array:
  - `type: "text"` → hash `text` field → `content_store` + `fts_content` + `content_blocks`
  - `type: "tool_use"` → serialize `input` to JSON string → hash → `content_store` + `content_blocks` + `tool_calls`
  - `type: "thinking"` → hash `thinking` field → `content_store` + `content_blocks` (NO FTS by default)
- For tool_use blocks: also insert into `tool_calls` denormalized table
- Apply dedup threshold: only hash+store blobs >= 256 bytes; inline smaller content

**User Story:**
> As the indexer, every assistant response is broken down into searchable text, tool calls, and thinking blocks — all deduplicated.

**Test Plan:**
- [ ] Text block extracted and searchable via FTS
- [ ] Tool_use block: tool name, input JSON stored
- [ ] Thinking block stored but NOT in FTS
- [ ] Content under 256 bytes stored inline (not in content_store)
- [ ] Content over 256 bytes stored in content_store with hash
- [ ] Duplicate content (same hash) → single entry in content_store
- [ ] `tool_calls` table populated with correct session_id, tool_name
- [ ] `content_blocks` has correct `block_index` ordering
- [ ] Message with no content blocks → message stored, zero content_blocks

#### T2.3.4: Implement user message handler

**Requirements:**
- Extract same metadata fields as assistant (uuid, parentUuid, timestamp, etc.)
- Also extract `permissionMode` if present
- Iterate `message.content[]`:
  - `type: "text"` → hash → `content_store` + `fts_content`
  - `type: "tool_result"` → extract `tool_use_id` to link back to tool_call; hash content → `content_store` + `fts_content`
- For tool_result blocks: update matching `tool_calls` row to set `output_hash`
- Detect file paths in tool results (Read, Write, Edit tool outputs contain file paths) → populate `file_references`

**User Story:**
> As the indexer, user messages and their tool results are indexed and linked back to the tool calls that produced them.

**Test Plan:**
- [ ] User text message indexed and searchable
- [ ] Tool result linked to correct tool_call via `tool_use_id`
- [ ] `tool_calls.output_hash` updated with result content hash
- [ ] File path detection: Read tool output → `file_references` entry with operation "read"
- [ ] Write tool result → `file_references` entry with operation "write"
- [ ] Edit tool result → `file_references` entry with operation "edit"
- [ ] Tool result content searchable via FTS

#### T2.3.5: Implement progress message handler (lightweight)

**Requirements:**
- Extract ONLY top-level metadata:
  - `timestamp`
  - `sessionId`
  - `data.type` (tool status type)
  - `data.tool` (tool name, if present)
- **DO NOT** parse or store `data.normalizedMessages` — this is the key optimization
- Use `serde_json::Value` and access fields by key to avoid deserializing the full object
- Optionally: use `serde::Deserializer::ignore` or `#[serde(skip)]` on `normalizedMessages`
- Do NOT insert into `messages` table (progress messages are ephemeral)

**User Story:**
> As the indexer, progress messages are processed in microseconds by skipping the 96% that is normalizedMessages.

**Test Plan:**
- [ ] 1.8MB progress line processed in < 1ms
- [ ] `normalizedMessages` field not allocated or stored
- [ ] Memory usage doesn't spike on progress messages
- [ ] Top-level metadata (timestamp, tool name) extractable
- [ ] No entry created in `messages` table

#### T2.3.6: Implement system/summary/file-history handlers

**Requirements:**

**System messages:**
- Extract `subType`: `"turn_duration"` or `"local_command"`
- For `turn_duration`: store `durationMs` in `messages.duration_ms`
- For `local_command`: store in messages table with content
- Insert into `messages` table with `type = "system"`

**Summary messages:**
- Store full content as-is in `messages` table with `type = "summary"`
- Index summary text in FTS

**File-history-snapshot:**
- Extract file paths from `trackedFileBackups` if non-empty
- Store metadata only (no full file content — that's in file-history/)
- Skip if `trackedFileBackups` is empty

**User Story:**
> As the indexer, session metadata like turn durations and summaries are captured for analytics.

**Test Plan:**
- [ ] `turn_duration` system message → `duration_ms` stored correctly
- [ ] `local_command` system message → content stored
- [ ] Summary message → searchable via FTS
- [ ] File-history-snapshot with empty backups → skipped
- [ ] File-history-snapshot with content → file paths extracted

---

## F2.4: Structured Data Parsers

### Description
Parse the smaller, structured data files: tasks, todos, facets, stats, plans.

### Tasks

#### T2.4.1: Parse tasks

**Requirements:**
- Walk `~/.claude/tasks/` directories
- Each UUID directory = a task session
- Each numbered JSON file = one task
- Skip `.lock` and `.highwatermark` files
- Deserialize into `TaskRecord` struct
- Populate `tasks` table and `task_dependencies` table
- Handle `blocks` and `blockedBy` arrays → `task_dependencies` rows

**User Story:**
> As a user, I can see all tasks across all sessions with their statuses and dependencies.

**Test Plan:**
- [ ] All task JSON files parsed successfully
- [ ] Task dependencies correctly populated (both blocks and blockedBy)
- [ ] `.lock` files skipped
- [ ] Invalid JSON → warning, skip file, continue
- [ ] Empty blocks/blockedBy arrays → no dependency rows

#### T2.4.2: Parse session outcomes (facets)

**Requirements:**
- Read all JSON files in `~/.claude/usage-data/facets/`
- Deserialize into `SessionFacet` struct
- Populate `session_outcomes` table
- Populate `outcome_categories` table (from `goal_categories` map)
- Populate `outcome_friction` table (from `friction_counts` map)
- Link to sessions table via `session_id`

**User Story:**
> As a user, I can see how Claude performed across sessions — success rates, friction types, helpfulness ratings.

**Test Plan:**
- [ ] All facet files parsed
- [ ] `outcome` enum values match expected set
- [ ] `goal_categories` map → individual rows in `outcome_categories`
- [ ] `friction_counts` map → individual rows in `outcome_friction`
- [ ] Empty `friction_counts` → no friction rows (valid)
- [ ] Session ID matches filename (without .json extension)

#### T2.4.3: Parse stats cache

**Requirements:**
- Read `~/.claude/stats-cache.json`
- Deserialize into `StatsCache` struct
- Populate `daily_stats` table from `dailyActivity` array
- Populate `model_usage` table from `modelUsage` map
- Store aggregate fields (totalSessions, totalMessages) as a special row or in a metadata table

**User Story:**
> As a user, I can see daily activity trends and model usage breakdowns from pre-computed stats.

**Test Plan:**
- [ ] All daily entries populated
- [ ] All model entries populated
- [ ] Token counts stored correctly (including large values like 3.7B cache_read_tokens)
- [ ] Integer overflow handled (use i64, not i32)
- [ ] Missing days → no row (not zero-filled)

#### T2.4.4: Parse plans

**Requirements:**
- Read all `.md` files in `~/.claude/plans/`
- Store full markdown content in `content_store` (treated as a blob, kind = "plan")
- Index in FTS5 for searchability
- Store metadata: filename (the whimsical name), file size, mtime

**User Story:**
> As a user, I can search for content in Claude's plans and browse them.

**Test Plan:**
- [ ] All plan markdown files indexed
- [ ] Search for text within a plan → found via FTS
- [ ] Plan filenames preserved (e.g., "calm-snuggling-sonnet.md")
- [ ] Large plans (19KB) stored and retrievable

#### T2.4.5: Parse history.jsonl

**Requirements:**
- Stream `~/.claude/history.jsonl` line by line
- Each line: `{ display, timestamp, project, sessionId, pastedContents }`
- Cross-reference with sessions: verify session IDs exist
- Store as a lightweight index (don't duplicate what's in sessions)
- Useful for: prompt-level search, finding sessions by what was asked

**User Story:**
> As a user, I can search by what I asked Claude, not just what Claude said.

**Test Plan:**
- [ ] All 3,383 records parsed
- [ ] Timestamps stored correctly
- [ ] Project paths linked to sessions
- [ ] Search for a prompt text → found
- [ ] `pastedContents` with `contentHash` → reference stored (not full content)

---

## F2.5: Incremental Indexing

### Description
After first full index, subsequent runs only process new/changed files.

### Tasks

#### T2.5.1: Implement indexed_files tracking

**Requirements:**
- After processing each file: insert/update `indexed_files` with path, mtime, size, byte_offset, timestamp
- For JSONL files: store `last_byte_offset` = position after last processed line
- Use this on subsequent runs to detect changes (F2.1.2) and resume (F2.3.1)
- Transaction-safe: update `indexed_files` only after successful processing of that file

**User Story:**
> As a user, running `blacklight index` a second time completes in seconds, not minutes.

**Test Plan:**
- [ ] First index: all files recorded in `indexed_files`
- [ ] Second index (no changes): zero files processed, completes in < 2 seconds
- [ ] New session JSONL added → only that file processed
- [ ] JSONL file grew (appended) → only new lines processed, starting from saved offset
- [ ] Processing failure → `indexed_files` not updated for that file (retry on next run)

#### T2.5.2: Implement stale data cleanup

**Requirements:**
- Files in `indexed_files` that no longer exist on disk → mark as stale
- Option 1 (default): leave indexed data in DB (it's still valid history)
- Option 2 (`--clean`): remove messages/content from deleted sessions
- Never auto-delete content_store blobs (may be referenced by other sessions)
- Log stale files found

**User Story:**
> As a user, if I delete old session files, the index gracefully handles it.

**Test Plan:**
- [ ] Deleted file detected as stale
- [ ] Default: data preserved in DB
- [ ] `--clean` flag: associated messages removed
- [ ] content_store blobs not orphaned (reference counting or lazy cleanup)

---

## F2.6: Full Indexer Integration

### Description
Wire all parsers together into the `blacklight index` command.

### Tasks

#### T2.6.1: Implement index orchestrator

**Requirements:**
- Orchestrate the full pipeline: discovery → metadata → conversations → structured data
- Wrap entire index run in progress reporting:
  - "Scanning ~/.claude/..."
  - "Found N files (X new, Y modified, Z unchanged)"
  - "Indexing sessions... [progress bar or count]"
  - "Indexing conversations... [N/M files, L lines processed]"
  - "Indexing tasks, plans, stats..."
  - "Done. Indexed N sessions, M messages, K tool calls, B unique blobs. DB size: X MB."
- Total wall-clock time reported at end
- Support `--source PATH` to index a non-default directory
- Support `--full` to force re-index everything
- Support `--verbose` for detailed per-file logging

**User Story:**
> As a user, I run `blacklight index` and see clear progress as my 4.5GB of data gets indexed.

**Test Plan:**
- [ ] Full index of real `~/.claude/` completes successfully
- [ ] All tables populated with correct data
- [ ] Progress output shows meaningful counts
- [ ] `--full` re-indexes everything (ignores indexed_files)
- [ ] `--source /tmp/test-claude` indexes alternate directory
- [ ] Exit code 0 on success, non-zero on failure
- [ ] Total time logged
- [ ] DB file size is < 1GB (dedup working)

---

---

# M3: LENS

> CLI tools: quick terminal access to search and stats.

## F3.1: CLI Framework

### Description
Set up the `clap` CLI with subcommands.

### Tasks

#### T3.1.1: Implement clap CLI structure

**Requirements:**
- Top-level binary: `blacklight`
- Subcommands:
  - `index` — with `--full`, `--source PATH`, `--verbose`
  - `serve` — with `--port PORT`, `--no-open`
  - `search` — with positional QUERY, `--project`, `--kind`, `--limit`, `--from`, `--to`
  - `stats` — with `--daily`, `--models`, `--projects`
- Global options:
  - `--db PATH` — custom database path
  - `--claude-dir PATH` — custom `~/.claude/` path
- Help text for all commands and options
- Version flag (`--version`)

**User Story:**
> As a user, `blacklight --help` shows me all available commands with clear descriptions.

**Test Plan:**
- [ ] `blacklight --help` shows all subcommands
- [ ] `blacklight index --help` shows index options
- [ ] `blacklight search --help` shows search options
- [ ] Unknown subcommand → clear error message
- [ ] Missing required args → clear error message
- [ ] `--version` shows version

---

## F3.2: CLI Search

### Description
Full-text search from the terminal.

### Tasks

#### T3.2.1: Implement search command

**Requirements:**
- Accept search query as positional argument
- Query FTS5 index with BM25 ranking
- Join results back to messages → sessions for context
- Display results as:
  ```
  [1] session:echo/abc123 (2026-02-07 15:30)
      ...matched text with <highlighted> terms...
      Tool: Read /src/engine_impl.rs

  [2] session:git-stunts/def456 (2026-02-05 10:15)
      ...another matched snippet...
  ```
- Support filters:
  - `--project NAME` → filter by project_slug
  - `--kind KIND` → filter by content kind (text, tool_output, thinking, plan)
  - `--from DATE` → filter by date range start
  - `--to DATE` → filter by date range end
  - `--limit N` → max results (default 10)
- Color output (if terminal supports it)
- Machine-readable output with `--json` flag

**User Story:**
> As a user, I can quickly search "auth bug" from my terminal and find the session where I fixed it.

**Test Plan:**
- [ ] Basic search returns ranked results
- [ ] Snippets show context around matched terms
- [ ] `--project echo` filters to echo sessions only
- [ ] `--kind tool_output` filters to tool outputs only
- [ ] `--limit 5` returns at most 5 results
- [ ] `--json` outputs valid JSON array
- [ ] No results → "No results found." message
- [ ] Search with special characters doesn't crash
- [ ] Results include session ID, project, timestamp

---

## F3.3: CLI Stats

### Description
Quick statistics from the terminal.

### Tasks

#### T3.3.1: Implement stats command

**Requirements:**
- Default (no flags): overview summary
  ```
  Claude Code Usage Summary
  ─────────────────────────
  Sessions:     363
  Messages:     169,701
  Tool Calls:   24,589
  Unique Blobs: 8,432
  DB Size:      742 MB
  Source Size:   4.5 GB
  Dedup Ratio:  83.5%
  Date Range:   2026-01-11 → 2026-02-10
  ```
- `--daily`: table of daily activity (date, messages, sessions, tool calls)
- `--models`: per-model token breakdown (model, input, output, cache_read)
- `--projects`: per-project session/message counts
- Formatted tables with alignment
- Color coding for emphasis

**User Story:**
> As a user, `blacklight stats` gives me an instant overview of my Claude Code usage.

**Test Plan:**
- [ ] Default output shows all overview fields
- [ ] `--daily` shows one row per active day
- [ ] `--models` shows all models with token counts
- [ ] `--projects` shows all projects sorted by message count
- [ ] Numbers formatted with commas (169,701 not 169701)
- [ ] Date range reflects actual data
- [ ] Dedup ratio calculated correctly (1 - db_size/source_size)

---

---

# M4: SIGNAL

> Web server and REST API.

## F4.1: Web Server Scaffold

### Description
Set up the axum web server with middleware and static file serving.

### Tasks

#### T4.1.1: Implement axum server with middleware

**Requirements:**
- Start HTTP server on configurable port (default 3141)
- Middleware stack:
  - `tower_http::cors` — allow localhost origins
  - `tower_http::compression` — gzip responses
  - Request logging via `tracing`
- Graceful shutdown on SIGINT/SIGTERM
- Open browser on startup (unless `--no-open`)
- Serve embedded static assets (frontend) at `/`
- API routes at `/api/*`
- SPA fallback: any non-`/api` route returns `index.html`
- Share SQLite connection pool via axum state

**User Story:**
> As a user, `blacklight serve` starts a local web server and opens my browser.

**Test Plan:**
- [ ] Server starts on specified port
- [ ] `GET /` returns HTML (index.html)
- [ ] `GET /api/status` returns JSON
- [ ] CORS headers present
- [ ] Gzip compression active for JSON responses
- [ ] SIGINT shuts down cleanly
- [ ] Port conflict → clear error message

---

## F4.2: Session & Message API

### Tasks

#### T4.2.1: Implement session endpoints

**Requirements:**
- `GET /api/sessions` — list sessions with pagination
  - Query params: `project`, `branch`, `from`, `to`, `limit` (default 50), `offset`
  - Response: `{ sessions: [...], total: N }`
  - Each session: id, project_slug, first_prompt (truncated), summary, message_count, created_at, modified_at, git_branch, model, outcome (if facet exists)
  - Default sort: `created_at DESC`
- `GET /api/sessions/:id` — single session with full details
  - Include session_outcome if exists
  - Include tool_call summary (count by tool_name)
- `GET /api/sessions/:id/messages` — messages for a session
  - Query params: `type` (comma-separated filter), `limit`, `offset`
  - Include content blocks (text content resolved from content_store)
  - Include tool call details (inputs, outputs resolved)
  - Ordered by timestamp ASC
- `GET /api/sessions/:id/tool-calls` — tool calls for a session
  - Include input and output content (resolved from content_store)
  - Ordered by timestamp ASC

**User Story:**
> As the frontend, I can list sessions, drill into one, and get the full conversation with all tool calls.

**Test Plan:**
- [ ] List sessions returns correct count and pagination
- [ ] Filter by project works
- [ ] Filter by date range works
- [ ] Session detail includes outcome data
- [ ] Messages returned in timestamp order
- [ ] Content blocks include resolved text (not just hashes)
- [ ] Tool calls include resolved inputs/outputs
- [ ] Non-existent session ID → 404
- [ ] Large session (1000+ messages) paginates correctly

---

## F4.3: Search API

### Tasks

#### T4.3.1: Implement search endpoint

**Requirements:**
- `GET /api/search` — full-text search
  - Query params: `q` (required), `project`, `kind`, `from`, `to`, `limit` (default 20), `offset`
  - Response: `{ results: [...], total: N, query: "..." }`
  - Each result: hash, kind, snippet (HTML with `<mark>` tags), rank, session_id, session_project, timestamp, message_type
  - Join through `blob_references` → `messages` → `sessions` to provide context
  - BM25 ranking
- Handle empty query → 400 error
- Handle FTS5 syntax errors → 400 with user-friendly message
- Escape user input to prevent FTS5 injection (or use parameterized queries)

**User Story:**
> As a user, I can search for any text and get ranked results with context showing which session and message the match came from.

**Test Plan:**
- [ ] Search returns ranked results with snippets
- [ ] Results include session context (project, timestamp)
- [ ] Filter by project narrows results
- [ ] Filter by kind narrows results
- [ ] Empty query → 400
- [ ] No results → empty array, 200
- [ ] FTS5 special characters handled safely
- [ ] Pagination works (offset + limit)

---

## F4.4: Analytics API

### Tasks

#### T4.4.1: Implement analytics endpoints

**Requirements:**
- `GET /api/stats/overview` — aggregate stats
  - Total sessions, messages, tool calls, unique blobs, DB size, source size
  - Date range (earliest → latest session)
  - Dedup ratio
- `GET /api/stats/daily` — daily timeseries
  - Array of `{ date, messageCount, sessionCount, toolCallCount }`
  - Query params: `from`, `to`
- `GET /api/stats/hourly` — hourly distribution
  - Array of `{ hour: 0-23, count }`
- `GET /api/stats/models` — per-model usage
  - Array of `{ model, inputTokens, outputTokens, cacheReadTokens, cacheCreationTokens }`
- `GET /api/stats/tools` — tool usage frequency
  - Query `tool_calls` table: `GROUP BY tool_name, COUNT(*)`
  - Array of `{ tool, count }` sorted by count DESC
- `GET /api/stats/outcomes` — session outcome distribution
  - Query `session_outcomes`: counts by outcome, helpfulness, session_type
  - Friction type distribution from `outcome_friction`

**User Story:**
> As the frontend, I can fetch all the data needed for charts and dashboards.

**Test Plan:**
- [ ] Overview returns all aggregate fields
- [ ] Daily timeseries has one entry per active day
- [ ] Hourly distribution has entries for hours 0-23
- [ ] Model usage includes all models with correct token counts
- [ ] Tool usage shows all tools sorted by frequency
- [ ] Outcome distribution has correct counts
- [ ] Date range filter works on daily endpoint

---

## F4.5: Storage & Files API

### Tasks

#### T4.5.1: Implement storage and file endpoints

**Requirements:**
- `GET /api/storage` — disk usage analysis
  - Per-project: raw JSONL size, indexed data size, session count
  - Total raw vs DB size, dedup ratio
  - Largest sessions by JSONL file size
- `GET /api/storage/blobs` — content store stats
  - Total blobs, total size, average size
  - Top 20 largest blobs with kind and reference count
- `GET /api/files` — files Claude has touched
  - Query `file_references`: group by file_path, count distinct sessions
  - Sort by frequency DESC
  - Query params: `project`, `limit`, `offset`
- `GET /api/files/:path/sessions` — sessions that touched a file
  - `:path` is URL-encoded file path
  - Include operation type (read, write, edit)
- `GET /api/content/:hash` — fetch raw blob content
  - Return content with appropriate content-type (text/plain)
  - 404 if hash not found

**User Story:**
> As a user, I can see where disk space is going and which files Claude has touched most.

**Test Plan:**
- [ ] Storage shows per-project breakdown
- [ ] File list ranked by touch frequency
- [ ] File sessions shows all sessions with operation types
- [ ] Content endpoint returns blob text
- [ ] URL-encoded file paths handled correctly
- [ ] Non-existent hash → 404

---

## F4.6: Tasks & Plans API

### Tasks

#### T4.6.1: Implement tasks and plans endpoints

**Requirements:**
- `GET /api/tasks` — all tasks
  - Query params: `status`, `session`
  - Include dependencies (blocks, blockedBy)
  - Sort by session, then id
- `GET /api/plans` — list plan files
  - Filename, size, mtime
  - Sort by mtime DESC
- `GET /api/plans/:name` — single plan content
  - Return rendered markdown (HTML) or raw markdown (with Accept header)
  - `:name` is the plan filename (e.g., "calm-snuggling-sonnet")

**User Story:**
> As a user, I can browse Claude's plans and tasks from the web UI.

**Test Plan:**
- [ ] Tasks list includes dependencies
- [ ] Filter by status works
- [ ] Plans list shows all plan files
- [ ] Plan content returned as markdown or HTML
- [ ] Non-existent plan → 404

---

## F4.7: System API

### Tasks

#### T4.7.1: Implement system endpoints

**Requirements:**
- `POST /api/reindex` — trigger re-index
  - Run indexer in background (don't block the HTTP response)
  - Return `{ status: "started", jobId: "..." }`
  - Support `GET /api/reindex/:jobId` to check progress
- `GET /api/status` — index health
  - Last indexed timestamp
  - File counts by type
  - DB size
  - Any stale files detected

**User Story:**
> As a user, I can trigger a re-index from the web UI and see index health.

**Test Plan:**
- [ ] Reindex starts without blocking response
- [ ] Job status checkable via GET
- [ ] Status endpoint returns current index health
- [ ] Concurrent reindex requests → only one runs

---

---

# M5: MIRROR

> Frontend: dashboard, session explorer, search.

## F5.1: Svelte SPA Scaffold

### Description
Set up the Svelte project with routing, layout, and build pipeline.

### Tasks

#### T5.1.1: Initialize Svelte project

**Requirements:**
- Vite + Svelte project in `frontend/` directory
- Client-side routing (svelte-routing or custom hash router)
- Layout: sidebar nav + main content area
- Routes:
  - `/` — Dashboard
  - `/sessions` — Session Explorer
  - `/sessions/:id` — Conversation Replay
  - `/search` — Search
  - `/analytics` — Analytics
  - `/storage` — Storage
  - `/files` — File Impact
  - `/plans` — Plans & Tasks
- Dark mode by default (CSS custom properties for theming)
- API client module: typed fetch wrappers for all `/api/*` endpoints
- Loading states and error handling for all API calls

**User Story:**
> As a user, I see a clean, navigable app with consistent layout across all views.

**Test Plan:**
- [ ] All routes render correct view
- [ ] Navigation between routes works
- [ ] Unknown route → 404 page or redirect to /
- [ ] API client handles errors gracefully (shows error message, not blank page)
- [ ] Dark mode renders correctly
- [ ] Responsive layout works at common widths (1280, 1440, 1920)

#### T5.1.2: Implement asset embedding

**Requirements:**
- `vite build` outputs to `frontend/dist/`
- Rust binary embeds `frontend/dist/` via `rust-embed`
- axum serves embedded assets at `/`
- SPA fallback: any non-`/api` GET returns embedded `index.html`
- Cache headers: immutable for hashed assets, no-cache for `index.html`

**User Story:**
> As a user, the entire app is one binary — no separate frontend server needed.

**Test Plan:**
- [ ] `cargo build --release` produces single binary with embedded frontend
- [ ] Binary serves frontend without any external files
- [ ] Hashed asset URLs work (e.g., `/assets/app-abc123.js`)
- [ ] Deep link to `/sessions/abc` returns index.html (SPA routing)

---

## F5.2: Home Dashboard

### Tasks

#### T5.2.1: Implement dashboard view

**Requirements:**
- **Activity heatmap:** GitHub-style calendar grid, colored by message count per day
  - Library: `cal-heatmap` or custom SVG
  - Data source: `GET /api/stats/daily`
  - Color scale: white → light → dark (4-5 stops)
  - Tooltip on hover: date + message count
- **Stat cards:** sessions, messages, tool calls, dedup ratio, DB size
  - Data source: `GET /api/stats/overview`
  - Large number with label, formatted with commas
- **Hourly distribution:** bar chart (24 bars)
  - Data source: `GET /api/stats/hourly`
  - Library: Chart.js
- **Model usage:** donut chart
  - Data source: `GET /api/stats/models`
  - Segments: one per model, sized by total tokens
- **Recent sessions:** list of 10 most recent
  - Data source: `GET /api/sessions?limit=10`
  - Each: first prompt (truncated), project badge, timestamp, message count
  - Click → navigates to session replay
- **Top projects:** bar chart or list
  - Data source: `GET /api/sessions` grouped by project (or new endpoint)
  - Ranked by message count

**User Story:**
> As a user, the dashboard gives me an instant overview of my Claude Code usage patterns.

**Test Plan:**
- [ ] Heatmap renders with correct colors for date range
- [ ] Stat cards show correct numbers
- [ ] Hourly chart shows 24 bars
- [ ] Model chart shows all models
- [ ] Recent sessions are clickable
- [ ] All data loads without errors
- [ ] Loading states shown while data fetches

---

## F5.3: Session Explorer

### Tasks

#### T5.3.1: Implement session list view

**Requirements:**
- Filterable, sortable, paginated session list
- **Filters:**
  - Project dropdown (populated from distinct projects)
  - Date range picker (from/to)
  - Git branch text input
  - Outcome dropdown (fully_achieved, mostly_achieved, etc.)
  - Model dropdown
- **Sort options:** date (default), message count, duration
- **List items:** card or row showing:
  - Project badge (colored)
  - First prompt (truncated to ~100 chars)
  - Summary (if available)
  - Message count, tool call count
  - Timestamp (relative: "2 days ago" + absolute on hover)
  - Outcome badge (green/yellow/orange/red)
  - Model badge
- **Pagination:** "Load more" or page numbers
- **Search within sessions:** text filter on first_prompt/summary
- Click → navigate to `/sessions/:id` (Conversation Replay)

**User Story:**
> As a user, I can find any session by filtering and searching, and click into it for the full conversation.

**Test Plan:**
- [ ] All sessions displayed with correct metadata
- [ ] Project filter narrows list
- [ ] Date range filter works
- [ ] Sort by message count shows busiest sessions first
- [ ] Pagination loads more sessions
- [ ] Click navigates to replay view
- [ ] Empty state (no matching sessions) shows helpful message
- [ ] Filters are preserved in URL (shareable/bookmarkable)

---

## F5.4: Search View

### Tasks

#### T5.4.1: Implement search view

**Requirements:**
- Search bar (prominent, top of page)
- Debounced search (300ms after typing stops)
- **Filters** (collapsible panel):
  - Project dropdown
  - Content kind: text, tool_output, thinking, plan (checkboxes)
  - Date range
- **Results list:**
  - Ranked by relevance (BM25)
  - Each result: snippet with highlighted terms, session info (project, date), content kind badge
  - Click → navigate to session replay, scrolled to the matched message
- **Result count:** "N results for 'query'"
- **Empty state:** "No results found. Try a different query."
- **Error state:** "Invalid search syntax. Try simpler terms."
- Support FTS5 syntax hints (quotes for exact phrase, - for exclude)

**User Story:**
> As a user, I can search for anything I've ever discussed with Claude and quickly find the relevant session.

**Test Plan:**
- [ ] Typing triggers debounced search
- [ ] Results highlighted correctly
- [ ] Clicking result navigates to correct session + message
- [ ] Filters narrow results
- [ ] FTS5 syntax works (quoted phrases, NOT)
- [ ] Invalid syntax shows friendly error
- [ ] No results → helpful empty state
- [ ] Search preserves query in URL

---

---

# M6: REPLAY

> Conversation replay — the killer feature.

## F6.1: Conversation Thread Renderer

### Tasks

#### T6.1.1: Implement conversation replay view

**Requirements:**
- Load all messages for a session via `GET /api/sessions/:id/messages`
- Render as a threaded conversation:
  - **User messages:** left-aligned or full-width, distinct background
  - **Assistant messages:** full-width, different background
  - **System messages:** subtle, centered, smaller text
- Markdown rendering for message text (using `marked`)
- Syntax highlighting for code blocks (using `Prism`)
- Message metadata: timestamp, model (for assistant)
- Smooth scrolling, keyboard navigation (up/down between messages)
- URL updates as you scroll (deep-linking to specific messages)

**User Story:**
> As a user, I can replay any conversation with Claude exactly as it happened, with rendered markdown and syntax highlighting.

**Test Plan:**
- [ ] All message types rendered correctly
- [ ] Markdown rendered (headers, lists, code blocks, links)
- [ ] Code blocks syntax-highlighted for common languages (JS, Rust, Python, SQL)
- [ ] Long conversations scroll smoothly
- [ ] Deep link to specific message works
- [ ] User vs assistant messages visually distinct
- [ ] Empty messages handled gracefully

---

## F6.2: Tool Call Cards

### Tasks

#### T6.2.1: Implement collapsible tool call cards

**Requirements:**
- Tool_use blocks rendered as expandable cards within assistant messages:
  - **Collapsed:** tool icon + tool name + one-line summary of input
  - **Expanded:** full input JSON (formatted) + output content (with syntax highlighting if code)
- Tool_result blocks (from user messages) linked to their tool_use cards
- Tool-specific rendering:
  - **Read:** show file path, output as syntax-highlighted code
  - **Write:** show file path, content as syntax-highlighted code
  - **Edit:** show file path, old_string → new_string as a diff
  - **Bash:** show command, output as terminal-style monospace
  - **Grep:** show pattern + results
  - **Glob:** show pattern + matched files
  - **Task:** show description, subagent type
- Default state: collapsed (expandable on click)
- "Expand all" / "Collapse all" toggle

**User Story:**
> As a user, I can see exactly what tools Claude used, what inputs it provided, and what outputs it received — with appropriate formatting for each tool type.

**Test Plan:**
- [ ] Each tool type renders with appropriate formatting
- [ ] Collapse/expand works
- [ ] Edit tool shows diff rendering
- [ ] Bash tool shows terminal-style output
- [ ] Read tool shows syntax-highlighted file content
- [ ] Tool output correctly linked to tool input
- [ ] Large tool outputs don't crash the page (virtualize or truncate)
- [ ] "Expand all" toggles all cards

---

## F6.3: Thinking Block Toggle

### Tasks

#### T6.3.1: Implement thinking block viewer

**Requirements:**
- Thinking blocks within assistant messages shown as a toggleable section
- Default: hidden (collapsed)
- Toggle button: "Show thinking" / "Hide thinking"
- When expanded: rendered in a distinct style (muted text, different background, monospace)
- Global toggle in session sidebar: "Show all thinking" / "Hide all thinking"
- Thinking content rendered as plain text (not markdown)

**User Story:**
> As a user, I can optionally see Claude's internal reasoning for any response.

**Test Plan:**
- [ ] Thinking blocks hidden by default
- [ ] Toggle shows/hides thinking
- [ ] Global toggle affects all messages
- [ ] Thinking text rendered as monospace
- [ ] Large thinking blocks (20KB+) don't crash the page
- [ ] Messages without thinking blocks show no toggle

---

## F6.4: Diff Viewer

### Tasks

#### T6.4.1: Implement inline diff rendering

**Requirements:**
- For Edit tool calls: render `old_string` → `new_string` as a unified diff
- Use `diff2html` library for rendering
- Side-by-side or inline mode (user toggle)
- Syntax highlighting within diffs
- For Write tool calls: show full file content (no diff, just "new file")
- For file-history versions: diff between consecutive versions

**User Story:**
> As a user, I can see exactly what Claude changed in each file edit, rendered as a familiar diff view.

**Test Plan:**
- [ ] Edit tool shows correct diff
- [ ] Added lines highlighted green, removed lines red
- [ ] Syntax highlighting works within diff
- [ ] Side-by-side toggle works
- [ ] Write tool shows "new file" content
- [ ] Large diffs scrollable without performance issues

---

## F6.5: Session Metadata Sidebar

### Tasks

#### T6.5.1: Implement session sidebar

**Requirements:**
- Fixed sidebar (right side or collapsible) showing:
  - Project name + path
  - Git branch
  - Model used
  - Session date + duration
  - Message count, tool call count
  - Outcome badge (if facet exists)
  - Friction summary (if any)
- **Message navigator:** scrollable list of messages (mini-map)
  - Each entry: type icon + truncated first line
  - Click → scroll to that message
  - Current message highlighted
- **Files touched:** list of files referenced in this session
  - Click → navigate to file impact view
- **Tools used:** summary of tool usage (count by type)

**User Story:**
> As a user, I have full context about the session and can quickly navigate to any message.

**Test Plan:**
- [ ] All metadata fields displayed correctly
- [ ] Message navigator scrolls to correct message
- [ ] Current message tracked as user scrolls
- [ ] Files touched list populated from file_references
- [ ] Tool usage summary correct
- [ ] Sidebar collapsible on smaller screens

---

---

# M7: SPECTRUM

> Analytics, storage analysis, file impact, plans & tasks.

## F7.1: Analytics Dashboard

### Tasks

#### T7.1.1: Implement analytics view

**Requirements:**
- **Token usage over time:** stacked area chart by model
  - X-axis: date, Y-axis: tokens
  - One series per model, stacked
  - Library: Chart.js
- **Cost estimates:** apply published per-token pricing
  - Show estimated cost per day and cumulative
  - Note: cost is estimated (no actual billing data)
- **Tool usage breakdown:** horizontal bar chart
  - Tool name → count, sorted by frequency
- **Session outcomes:** stacked bar chart
  - Bars: fully_achieved, mostly_achieved, partially_achieved, not_achieved
  - Grouped by week or month
- **Friction analysis:** bar chart of friction types
  - buggy_code, wrong_approach, excessive_changes, etc.
  - Count per type
- **Per-project breakdown:** dropdown to filter all charts by project

**User Story:**
> As a user, I can see trends in my Claude usage — what models I use, what tools are most popular, where Claude struggles.

**Test Plan:**
- [ ] Token chart shows correct stacked data
- [ ] Cost estimates roughly match expectations
- [ ] Tool chart shows all tools
- [ ] Outcome chart shows correct distribution
- [ ] Friction chart populated from facets data
- [ ] Project filter narrows all charts
- [ ] Charts resize responsively

---

## F7.2: Storage Analysis View

### Tasks

#### T7.2.1: Implement storage view

**Requirements:**
- **Treemap:** disk usage by project
  - Library: D3 treemap
  - Size = raw JSONL file size per project
  - Color = dedup ratio (redder = more redundancy)
  - Click project → drilldown to sessions
- **Stats cards:** total raw size, DB size, dedup ratio, blob count
- **Largest sessions:** table of top 20 sessions by JSONL size
  - Columns: session ID, project, date, JSONL size, message count
- **Largest blobs:** table of top 20 blobs in content_store
  - Columns: hash (truncated), kind, size, reference count

**User Story:**
> As a user, I can see where my disk space is going and identify the most redundant data.

**Test Plan:**
- [ ] Treemap renders all projects with correct sizes
- [ ] Color scale reflects redundancy
- [ ] Clicking project shows its sessions
- [ ] Stats cards show correct numbers
- [ ] Largest sessions table populated
- [ ] Largest blobs table shows reference counts

---

## F7.3: File Impact View

### Tasks

#### T7.3.1: Implement file impact view

**Requirements:**
- **File list:** all files Claude has touched, ranked by frequency
  - Columns: file path, session count, operation types, last touched
  - Searchable/filterable
  - Click → file detail view
- **File detail:** for a specific file
  - List of sessions that touched it (with operation type)
  - File-history timeline: version snapshots over time
  - Diff viewer between consecutive versions (reuse F6.4)
  - Click session → navigate to replay view

**User Story:**
> As a user, I can see which files Claude has modified most and review the change history.

**Test Plan:**
- [ ] File list shows correct frequencies
- [ ] Search within file list works
- [ ] File detail shows all sessions
- [ ] Version timeline renders correctly
- [ ] Diff between versions works
- [ ] Click session navigates to replay

---

## F7.4: Plans & Tasks View

### Tasks

#### T7.4.1: Implement plans & tasks view

**Requirements:**
- **Plans tab:**
  - List of plan files with whimsical names
  - Sort by date modified
  - Click → rendered markdown view (using `marked`)
  - Searchable within plans
- **Tasks tab:**
  - Task board grouped by session
  - Status columns: pending, in_progress, completed
  - Dependency lines between tasks (if blocks/blockedBy)
  - Filter by status
  - Expandable task cards showing full description

**User Story:**
> As a user, I can browse Claude's plans and see task status across sessions.

**Test Plan:**
- [ ] Plans list shows all plan files
- [ ] Plan content rendered as HTML from markdown
- [ ] Tasks grouped by session correctly
- [ ] Status filter works
- [ ] Dependencies shown between related tasks
- [ ] Task descriptions expandable

---

---

## Milestone Acceptance Criteria

### FOUNDATION — Complete When:
- [ ] `cargo build` produces a binary
- [ ] SQLite DB created with all 17 tables + FTS5 + indexes
- [ ] Content store hashes, stores, deduplicates, and retrieves blobs
- [ ] All core Rust types compile and deserialize sample data

### EXCAVATION — Complete When:
- [ ] `blacklight index` processes real `~/.claude/` data end-to-end
- [ ] All sessions, messages, tool calls, tasks, facets, stats, plans indexed
- [ ] FTS5 search returns results
- [ ] Incremental indexing works (second run completes in < 5 seconds)
- [ ] DB size < 1GB (dedup working)
- [ ] Progress messages skipped (normalizedMessages not stored)

### LENS — Complete When:
- [ ] `blacklight search "query"` returns ranked results from terminal
- [ ] `blacklight stats` shows usage overview
- [ ] All CLI flags documented in `--help`

### SIGNAL — Complete When:
- [ ] `blacklight serve` starts web server
- [ ] All API endpoints return correct data
- [ ] API handles errors gracefully (404, 400, 500)
- [ ] Concurrent requests handled without blocking

### MIRROR — Complete When:
- [ ] Dashboard renders with heatmap, charts, stats
- [ ] Session explorer lists, filters, and paginates sessions
- [ ] Search returns results with highlighted snippets
- [ ] Single binary serves the entire frontend

### REPLAY — Complete When:
- [ ] Full conversation thread renders with markdown + syntax highlighting
- [ ] Tool calls shown as expandable cards with tool-specific formatting
- [ ] Thinking blocks toggleable
- [ ] Edit diffs render correctly
- [ ] Session sidebar with navigator and metadata

### SPECTRUM — Complete When:
- [ ] Analytics charts render (tokens, tools, outcomes, friction)
- [ ] Storage treemap shows per-project disk usage
- [ ] File impact view ranks files by touch frequency
- [ ] Plans browsable with rendered markdown
- [ ] Tasks viewable with dependency visualization
