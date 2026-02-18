-- V4: Multi-LLM provenance + backup tracking + CAS fingerprints

-- Add provenance to sessions
ALTER TABLE sessions ADD COLUMN source_name TEXT;
ALTER TABLE sessions ADD COLUMN source_kind TEXT;
ALTER TABLE sessions ADD COLUMN app_version TEXT;
ALTER TABLE sessions ADD COLUMN fingerprint TEXT;

-- Add provenance to messages
ALTER TABLE messages ADD COLUMN turn_index INTEGER;
ALTER TABLE messages ADD COLUMN source_name TEXT;
ALTER TABLE messages ADD COLUMN fingerprint TEXT;

-- Add fingerprint to tool_calls
ALTER TABLE tool_calls ADD COLUMN fingerprint TEXT;

-- Track file backups for transient sources (like Gemini tmp dirs) using CAS
CREATE TABLE IF NOT EXISTS session_backups (
    session_id TEXT PRIMARY KEY REFERENCES sessions(id),
    original_path TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    backed_up_at TEXT NOT NULL,
    file_size INTEGER NOT NULL
);

-- Index for source filtering
CREATE INDEX IF NOT EXISTS idx_sessions_source ON sessions(source_name, source_kind);
CREATE INDEX IF NOT EXISTS idx_messages_source ON messages(source_name);
CREATE INDEX IF NOT EXISTS idx_sessions_fingerprint ON sessions(fingerprint);
CREATE INDEX IF NOT EXISTS idx_messages_fingerprint ON messages(fingerprint);
