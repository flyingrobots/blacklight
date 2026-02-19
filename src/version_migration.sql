-- Migration 005: Versioning and Scheduler tracking

-- Add version columns to sessions
ALTER TABLE sessions ADD COLUMN index_version INTEGER DEFAULT 0;
ALTER TABLE sessions ADD COLUMN enrichment_version INTEGER DEFAULT 0;

-- Update schedule_config to track execution times
ALTER TABLE schedule_config ADD COLUMN last_run_at TEXT;
ALTER TABLE schedule_config ADD COLUMN next_run_at TEXT;
