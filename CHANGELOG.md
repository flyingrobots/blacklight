# Changelog

All notable changes to this project will be documented in this file.

## [0.3.0] - 2026-02-18

### Added
- **Maintenance Badges:** The IndexerHUD now shows visual badges ('!') when re-indexing, enrichment, or migration is required due to logic or schema updates.
- **Scheduler Tracking:** The Schedule tab now displays 'Last Run' and 'Next Run' timestamps for background maintenance tasks.
- **Session Versioning:** Implemented internal version tracking for indexing and enrichment to detect outdated history.
- **Interactive Activity Heatmap Tooltips:** Hovering over heatmap cells now displays a detailed breakdown of session counts per project for that day.
- **LLM Source Metadata:** The indexer now records the specific source LLM (Claude, Gemini, Codex) for every session based on the configuration TOML.
- **Zero-Value Filtering:** Dashboard charts now automatically hide categories with zero activity, providing a cleaner view of active projects and models.
- **D3 Dashboard Visualizations:** Replaced static stats with interactive D3.js charts, including a GitHub-style activity heatmap and horizontal bar charts for Project/LLM breakdowns.
- **Dynamic Time Windowing:** Added a global time slider (7d, 30d, 90d, 1y, All) that re-aggregates all dashboard analytics in real-time.

### Changed
- **Dashboard Performance:** Split dashboard data loading into essential (sessions) and non-essential (analytics) requests. Recent sessions now appear immediately while analytics are processed in the background.
- **Dashboard Layout:** Reordered the home screen to prioritize "Recent Sessions" at the top, followed by interactive activity and analytics sections.
- **Default Time Window:** The dashboard now defaults to the "Last 7 Days" for a more focused view of recent work.
- **Heatmap Responsive Range:** The activity heatmap now respects the global time slider window (defaulting to 6 months for 'All Time').
- **Heatmap Styling:** Improved heatmap legibility and transitioned all chart elements to use theme tokens.
- **Backend Stability:** Switched server binding to `127.0.0.1` and added startup delays to frontend polling to eliminate connection noise during development.

### Fixed
- **Database Contention:** Added a 5-second `busy_timeout` to SQLite connections to resolve "database is locked" errors during high-concurrency operations like the V4 migration.
- **Dashboard Initialization:** Resolved an issue where the dashboard could stay in a "Loading..." state by ensuring the time slider always emits an initial selection and adding a fallback fetch on mount.
- **Heatmap Styling:** Fixed "impossible to read" text in the heatmap by using theme tokens and improving contrast.
- **Theme Compositor:** Fixed a build-breaking missing function in the theme engine.

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

### Fixed
- **Analytics SQL Queries:** Fixed "Wrong number of parameters" error when loading the dashboard by ensuring SQL placeholders consistently match the passed arguments regardless of filter state.
- **FTS5 Search Sanitization:** Fixed "no such column" error when searching for terms containing colons or hyphens (like CAS prefixes).
- **Indexer Phase 3:** Restored structured data parsing (tasks, facets, plans) that was bypassed during multi-source refactor.
- **Test Suite:** Updated and stabilized entire test suite for schema v4 and multi-source configuration.

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
