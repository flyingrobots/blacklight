-- Migration: Add weekly_digests table
CREATE TABLE weekly_digests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    start_date TEXT NOT NULL, -- ISO 8601 (inclusive)
    end_date TEXT NOT NULL,   -- ISO 8601 (exclusive)
    content TEXT NOT NULL,    -- Markdown content
    
    -- Cached stats for the week
    session_count INTEGER DEFAULT 0,
    success_count INTEGER DEFAULT 0,
    failed_count INTEGER DEFAULT 0,
    partial_count INTEGER DEFAULT 0,
    abandoned_count INTEGER DEFAULT 0,
    message_count INTEGER DEFAULT 0,
    
    created_at TEXT NOT NULL
);

CREATE INDEX idx_weekly_digests_start ON weekly_digests(start_date);
