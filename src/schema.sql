-- Blacklight V1 Schema
-- All tables, indexes, and FTS5 virtual table for ~/.claude/ data indexing.

-- Session metadata (from sessions-index.json)
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    project_path TEXT NOT NULL,
    project_slug TEXT NOT NULL,
    first_prompt TEXT,
    summary TEXT,
    message_count INTEGER,
    created_at TEXT NOT NULL,
    modified_at TEXT NOT NULL,
    git_branch TEXT,
    claude_version TEXT,
    is_sidechain INTEGER DEFAULT 0,
    source_file TEXT NOT NULL
);

-- Individual messages (from session JSONL)
CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    parent_id TEXT,
    type TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    model TEXT,
    stop_reason TEXT,
    cwd TEXT,
    git_branch TEXT,
    duration_ms INTEGER
);

-- Content blocks within messages (flattened from message.content[])
CREATE TABLE content_blocks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    message_id TEXT NOT NULL REFERENCES messages(id),
    block_index INTEGER NOT NULL,
    block_type TEXT NOT NULL,
    content_hash TEXT REFERENCES content_store(hash),
    tool_name TEXT,
    tool_use_id TEXT,
    tool_input_hash TEXT REFERENCES content_store(hash)
);

-- Tool calls (denormalized view for fast queries)
CREATE TABLE tool_calls (
    id TEXT PRIMARY KEY,
    message_id TEXT NOT NULL REFERENCES messages(id),
    session_id TEXT NOT NULL REFERENCES sessions(id),
    tool_name TEXT NOT NULL,
    input_hash TEXT REFERENCES content_store(hash),
    output_hash TEXT REFERENCES content_store(hash),
    timestamp TEXT NOT NULL
);

-- Content addressable blob store
CREATE TABLE content_store (
    hash TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    size INTEGER NOT NULL,
    kind TEXT
);

-- Maps blobs to every message that produced/consumed them
CREATE TABLE blob_references (
    hash TEXT NOT NULL REFERENCES content_store(hash),
    message_id TEXT NOT NULL REFERENCES messages(id),
    context TEXT NOT NULL,
    PRIMARY KEY (hash, message_id, context)
);

-- File path -> content hash -> sessions
CREATE TABLE file_references (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,
    content_hash TEXT NOT NULL REFERENCES content_store(hash),
    session_id TEXT NOT NULL REFERENCES sessions(id),
    message_id TEXT NOT NULL REFERENCES messages(id),
    operation TEXT NOT NULL
);

-- Tasks (from tasks/*.json)
CREATE TABLE tasks (
    id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    subject TEXT NOT NULL,
    description TEXT NOT NULL,
    active_form TEXT,
    status TEXT NOT NULL,
    PRIMARY KEY (session_id, id)
);

-- Task dependencies
CREATE TABLE task_dependencies (
    session_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    depends_on TEXT NOT NULL,
    PRIMARY KEY (session_id, task_id, depends_on)
);

-- Session outcomes (from usage-data/facets/*.json)
CREATE TABLE session_outcomes (
    session_id TEXT PRIMARY KEY,
    underlying_goal TEXT,
    outcome TEXT,
    helpfulness TEXT,
    session_type TEXT,
    primary_success TEXT,
    friction_detail TEXT,
    brief_summary TEXT
);

-- Outcome goal categories
CREATE TABLE outcome_categories (
    session_id TEXT NOT NULL REFERENCES session_outcomes(session_id),
    category TEXT NOT NULL,
    count INTEGER DEFAULT 1,
    PRIMARY KEY (session_id, category)
);

-- Outcome friction types
CREATE TABLE outcome_friction (
    session_id TEXT NOT NULL REFERENCES session_outcomes(session_id),
    friction_type TEXT NOT NULL,
    count INTEGER DEFAULT 1,
    PRIMARY KEY (session_id, friction_type)
);

-- Daily stats (from stats-cache.json)
CREATE TABLE daily_stats (
    date TEXT PRIMARY KEY,
    message_count INTEGER,
    session_count INTEGER,
    tool_call_count INTEGER
);

-- Model usage (from stats-cache.json)
CREATE TABLE model_usage (
    model TEXT PRIMARY KEY,
    input_tokens INTEGER,
    output_tokens INTEGER,
    cache_read_tokens INTEGER,
    cache_creation_tokens INTEGER
);

-- Indexer state: track what's been indexed for incremental updates
CREATE TABLE indexed_files (
    file_path TEXT PRIMARY KEY,
    mtime_ms INTEGER NOT NULL,
    size_bytes INTEGER NOT NULL,
    last_byte_offset INTEGER DEFAULT 0,
    indexed_at TEXT NOT NULL
);

-- Indexes
CREATE INDEX idx_messages_session ON messages(session_id, timestamp);
CREATE INDEX idx_messages_type ON messages(type);
CREATE INDEX idx_tool_calls_session ON tool_calls(session_id);
CREATE INDEX idx_tool_calls_name ON tool_calls(tool_name);
CREATE INDEX idx_content_blocks_message ON content_blocks(message_id);
CREATE INDEX idx_blob_refs_hash ON blob_references(hash);
CREATE INDEX idx_blob_refs_message ON blob_references(message_id);
CREATE INDEX idx_file_refs_path ON file_references(file_path);
CREATE INDEX idx_file_refs_session ON file_references(session_id);
CREATE INDEX idx_sessions_project ON sessions(project_slug);
CREATE INDEX idx_sessions_created ON sessions(created_at);

-- Full-text search index over unique content blobs
CREATE VIRTUAL TABLE fts_content USING fts5(
    hash UNINDEXED,
    kind,
    content,
    tokenize='porter unicode61'
);
