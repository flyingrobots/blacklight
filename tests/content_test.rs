use blacklight::content::*;
use blacklight::db;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Hashing tests
// ---------------------------------------------------------------------------

#[test]
fn test_hash_deterministic() {
    let h1 = hash_content("hello world");
    let h2 = hash_content("hello world");
    assert_eq!(h1, h2);
}

#[test]
fn test_hash_different_input_different_output() {
    let h1 = hash_content("hello");
    let h2 = hash_content("world");
    assert_ne!(h1, h2);
}

#[test]
fn test_hash_is_64_hex_chars() {
    let h = hash_content("test");
    assert_eq!(h.len(), 64);
    assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_hash_empty_string() {
    let h = hash_content("");
    assert_eq!(h.len(), 64);
}

#[test]
fn test_hash_bytes() {
    let h1 = hash_content("test");
    let h2 = hash_content_bytes(b"test");
    assert_eq!(h1, h2);
}

#[test]
fn test_should_dedup_threshold() {
    assert!(!should_dedup(""));
    assert!(!should_dedup(&"x".repeat(255)));
    assert!(should_dedup(&"x".repeat(256)));
    assert!(should_dedup(&"x".repeat(1000)));
}

// ---------------------------------------------------------------------------
// Content store tests
// ---------------------------------------------------------------------------

fn open_test_db() -> rusqlite::Connection {
    let tmp = TempDir::new().unwrap();
    // We leak the TempDir so it doesn't get cleaned up during the test.
    // This is fine for tests.
    let path = tmp.path().join("test.db");
    let conn = db::open(&path).unwrap();
    std::mem::forget(tmp);
    conn
}

#[test]
fn test_insert_and_retrieve_blob() {
    let conn = open_test_db();
    let hash = hash_content("blob content here");
    let is_new = insert_blob(&conn, &hash, "blob content here", 17, "text").unwrap();
    assert!(is_new);

    let blob = get_blob(&conn, &hash).unwrap().unwrap();
    assert_eq!(blob.content, "blob content here");
    assert_eq!(blob.size, 17);
    assert_eq!(blob.kind, "text");
}

#[test]
fn test_insert_blob_dedup() {
    let conn = open_test_db();
    let hash = hash_content("dedup test");

    let first = insert_blob(&conn, &hash, "dedup test", 10, "text").unwrap();
    assert!(first, "first insert should be new");

    let second = insert_blob(&conn, &hash, "dedup test", 10, "text").unwrap();
    assert!(!second, "second insert should be dedup hit");
}

#[test]
fn test_blob_exists() {
    let conn = open_test_db();
    let hash = hash_content("exists test");

    assert!(!blob_exists(&conn, &hash).unwrap());
    insert_blob(&conn, &hash, "exists test", 11, "text").unwrap();
    assert!(blob_exists(&conn, &hash).unwrap());
}

#[test]
fn test_get_nonexistent_blob() {
    let conn = open_test_db();
    let result = get_blob(&conn, "nonexistent_hash").unwrap();
    assert!(result.is_none());
}

#[test]
fn test_blob_references() {
    let conn = open_test_db();
    let hash = hash_content("ref test content");
    insert_blob(&conn, &hash, "ref test content", 16, "text").unwrap();

    // We need a session and message for foreign key constraints
    conn.execute(
        "INSERT INTO sessions (id, project_path, project_slug, created_at, modified_at, source_file)
         VALUES ('sess-1', '/test', 'test', '2026-01-01', '2026-01-01', '/test.jsonl')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO messages (id, session_id, type, timestamp)
         VALUES ('msg-1', 'sess-1', 'user', '2026-01-01')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO messages (id, session_id, type, timestamp)
         VALUES ('msg-2', 'sess-1', 'assistant', '2026-01-01')",
        [],
    )
    .unwrap();

    insert_blob_reference(&conn, &hash, "msg-1", "response_text").unwrap();
    insert_blob_reference(&conn, &hash, "msg-2", "tool_output").unwrap();

    let refs = get_blob_references(&conn, &hash).unwrap();
    assert_eq!(refs.len(), 2);
    assert!(refs.iter().any(|r| r.message_id == "msg-1"));
    assert!(refs.iter().any(|r| r.message_id == "msg-2"));
}

#[test]
fn test_insert_blob_reference_idempotent() {
    let conn = open_test_db();
    let hash = hash_content("ref idempotent");
    insert_blob(&conn, &hash, "ref idempotent", 14, "text").unwrap();

    conn.execute(
        "INSERT INTO sessions (id, project_path, project_slug, created_at, modified_at, source_file)
         VALUES ('sess-1', '/test', 'test', '2026-01-01', '2026-01-01', '/test.jsonl')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO messages (id, session_id, type, timestamp)
         VALUES ('msg-1', 'sess-1', 'user', '2026-01-01')",
        [],
    )
    .unwrap();

    // Insert same reference twice — should not error
    insert_blob_reference(&conn, &hash, "msg-1", "response_text").unwrap();
    insert_blob_reference(&conn, &hash, "msg-1", "response_text").unwrap();

    let refs = get_blob_references(&conn, &hash).unwrap();
    assert_eq!(refs.len(), 1);
}

#[test]
fn test_batch_insert_blobs() {
    let conn = open_test_db();

    let blobs: Vec<ContentBlob> = (0..100)
        .map(|i| {
            let content = format!("batch content {i}");
            ContentBlob {
                hash: hash_content(&content),
                content,
                size: 15,
                kind: "text".to_string(),
            }
        })
        .collect();

    let inserted = insert_blobs_batch(&conn, &blobs).unwrap();
    assert_eq!(inserted, 100);

    // Insert same batch again — all should be dedup hits
    let inserted2 = insert_blobs_batch(&conn, &blobs).unwrap();
    assert_eq!(inserted2, 0);
}

// ---------------------------------------------------------------------------
// FTS5 tests
// ---------------------------------------------------------------------------

#[test]
fn test_index_and_search_content() {
    let conn = open_test_db();

    let content1 = "implementing authentication with JWT tokens";
    let content2 = "building a REST API with axum framework";
    let content3 = "debugging a segmentation fault in C code";

    let h1 = hash_content(content1);
    let h2 = hash_content(content2);
    let h3 = hash_content(content3);

    insert_blob(&conn, &h1, content1, content1.len() as i64, "text").unwrap();
    insert_blob(&conn, &h2, content2, content2.len() as i64, "text").unwrap();
    insert_blob(&conn, &h3, content3, content3.len() as i64, "text").unwrap();

    index_content(&conn, &h1, "text", content1).unwrap();
    index_content(&conn, &h2, "text", content2).unwrap();
    index_content(&conn, &h3, "text", content3).unwrap();

    let results = search(&conn, "authentication", 10, 0).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].hash, h1);
    assert!(results[0].snippet.contains("<mark>"));
}

