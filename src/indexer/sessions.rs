use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;

use crate::models::SessionIndex;

/// Parse a sessions-index.json file and upsert all entries into the sessions table.
/// Returns the count of upserted sessions.
pub fn parse_session_index(conn: &Connection, path: &Path) -> Result<usize> {
    let data = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let index: SessionIndex = serde_json::from_str(&data)
        .with_context(|| format!("failed to parse {}", path.display()))?;

    let source_file = path.to_string_lossy().to_string();
    let mut count = 0;

    let tx = conn.unchecked_transaction()?;
    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR REPLACE INTO sessions
             (id, project_path, project_slug, first_prompt, summary, message_count,
              created_at, modified_at, git_branch, claude_version, is_sidechain, source_file)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        )?;

        for entry in &index.entries {
            let project_path = entry
                .project_path
                .as_deref()
                .or(index.original_path.as_deref())
                .unwrap_or("unknown");

            let project_slug = project_path
                .rsplit('/')
                .next()
                .unwrap_or("unknown")
                .to_string();

            let created = entry
                .created
                .as_deref()
                .unwrap_or("1970-01-01T00:00:00Z");
            let modified = entry
                .modified
                .as_deref()
                .unwrap_or(created);

            stmt.execute(params![
                entry.session_id,
                project_path,
                project_slug,
                entry.first_prompt,
                entry.summary,
                entry.message_count,
                created,
                modified,
                entry.git_branch,
                Option::<String>::None, // claude_version not in index
                entry.is_sidechain.unwrap_or(false) as i32,
                source_file,
            ])?;
            count += 1;
        }
    }
    tx.commit()?;

    tracing::info!(
        "parsed {} sessions from {}",
        count,
        path.display()
    );

    Ok(count)
}

/// Ensure a session row exists. Creates a minimal row if it doesn't.
/// Used for subagent files that may not have an entry in sessions-index.json.
pub fn ensure_session(
    conn: &Connection,
    session_id: &str,
    source_file: &str,
    cwd: Option<&str>,
    git_branch: Option<&str>,
    timestamp: &str,
) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sessions WHERE id = ?1)",
        params![session_id],
        |row| row.get(0),
    )?;

    if !exists {
        let project_path = cwd.unwrap_or("unknown");
        let project_slug = project_path
            .rsplit('/')
            .next()
            .unwrap_or("unknown");

        conn.execute(
            "INSERT INTO sessions
             (id, project_path, project_slug, created_at, modified_at, source_file)
             VALUES (?1, ?2, ?3, ?4, ?4, ?5)",
            params![session_id, project_path, project_slug, timestamp, source_file],
        )?;

        if git_branch.is_some() {
            conn.execute(
                "UPDATE sessions SET git_branch = ?2 WHERE id = ?1",
                params![session_id, git_branch],
            )?;
        }

        tracing::debug!("created minimal session row for {session_id}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_parse_session_index() {
        let tmp = TempDir::new().unwrap();
        let conn = db::open(&tmp.path().join("test.db")).unwrap();

        let index_json = r#"{
            "version": 1,
            "entries": [
                {
                    "sessionId": "sess-001",
                    "fullPath": "/home/.claude/projects/abc/sess-001.jsonl",
                    "firstPrompt": "hello",
                    "summary": "test session",
                    "messageCount": 10,
                    "created": "2024-01-01T00:00:00Z",
                    "modified": "2024-01-02T00:00:00Z",
                    "projectPath": "/Users/james/git/myproject",
                    "gitBranch": "main"
                }
            ],
            "originalPath": "/Users/james/git/fallback"
        }"#;

        let index_path = tmp.path().join("sessions-index.json");
        let mut f = std::fs::File::create(&index_path).unwrap();
        f.write_all(index_json.as_bytes()).unwrap();

        let count = parse_session_index(&conn, &index_path).unwrap();
        assert_eq!(count, 1);

        let slug: String = conn
            .query_row(
                "SELECT project_slug FROM sessions WHERE id = 'sess-001'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(slug, "myproject");
    }

    #[test]
    fn test_ensure_session_creates_minimal() {
        let tmp = TempDir::new().unwrap();
        let conn = db::open(&tmp.path().join("test.db")).unwrap();

        ensure_session(
            &conn,
            "sess-new",
            "test.jsonl",
            Some("/Users/james/git/project"),
            Some("feature-branch"),
            "2024-01-01T00:00:00Z",
        )
        .unwrap();

        let slug: String = conn
            .query_row(
                "SELECT project_slug FROM sessions WHERE id = 'sess-new'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(slug, "project");
    }

    #[test]
    fn test_ensure_session_noop_if_exists() {
        let tmp = TempDir::new().unwrap();
        let conn = db::open(&tmp.path().join("test.db")).unwrap();

        conn.execute(
            "INSERT INTO sessions (id, project_path, project_slug, created_at, modified_at, source_file)
             VALUES ('sess-exists', '/proj', 'proj', '2024-01-01', '2024-01-01', 'test.jsonl')",
            [],
        ).unwrap();

        // Should not error
        ensure_session(
            &conn,
            "sess-exists",
            "other.jsonl",
            None,
            None,
            "2024-02-01T00:00:00Z",
        )
        .unwrap();

        // Original slug should be preserved
        let slug: String = conn
            .query_row(
                "SELECT project_slug FROM sessions WHERE id = 'sess-exists'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(slug, "proj");
    }
}
