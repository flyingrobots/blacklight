use blacklight::db;
use rusqlite::params;
use tempfile::TempDir;

#[test]
fn test_fresh_db_creates_all_tables() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let conn = db::open(&db_path).unwrap();

    let tables = [
        "sessions",
        "messages",
        "content_blocks",
        "tool_calls",
        "content_store",
        "blob_references",
        "file_references",
        "tasks",
        "task_dependencies",
        "session_outcomes",
        "outcome_categories",
        "outcome_friction",
        "daily_stats",
        "model_usage",
        "indexed_files",
    ];

    for table in &tables {
        let count: i32 = conn
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{table}'"
                ),
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "table '{table}' should exist");
    }
}

#[test]
fn test_fts5_virtual_table_exists() {
    let tmp = TempDir::new().unwrap();
    let conn = db::open(&tmp.path().join("test.db")).unwrap();

    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='fts_content'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "fts_content virtual table should exist");
}

#[test]
fn test_fts5_insert_and_search() {
    let tmp = TempDir::new().unwrap();
    let conn = db::open(&tmp.path().join("test.db")).unwrap();

    conn.execute(
        "INSERT INTO fts_content (hash, kind, content) VALUES (?1, ?2, ?3)",
        params!["abc123", "text", "the quick brown fox jumps over the lazy dog"],
    )
    .unwrap();

    let snippet: String = conn
        .query_row(
            "SELECT snippet(fts_content, 2, '<mark>', '</mark>', '...', 64) FROM fts_content WHERE fts_content MATCH 'fox'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(snippet.contains("<mark>fox</mark>"));
}

#[test]
fn test_fts5_bm25_ranking() {
    let tmp = TempDir::new().unwrap();
    let conn = db::open(&tmp.path().join("test.db")).unwrap();

    conn.execute(
        "INSERT INTO fts_content (hash, kind, content) VALUES (?1, ?2, ?3)",
        params!["h1", "text", "rust rust rust programming language"],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO fts_content (hash, kind, content) VALUES (?1, ?2, ?3)",
        params!["h2", "text", "python is a programming language"],
    )
    .unwrap();

    let mut stmt = conn
        .prepare(
            "SELECT hash, bm25(fts_content) as rank FROM fts_content WHERE fts_content MATCH 'rust' ORDER BY rank",
        )
        .unwrap();

    let results: Vec<(String, f64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "h1");
}

#[test]
fn test_fts5_porter_stemming() {
    let tmp = TempDir::new().unwrap();
    let conn = db::open(&tmp.path().join("test.db")).unwrap();

    conn.execute(
        "INSERT INTO fts_content (hash, kind, content) VALUES (?1, ?2, ?3)",
        params!["stem1", "text", "the developer is running the tests"],
    )
    .unwrap();

    // "run" should match "running" via porter stemmer
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM fts_content WHERE fts_content MATCH 'run'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "porter stemming should match 'run' to 'running'");
}

#[test]
fn test_all_indexes_exist() {
    let tmp = TempDir::new().unwrap();
    let conn = db::open(&tmp.path().join("test.db")).unwrap();

    let expected_indexes = [
        "idx_messages_session",
        "idx_messages_type",
        "idx_tool_calls_session",
        "idx_tool_calls_name",
        "idx_content_blocks_message",
        "idx_blob_refs_hash",
        "idx_blob_refs_message",
        "idx_file_refs_path",
        "idx_file_refs_session",
        "idx_sessions_project",
        "idx_sessions_created",
    ];

    for idx in &expected_indexes {
        let count: i32 = conn
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='{idx}'"
                ),
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "index '{idx}' should exist");
    }
}

#[test]
fn test_foreign_keys_enforced() {
    let tmp = TempDir::new().unwrap();
    let conn = db::open(&tmp.path().join("test.db")).unwrap();

    // Trying to insert a message referencing a non-existent session should fail
    let result = conn.execute(
        "INSERT INTO messages (id, session_id, type, timestamp) VALUES ('m1', 'nonexistent', 'user', '2026-01-01T00:00:00Z')",
        [],
    );
    assert!(
        result.is_err(),
        "foreign key constraint should prevent inserting message with invalid session_id"
    );
}

#[test]
fn test_wal_mode_active() {
    let tmp = TempDir::new().unwrap();
    let conn = db::open(&tmp.path().join("test.db")).unwrap();

    let mode: String = conn
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
}

#[test]
fn test_migration_version_set() {
    let tmp = TempDir::new().unwrap();
    let conn = db::open(&tmp.path().join("test.db")).unwrap();

    let version: u32 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, 2);
}

#[test]
fn test_already_migrated_db_no_error() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");

    // First open runs migrations
    let _conn1 = db::open(&db_path).unwrap();
    drop(_conn1);

    // Second open should succeed without re-running migrations
    let conn2 = db::open(&db_path).unwrap();
    let version: u32 = conn2
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, 2);
}

#[test]
fn test_insert_and_select_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let conn = db::open(&tmp.path().join("test.db")).unwrap();

    // Insert a session
    conn.execute(
        "INSERT INTO sessions (id, project_path, project_slug, created_at, modified_at, source_file)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "sess-1",
            "/Users/test/project",
            "project",
            "2026-01-01T00:00:00Z",
            "2026-01-01T01:00:00Z",
            "/path/to/file.jsonl"
        ],
    )
    .unwrap();

    // Insert a message
    conn.execute(
        "INSERT INTO messages (id, session_id, type, timestamp)
         VALUES (?1, ?2, ?3, ?4)",
        params!["msg-1", "sess-1", "user", "2026-01-01T00:00:00Z"],
    )
    .unwrap();

    // Verify roundtrip
    let session_slug: String = conn
        .query_row(
            "SELECT project_slug FROM sessions WHERE id = 'sess-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(session_slug, "project");

    let msg_type: String = conn
        .query_row(
            "SELECT type FROM messages WHERE id = 'msg-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(msg_type, "user");
}
