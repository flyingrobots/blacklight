# Refactoring Plan: Architectural Improvements for Blacklight

This document outlines a prioritized plan to enact five major architectural improvements to the Blacklight codebase. Each section is formatted as a ready-to-use GitHub Issue, containing all the context, goals, and technical details required for an AI agent or human developer to implement the change autonomously.

---

## Issue 1: Implement Structured Domain Errors

**Priority:** 1 (Foundation)
**Labels:** `refactor`, `backend`, `tech-debt`

### Background
Currently, the application heavily relies on `anyhow::Error` for error propagation. At the API boundary (in `src/server/errors.rs`), `AppError` acts as a generic wrapper that mostly converts these `anyhow` errors into HTTP `500 Internal Server Error` responses with a string message. This makes it difficult for the frontend to handle specific error cases gracefully and makes server logs less structured.

### Goal
Replace generic `anyhow` errors at the domain and API boundaries with a structured, strongly-typed error enum using the `thiserror` crate. Map these domain errors to semantically appropriate HTTP status codes.

### Implementation Steps
1. **Add Dependency:** Add `thiserror` to `Cargo.toml`.
2. **Define Domain Errors:** Create `src/error.rs` (or heavily refactor `src/server/errors.rs`) to define a `BlacklightError` enum:
   ```rust
   use thiserror::Error;

   #[derive(Error, Debug)]
   pub enum BlacklightError {
       #[error("Entity not found: {0}")]
       NotFound(String),
       #[error("Database error: {0}")]
       Database(#[from] rusqlite::Error),
       #[error("I/O error: {0}")]
       Io(#[from] std::io::Error),
       #[error("Parse error: {0}")]
       Parse(String),
       #[error("Internal error: {0}")]
       Internal(String),
       // ... add others like BadRequest, ConfigError, etc.
   }
   ```
3. **Implement IntoResponse:** Implement `axum::response::IntoResponse` for `BlacklightError` to map `NotFound` to 404, `Database/Io/Internal` to 500, etc.
4. **Refactor API Handlers:** Update all handlers in `src/server/api/*.rs` to return `Result<axum::Json<T>, BlacklightError>` instead of `AppError`.
5. **Push Down:** Gradually push the use of `BlacklightError` down into `src/server/queries/` and `src/db.rs` so that errors are typed closer to their source. Keep `anyhow` only for CLI-specific top-level logic if desired.

---

## Issue 2: Refactor Global State & Concurrency Model

**Priority:** 2 (Architecture/Performance)
**Labels:** `refactor`, `backend`, `performance`

### Background
The application state (`AppState` in `src/server/state.rs`) wraps highly contended resources like `IndexerState`, `EnricherState`, and `MigrationState` in `Arc<tokio::sync::Mutex<T>>`. Because the Indexer performs heavy synchronous I/O and database operations, holding a standard Tokio Mutex can cause the web server to stall when API handlers attempt to read the state for UI progress updates.

### Goal
Move away from heavy, write-locked Mutexes for state that is frequently read by the UI. Implement a lock-free or read-optimized concurrency model.

### Implementation Steps
1. **Adopt Watch Channels:** For progress tracking (e.g., `IndexerState`), replace the `Mutex` with `tokio::sync::watch` channels.
   - The Indexer should own the `watch::Sender<IndexerProgress>`.
   - The `AppState` should store the `watch::Receiver<IndexerProgress>`.
2. **Update AppState:** Refactor `AppState` to look something like:
   ```rust
   pub struct AppState {
       pub db: Arc<DbPool>,
       pub config: Arc<BlacklightConfig>,
       pub indexer_progress: tokio::sync::watch::Receiver<IndexerState>,
       // Use Actor pattern channels for control
       pub indexer_tx: tokio::sync::mpsc::Sender<IndexerCommand>, 
       // ...
   }
   ```
3. **Actor Pattern for Control:** Instead of locking `IndexerState` to mutate its pause/cancel flags, send commands (`Start`, `Pause`, `Stop`) via an `mpsc` channel to an Indexer background task (Actor).
4. **Refactor Indexer Mod:** Update `src/indexer/mod.rs` to take the `watch::Sender` and broadcast updates via `sender.send(...)` instead of taking a Mutex guard and mutating state.
5. **Update API:** Update handlers in `src/server/api/indexer.rs` to read from the watch receiver (`state.indexer_progress.borrow().clone()`), ensuring instantaneous, non-blocking reads.

---

## Issue 3: Type-Safe SQL & Centralized Queries (Repository Pattern)

**Priority:** 3 (Maintainability)
**Labels:** `refactor`, `database`

### Background
Currently, raw SQL strings and manual row mapping using `rusqlite` are scattered across API handlers, `src/server/queries/*.rs`, and `src/indexer/*.rs`. This makes the application brittle to schema changes and obscures the data access layer.

