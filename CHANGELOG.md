# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-02-18

### Added
- **Multi-Source Support:** Index data from `~/.claude/`, `~/.gemini/`, and `~/.codex/` simultaneously.
- **Claude Desktop Support:** Automatic discovery of sessions in `~/Library/Application Support/Claude/`.
- **Codex Integration:** Support for Codex's unique rollout JSONL format.
- **Bit-Perfect Provenance:** Every turn and tool call is cryptographically fingerprinted with BLAKE3. Sessions use Merkle roots for verifiable integrity.
- **Configurable CAS Backups:** Integration with `git-cas` to automatically vault transient files into a dedicated Git repository.
- **V3 to V4 Migration:** Automated bulk backup and fingerprinting process with a dedicated HUD tab for real-time progress tracking.
- **Provenance UI:** Origin and fingerprint details are now visible in session list and detail views.
- **Materialized Cache:** Snappy raw file viewing in the UI even when restoring from Git CAS.

### Changed
- Refactored `BlacklightConfig` to support a `sources` array with custom `cas_prefix` per source.
- Updated database schema to version 4 to include provenance, fingerprint, and backup tracking.
- Switched `/raw` API to strictly serve from the CAS master record.

## [0.1.0] - 2026-02-10
- Initial release with Claude Code JSONL support.
- Full-text search with FTS5.
- BLAKE3 content deduplication.
- AI enrichment (Ollama/Gemini/Claude).
- Web dashboard with analytics and session replay.
