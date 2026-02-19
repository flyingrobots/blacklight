# Changelog

All notable changes to this project will be documented in this file.

## [0.3.0] - 2026-02-18

### Added
- **D3 Dashboard Visualizations:** Replaced static stats with interactive D3.js charts, including a GitHub-style activity heatmap and horizontal bar charts for Project/LLM breakdowns.
- **Dynamic Time Windowing:** Added a global time slider (7d, 30d, 90d, 1y, All) that re-aggregates all dashboard analytics in real-time.
- **LLM Usage Breakdown:** New analytics for Sessions, Messages, and Tool usage partitioned by LLM source (Claude, Gemini, Codex).
- **TUI-Style Navigation:** Implemented `j/k` (Vim-style) keyboard navigation in the session list.
- **Centered Scrolling:** Navigation uses GSAP `ScrollToPlugin` to keep the selected item vertically centered in the viewport.
- **Vim-style Search:** Press `/` anywhere to instantly focus the search bar.
- **Quartz Light Theme:** A high-contrast light mode for better legibility in bright environments.
- **Terminal Aesthetic:** Introduced a "Mono" design system using monospace fonts, sharp borders, and high-contrast indicators (`>`) for a tactile TUI feel.

### Changed
- **Backend Stability:** Switched server binding to `127.0.0.1` and added startup delays to frontend polling to eliminate connection noise during development.
- **Improved Contrast:** Adjusted background blending in dark themes (Slate, Indigo, Orchid) to eliminate "pitch black" backgrounds and improve text legibility.
- **Layout Refinement:** Moved navigation tabs above the wordmark and tightened the header layout for better focal hierarchy.
- **Developer Experience:** Added a root-level `npm run dev` script using `concurrently` to start both the Rust backend and Vite dev server.

### Fixed
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
