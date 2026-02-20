# Blacklight Roadmap

Last updated: 2026-02-20
Status: Active

## Product Plan

### Mission
Blacklight is a local-first memory and decision-support system for AI-assisted engineering work.

Blacklight should answer four questions faster than any manual workflow:
1. Have I solved this before?
2. What worked last time, and why?
3. What keeps failing repeatedly?
4. What should I do next?

### Product Positioning
Blacklight is not a generic analytics dashboard.

Analytics is useful only when it changes behavior.
The product focus is memory, leverage, and decision quality.

### North-Star Use Cases
1. Incident recall in under 60 seconds: find past fixes with exact error text and project scope.
2. Project memory for future-you: preserve repo-specific gotchas, migrations, and proven workflows.
3. Failure autopsy loop: identify recurring failure modes and corrective actions.
4. Decision provenance: trace a code change back to session/tool/rationale context.
5. Reusable playbooks: turn successful session patterns into repeatable procedures.

### Product Principles
1. Trust before insight: if ingestion is incomplete or incorrect, every downstream feature is noise.
2. Local-first by default: fast, private, and operational without external services.
3. Evidence over vibes: recommendations should link to concrete sessions and artifacts.
4. Low-friction daily loop: command-line and UI workflows must be one action away.
5. Behavioral impact over chart volume: features must produce better decisions, not just visuals.

## Current State (Reality Check)

### Working Well
1. Multi-source ingestion for Claude, Gemini, and Codex.
2. Local storage/search stack: SQLite + FTS5 + embedded web UI.
3. Background indexing/enrichment control surface (HUD + scheduler).
4. Provenance foundations: session/message/tool fingerprints.
5. Recent hardening fixes: scanner classification gaps, raw retrieval mismatch, search project filtering, wildcard CORS removal.

### Gaps That Matter
1. CLI `search` and `stats` are still stubs in `src/main.rs`.
2. Planning documents were drifting from implementation details.
3. No first-class ingestion coverage report per source/path/kind.
4. Privacy controls are incomplete: redaction/exclusion/retention are not productized.
5. Decision-support layer is early: no shipped autopsy/playbook engine yet.

## 2026 Priorities

### P0: Trust and Safety
Goal: Make Blacklight reliable enough to be used as operational memory.

Success criteria:
1. Ingestion completeness is measurable and visible after every run.
2. Parse failures are surfaced with actionable diagnostics.
3. Sensitive content controls exist (redaction/exclusion/retention).

### P1: Retrieval Speed and Daily Workflow
Goal: Make recall faster than re-solving.

Success criteria:
1. Project-scoped retrieval is one command or two clicks away.
2. Session open/jump workflows are fast and stable.
3. Query-to-answer loop is under 60 seconds for common debugging tasks.

### P2: Decision Support
Goal: Convert historical sessions into actionable guidance.

Success criteria:
1. Users can classify outcomes and recurring failure modes.
2. Weekly digest provides concrete actions (not vanity metrics).
3. At least one high-leverage feature (autopsy or playbook miner) is used weekly.

### P3: Provenance and Team-Grade Explainability
Goal: Explain why changes happened and what evidence supported them.

Success criteria:
1. Session-to-commit/file lineage is queryable.
2. Top-risk files and workflows are discoverable from evidence.
3. Rationale retrieval reduces repeated archaeology.

## Delivery Plan (Now / Next / Later)

### Now (0-6 weeks)

#### Track A: Ingestion Trust Dashboard
Scope:
1. Add index run ledger with per-source counts and parse errors.
2. Add coverage views by source kind and file kind.
3. Add regression tests for source-specific scanner/classifier behavior.

Acceptance criteria:
1. Each index run persists totals for scanned files, processed files, sessions, messages, errors.
2. Coverage drops are visible in UI and API responses.
3. Unknown or skipped file kinds are explicitly listed.

#### Track B: CLI Retrieval Loop
Scope:
1. Implement `blacklight search` with project/kind/limit/from/to filters.
2. Add `blacklight open <session-id>` to deep-link into the local UI session page.
3. Keep CLI output script-friendly with JSON mode.

