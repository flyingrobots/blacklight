-- V2: AI-powered session enrichment tables

CREATE TABLE IF NOT EXISTS session_enrichments (
    session_id TEXT PRIMARY KEY REFERENCES sessions(id),
    title TEXT NOT NULL,
    summary TEXT NOT NULL,
    enriched_at TEXT NOT NULL,
    model_used TEXT
);

CREATE TABLE IF NOT EXISTS session_tags (
    session_id TEXT NOT NULL REFERENCES sessions(id),
    tag TEXT NOT NULL,
    confidence REAL NOT NULL,
    PRIMARY KEY (session_id, tag)
);
