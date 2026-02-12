-- V3: Scheduled enrichment + approval workflow

ALTER TABLE session_enrichments ADD COLUMN approval_status TEXT NOT NULL DEFAULT 'approved';
ALTER TABLE session_enrichments ADD COLUMN reviewed_at TEXT;

CREATE TABLE IF NOT EXISTS schedule_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    enabled INTEGER NOT NULL DEFAULT 1,
    interval_minutes INTEGER NOT NULL DEFAULT 60,
    run_enrichment INTEGER NOT NULL DEFAULT 1,
    enrichment_concurrency INTEGER NOT NULL DEFAULT 5,
    updated_at TEXT NOT NULL
);

INSERT OR IGNORE INTO schedule_config (id, enabled, interval_minutes, run_enrichment, enrichment_concurrency, updated_at)
VALUES (1, 1, 60, 1, 5, datetime('now'));

CREATE INDEX IF NOT EXISTS idx_enrichments_approval
    ON session_enrichments(approval_status) WHERE approval_status = 'pending_review';
