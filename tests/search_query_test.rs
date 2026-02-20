use blacklight::content::{hash_content, index_content, insert_blob};
use blacklight::db;
use blacklight::server::queries::search::search_content;
use tempfile::TempDir;

#[test]
fn test_search_with_special_characters() {
    let tmp = TempDir::new().unwrap();
    let conn = db::open(&tmp.path().join("test.db")).unwrap();

    let content = "The prefix is gemini:session-123 and we use git-cas.";
    let h = hash_content(content);
    insert_blob(&conn, &h, content, content.len() as i64, "text").unwrap();
    index_content(&conn, &h, "text", content).unwrap();

    // 1. Search for a term with a colon (the bug reported)
    // Before sanitization, this would fail with "no such column: gemini"
    let res = search_content(&conn, "gemini:session", None, None, 10, 0).unwrap();
    assert_eq!(res.items.len(), 1);
    assert!(res.items[0].snippet.contains("gemini:session"));

    // 2. Search for a term with a hyphen
    let res2 = search_content(&conn, "git-cas", None, None, 10, 0).unwrap();
    assert_eq!(res2.items.len(), 1);
    assert!(res2.items[0].snippet.contains("git-cas"));

    // 3. Search with kind filter and special characters
    let res3 = search_content(&conn, "gemini:session", Some("text"), None, 10, 0).unwrap();
    assert_eq!(res3.items.len(), 1);
    assert!(res3.items[0].snippet.contains("gemini:session"));

    // 4. Search with internal quotes (should be escaped)
    let res4 = search_content(&conn, "prefix is \"gemini\"", None, None, 10, 0).unwrap();
    assert_eq!(res4.items.len(), 1);
    assert!(res4.items[0].snippet.contains("gemini"));

    // 5. Project filtering should scope results to the requested project slug.
    conn.execute(
        "INSERT INTO sessions (id, project_path, project_slug, created_at, modified_at, source_file)
         VALUES ('sess-alpha', '/work/alpha', 'alpha', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 'alpha.jsonl')",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO sessions (id, project_path, project_slug, created_at, modified_at, source_file)
         VALUES ('sess-beta', '/work/beta', 'beta', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 'beta.jsonl')",
        [],
    ).unwrap();

    conn.execute(
        "INSERT INTO messages (id, session_id, type, timestamp) VALUES ('msg-alpha', 'sess-alpha', 'assistant', '2026-01-01T00:00:01Z')",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO messages (id, session_id, type, timestamp) VALUES ('msg-beta', 'sess-beta', 'assistant', '2026-01-01T00:00:02Z')",
        [],
    ).unwrap();

    let alpha_content = "deploy pipeline completed successfully";
    let alpha_hash = hash_content(alpha_content);
    insert_blob(
        &conn,
        &alpha_hash,
        alpha_content,
        alpha_content.len() as i64,
        "text",
    )
    .unwrap();
    index_content(&conn, &alpha_hash, "text", alpha_content).unwrap();
    conn.execute(
        "INSERT INTO blob_references (hash, message_id, context) VALUES (?1, 'msg-alpha', 'response_text')",
        [&alpha_hash],
    ).unwrap();

    let beta_content = "deploy pipeline failed during verification";
    let beta_hash = hash_content(beta_content);
    insert_blob(
        &conn,
        &beta_hash,
        beta_content,
        beta_content.len() as i64,
        "text",
    )
    .unwrap();
    index_content(&conn, &beta_hash, "text", beta_content).unwrap();
    conn.execute(
        "INSERT INTO blob_references (hash, message_id, context) VALUES (?1, 'msg-beta', 'response_text')",
        [&beta_hash],
    ).unwrap();

    let unfiltered = search_content(&conn, "deploy pipeline", None, None, 10, 0).unwrap();
    assert_eq!(unfiltered.items.len(), 2);

    let filtered = search_content(&conn, "deploy pipeline", None, Some("alpha"), 10, 0).unwrap();
    assert_eq!(filtered.items.len(), 1);
    assert_eq!(filtered.items[0].session_id.as_deref(), Some("sess-alpha"));
}