### Goal
Centralize all database interactions into a strict Repository Pattern and introduce stronger typing for SQL inputs and outputs, eliminating raw SQL in API and Indexer orchestration logic.

### Implementation Steps
1. **Centralize Data Access:** Ensure *all* SQL execution is confined to files within `src/server/queries/` (for reads) and `src/indexer/db_ops.rs` (for writes). API handlers and the main indexer loop should never call `conn.execute(...)` directly.
2. **Strict Typed Structs:** Define specific Rust structs for every query's parameters and return types. Avoid returning dynamic or loosely typed tuples.
3. **Compile-time Checking (Optional but recommended):** While the app currently uses synchronous `rusqlite`, evaluate migrating to `sqlx` (async SQLite). If migrating to `sqlx`:
   - Replace `rusqlite::Connection` with `sqlx::SqlitePool`.
   - Rewrite queries using the `sqlx::query!` macro to validate SQL against the schema at compile time.
4. **Remove `serde_json::Value` from Queries:** Currently, many queries in `src/server/queries/sessions.rs` return generic JSON values. Refactor these to return proper Rust domain models (e.g., `Session`, `Message`), and let the API layer handle JSON serialization.

---

## Issue 4: Modularize the Indexer (Source Provider Pattern)

**Priority:** 4 (Extensibility)
**Labels:** `refactor`, `indexer`, `architecture`

### Background
The core indexing logic in `src/indexer/mod.rs` (`run_index`) is procedural and monolithic. It explicitly hardcodes the partitioning, change detection, and execution phases for `claude`, `gemini`, and `codex`. Adding a new LLM log source requires risky modifications to this central loop.

### Goal
Implement a `SourceProvider` trait. Refactor the indexer to dynamically register providers and iterate over them, decoupling the indexing engine from the specifics of any single LLM log format.

### Implementation Steps
1. **Define the Trait:** In `src/indexer/provider.rs`, define:
   ```rust
   pub trait SourceProvider: Send + Sync {
       fn name(&self) -> &'static str;
       fn supported_kinds(&self) -> Vec<FileKind>;
       /// How to parse metadata/indexes
       fn process_metadata(&self, conn: &Connection, entry: &FileEntry) -> Result<()>;
       /// How to process actual chat content
       fn process_content(&self, conn: &Connection, entry: &FileEntry, start_offset: u64) -> Result<ProcessStats>;
   }
   ```
2. **Implement Providers:** Create structs like `ClaudeProvider`, `GeminiProvider`, and `CodexProvider` that implement this trait, moving the logic currently scattered in `run_index`, `gemini.rs`, `codex.rs`, etc., into these implementations.
3. **Refactor Engine Loop:** Modify `run_index` to:
   - Perform a global scan.
   - Run the generic change detection (`change::detect_changes`).
   - Group changed files by `FileKind`.
   - Dispatch files to the appropriate `SourceProvider` based on their kind.
4. **Remove Hardcoding:** Ensure no variable names like `gemini_sessions` or `session_jsonls` exist in `run_index`. It should only deal with abstract `FileEntry` and `ProcessStats`.

---

## Issue 5: End-to-End Type Safety (Rust to TypeScript)

**Priority:** 5 (DX / Developer Experience)
**Labels:** `frontend`, `backend`, `dx`

### Background
The API currently returns weakly typed `serde_json::Value` in Axum handlers, and the Vue.js frontend manually defines TypeScript interfaces in `frontend/src/types/index.ts`. If a backend model changes, the frontend might break silently until runtime.

### Goal
Generate TypeScript interfaces automatically from the Rust backend structs to ensure the API contract is strictly enforced at compile-time on both sides.

### Implementation Steps
1. **Add `ts-rs`:** Add `ts-rs = "7.0"` to `Cargo.toml`.
2. **Annotate Rust Models:** In `src/models.rs`, `src/server/responses.rs`, and `src/server/params.rs`, add the derivation macros to API-facing structs:
   ```rust
   use ts_rs::TS;
   use serde::{Serialize, Deserialize};

   #[derive(Serialize, Deserialize, TS)]
   #[ts(export, export_to = "../frontend/src/types/generated/")]
   pub struct Session {
       pub id: String,
       pub title: Option<String>,
       // ...
   }
   ```
3. **Refactor API Handlers:** Ensure all Axum handlers return strongly typed JSON: `Result<Json<Session>, BlacklightError>` instead of `Result<Json<serde_json::Value>, ...>`.
4. **Build Script (Optional):** Add a unit test or `build.rs` script that ensures `cargo test` triggers the generation of the `.ts` files, keeping them up to date.
5. **Update Frontend:** In the Vue.js frontend, delete the manual types in `frontend/src/types/index.ts` and update all imports to point to the new `frontend/src/types/generated/` definitions. Fix any resulting TypeScript compiler errors.