Acceptance criteria:
1. `blacklight search "query" --project <slug>` returns ranked results.
2. P95 command latency under 500ms on warm DB for typical queries.
3. CLI help matches actual behavior and docs.

#### Track C: Privacy Controls v1
Scope:
1. Add configurable path exclusion rules.
2. Add optional redaction pass for common secrets in indexed text.
3. Add retention controls for raw backups.

Acceptance criteria:
1. Excluded paths are never parsed or indexed.
2. Redaction behavior is deterministic and test-covered.
3. Retention policy can prune old backup artifacts safely.

### Next (6-12 weeks)

#### Track D: Outcome Taxonomy + Failure Autopsy v1
Scope:
1. Add explicit session outcome labels (`success`, `partial`, `failed`, `abandoned`).
2. Add failure reason codes (repro missing, context drift, tool misuse, dependency trap, unknown).
3. Build autopsy report page with recurring failure clusters.

Acceptance criteria:
1. Outcome/reason is stored and queryable for >= 80% of recent sessions.
2. Weekly autopsy report includes top recurring patterns and examples.
3. Report generates at least one recommended process improvement.

#### Track E: Weekly Decision Digest
Scope:
1. Generate digest from last 7 days by project.
2. Highlight recurring wins, recurring failures, and open loops.
3. Include direct links to supporting sessions.

Acceptance criteria:
1. Digest lists at least three actionable items.
2. Every item links to evidence (session IDs + snippets).
3. Users can mark digest items completed/dismissed.

### Later (12+ weeks)

#### Track F: Playbook Miner v1
Scope:
1. Extract repeated successful sequences by task archetype.
2. Save as reusable playbooks with "works when" conditions.
3. Show confidence derived from repeated outcomes.

Acceptance criteria:
1. At least five useful playbooks generated from real data.
2. Each playbook references source sessions.
3. Users can accept/edit/reject generated playbooks.

#### Track G: Git Provenance Integration
Scope:
1. Link sessions to commits/diffs/touched files when available.
2. Enable reverse query: file or commit -> originating sessions.
3. Surface high-risk files with repeated failed sessions.

Acceptance criteria:
1. Session-to-commit linkage works for new commits.
2. File-centric view shows associated sessions and outcomes.
3. Provenance queries are available in API and UI.

## Metrics That Matter

### Product Metrics
1. Recall success rate: percent of "known prior issue" lookups resolved from history.
2. Median time-to-recall: query to actionable prior context.
3. Repeat failure rate: recurring failure mode frequency week-over-week.
4. Playbook reuse rate: number of sessions using accepted playbooks.

### System Metrics
1. Ingestion coverage by source/kind.
2. Parse error rate per 10k records.
3. Index duration and throughput.
4. Search latency (P50/P95).

Target direction:
1. Increase recall success and playbook reuse.
2. Decrease time-to-recall and repeat failure rate.

## Non-Goals (for this cycle)
1. Expanding vanity dashboards without clear decisions they support.
2. Adding vector search before trust/retrieval basics are stable.
3. Multi-tenant or cloud synchronization features.
4. Large UI redesigns unrelated to retrieval or decision support.

## Execution Backlog (Priority Ordered)
1. Add run-ledger schema + API endpoints for coverage/health.
2. Implement CLI `search` and `open`; align docs/help output.
3. Add configurable exclusions + redaction + retention controls.
4. Add outcome + reason-code capture and editing flows.
5. Implement autopsy clustering and weekly digest generation.
6. Ship playbook miner prototype and feedback loop.

## Definition of Done for This Roadmap Refresh
1. Roadmap is the source of truth for active priorities.
2. Every shipped feature maps to one track above.
3. CHANGELOG entries reference roadmap tracks.
4. Roadmap gets reviewed and updated at least bi-weekly.

## Update Process
1. Keep this file as the living plan.
2. Keep `TECH-PLAN.md` as deep technical reference.
3. Any major scope change must update this roadmap in the same PR.
