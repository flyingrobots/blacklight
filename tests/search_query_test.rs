use blacklight::db;
use blacklight::content::{hash_content, insert_blob, index_content};
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
}
