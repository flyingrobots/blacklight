# Blacklight Architecture

This document describes how Blacklight turns local LLM logs into a unified, verifiable "Causal Memory."

## Overview

Blacklight is a single binary that combines a filesystem crawler, a multi-format parser, a bit-perfect provenance engine, and an embedded web server. It treats every conversation as an immutable record in a content-addressed storage (CAS) layer.

---

## 1. Ingest & Backup Pipeline

Ingest is the process of discovering and securing source data.

### Discovery
The indexer scans configured and auto-discovered directories (e.g., `~/.claude`, `~/.gemini`). It uses a recursion depth of up to 6 to find deeply nested `.claude` projects in Application Support.

### CAS Backup (The Master Record)
Before parsing, transient files (like Gemini `tmp` sessions) are copied to a **Content-Addressable Storage (CAS)** layer.
- **Location:** `~/.blacklight/backups/`
- **Modes:** 
  - `gitcas`: Uses `git-cas` to chunk and deduplicate files into a Git ODB.
  - `simple`: Stores full files keyed by their BLAKE3 hash.
- **Naming:** Files are vaulted using a configurable prefix (e.g., `gemini:<hash>`), ensuring clear provenance at the storage level.

### Materialized Cache
To keep the UI snappy, files restored from `gitcas` are cached in `~/.blacklight/materialized/`.

---

## 2. Indexing & Fingerprinting

Indexing transforms raw files into queryable relational data.

### Multi-Format Parsing (Extensions)
Blacklight supports multiple LLM "Extensions":
- **Claude:** Parses JSONL streaming files and `sessions-index.json` metadata.
- **Gemini:** Parses standalone JSON session files with complex `thoughts` and `toolCalls` arrays.
- **Codex:** Parses unique rollout JSONL formats.

### Bit-Perfect Provenance
Every entity in the database is cryptographically locked:
1. **Turn Fingerprint:** A BLAKE3 hash of the message type, timestamp, and content blocks.
2. **Tool Call Fingerprint:** A BLAKE3 hash of the name, input, and output.
3. **Session Fingerprint (Merkle Root):** A BLAKE3 hash of the sorted list of all message fingerprints in the session.

This hierarchy ensures that any modification to a session's history is immediately detectable.

---

## 3. Data Storage

### SQLite (The Index)
The relational database stores the metadata and structure of the sessions.
- **FTS5:** Provides full-text search across all content.
- **WAL Mode:** Allows the web server to read while the indexer is writing.

### Blob Store (Content Deduplication)
Text blobs larger than 256 bytes (e.g., a source file read 100 times) are stored exactly once in the `content_store` table, keyed by BLAKE3 hash.

---

## 4. AI Enrichment

Enrichment adds semantic metadata to raw sessions.
- **Workflow:** A digest of the first 20 messages is sent to an LLM (Ollama, Gemini, or Claude).
- **Output:** The LLM generates a title, summary, and confidence-scored tags.
- **Approval:** Enrichments with low confidence scores land in a `pending_review` queue for manual user approval.

---

## 5. Migration (V3 to V4)

Migration is the process of upgrading an existing index to the bit-perfect standard.
- **Bulk Backup:** Scans all existing session files and vaults them into the CAS.
- **Fingerprinting:** Iterates through every message and session in the database to calculate and store their fingerprints.
- **Real-time Progress:** Managed via the `MigrationState` and displayed in the `IndexerHUD`.

---

## Extending Blacklight

The parser logic is designed to be extensible. To add a new LLM:
1. Define the data model in `models.rs`.
2. Implement a parser module in `indexer/<name>.rs`.
3. Register the `FileKind` in `scanner.rs` and the route in `indexer/mod.rs`.
