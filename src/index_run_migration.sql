-- Migration: Add index_runs table to track ingestion history
CREATE TABLE index_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    status TEXT NOT NULL, -- 'running', 'completed', 'failed', 'cancelled'
    
    -- Config
    is_full INTEGER DEFAULT 0,
    
    -- Stats
    files_scanned INTEGER DEFAULT 0,
    files_processed INTEGER DEFAULT 0,
    files_unchanged INTEGER DEFAULT 0,
    sessions_parsed INTEGER DEFAULT 0,
    messages_processed INTEGER DEFAULT 0,
    blobs_inserted INTEGER DEFAULT 0,
    errors INTEGER DEFAULT 0,
    
    error_message TEXT
);

CREATE INDEX idx_index_runs_started ON index_runs(started_at);