#[test]
fn test_search_no_results() {
    let conn = open_test_db();
    let results = search(&conn, "nonexistentterm", 10, 0).unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_fts_dedup_index() {
    let conn = open_test_db();
    let content = "duplicate FTS content test";
    let hash = hash_content(content);
    insert_blob(&conn, &hash, content, content.len() as i64, "text").unwrap();

    // Index same hash twice
    index_content(&conn, &hash, "text", content).unwrap();
    index_content(&conn, &hash, "text", content).unwrap();

    // Should only appear once in results
    let results = search(&conn, "duplicate", 10, 0).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_fts_porter_stemming() {
    let conn = open_test_db();
    let content = "the developer is running all the tests";
    let hash = hash_content(content);
    insert_blob(&conn, &hash, content, content.len() as i64, "text").unwrap();
    index_content(&conn, &hash, "text", content).unwrap();

    // "run" should match "running" via porter stemmer
    let results = search(&conn, "run", 10, 0).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_fts_invalid_syntax() {
    let conn = open_test_db();
    // FTS5 syntax error — unmatched quote
    let result = search(&conn, "\"unclosed quote", 10, 0);
    assert!(result.is_err());
}

#[test]
fn test_fts_boolean_operators() {
    let conn = open_test_db();

    let c1 = "rust programming language";
    let c2 = "python programming language";
    let c3 = "rust and python together";

    for (content, kind) in [(c1, "text"), (c2, "text"), (c3, "text")] {
        let h = hash_content(content);
        insert_blob(&conn, &h, content, content.len() as i64, kind).unwrap();
        index_content(&conn, &h, kind, content).unwrap();
    }

    // AND search
    let results = search(&conn, "rust AND python", 10, 0).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].hash, hash_content(c3));
}

#[test]
fn test_search_snippets_have_marks() {
    let conn = open_test_db();
    let content = "this is a test of the snippet highlighting feature in FTS5";
    let hash = hash_content(content);
    insert_blob(&conn, &hash, content, content.len() as i64, "text").unwrap();
    index_content(&conn, &hash, "text", content).unwrap();

    let results = search(&conn, "snippet", 10, 0).unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].snippet.contains("<mark>"));
    assert!(results[0].snippet.contains("</mark>"));
}

#[test]
fn test_search_with_limit_and_offset() {
    let conn = open_test_db();

    for i in 0..5 {
        let content = format!("searchable item number {i} with common term");
        let hash = hash_content(&content);
        insert_blob(&conn, &hash, &content, content.len() as i64, "text").unwrap();
        index_content(&conn, &hash, "text", &content).unwrap();
    }

    let page1 = search(&conn, "searchable", 2, 0).unwrap();
    assert_eq!(page1.len(), 2);

    let page2 = search(&conn, "searchable", 2, 2).unwrap();
    assert_eq!(page2.len(), 2);

    let page3 = search(&conn, "searchable", 2, 4).unwrap();
    assert_eq!(page3.len(), 1);
}
