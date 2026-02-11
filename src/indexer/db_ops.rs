use anyhow::{Context, Result};
use rusqlite::{params, Connection};

/// Row data for a message insert.
#[derive(Debug, Clone)]
pub struct MessageRow {
    pub id: String,
    pub session_id: String,
    pub parent_id: Option<String>,
    pub msg_type: String,
    pub timestamp: String,
    pub model: Option<String>,
    pub stop_reason: Option<String>,
    pub cwd: Option<String>,
    pub git_branch: Option<String>,
    pub duration_ms: Option<u64>,
}

/// Row data for a content_block insert.
#[derive(Debug, Clone)]
pub struct ContentBlockRow {
    pub message_id: String,
    pub block_index: i64,
    pub block_type: String,
    pub content_hash: Option<String>,
    pub tool_name: Option<String>,
    pub tool_use_id: Option<String>,
    pub tool_input_hash: Option<String>,
}

/// Row data for a tool_call insert.
#[derive(Debug, Clone)]
pub struct ToolCallRow {
    pub id: String,
    pub message_id: String,
    pub session_id: String,
    pub tool_name: String,
    pub input_hash: Option<String>,
    pub timestamp: String,
}

/// Row data for a file_reference insert.
#[derive(Debug, Clone)]
pub struct FileRefRow {
    pub file_path: String,
    pub content_hash: String,
    pub session_id: String,
    pub message_id: String,
    pub operation: String,
}

/// Accumulated operations from processing a single JSONL line.
#[derive(Debug, Default)]
pub struct LineOps {
    pub message: Option<MessageRow>,
    pub content_blocks: Vec<ContentBlockRow>,
    pub tool_calls: Vec<ToolCallRow>,
    /// (hash, content, size, kind)
    pub blobs: Vec<(String, String, i64, String)>,
    /// (hash, message_id, context)
    pub blob_refs: Vec<(String, String, String)>,
    /// (hash, kind, content)
    pub fts_entries: Vec<(String, String, String)>,
    pub file_refs: Vec<FileRefRow>,
    /// (tool_use_id, output_hash)
    pub tool_output_links: Vec<(String, String)>,
}

/// Statistics from a flush_batch call.
#[derive(Debug, Default)]
pub struct FlushStats {
    pub messages_inserted: usize,
    pub blobs_inserted: usize,
    pub content_blocks_inserted: usize,
    pub tool_calls_inserted: usize,
    pub file_refs_inserted: usize,
}

/// Flush a batch of LineOps in a single transaction.
pub fn flush_batch(conn: &Connection, batch: &[LineOps]) -> Result<FlushStats> {
    let mut stats = FlushStats::default();
    let tx = conn.unchecked_transaction().context("failed to begin transaction")?;

    // 1. INSERT OR IGNORE blobs → content_store
    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO content_store (hash, content, size, kind) VALUES (?1, ?2, ?3, ?4)",
        )?;
        for ops in batch {
            for (hash, content, size, kind) in &ops.blobs {
                let changes = stmt.execute(params![hash, content, size, kind])?;
                if changes > 0 {
                    stats.blobs_inserted += 1;
                }
            }
        }
    }

    // 2. INSERT messages
    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO messages (id, session_id, parent_id, type, timestamp, model, stop_reason, cwd, git_branch, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )?;
        for ops in batch {
            if let Some(msg) = &ops.message {
                stmt.execute(params![
                    msg.id,
                    msg.session_id,
                    msg.parent_id,
                    msg.msg_type,
                    msg.timestamp,
                    msg.model,
                    msg.stop_reason,
                    msg.cwd,
                    msg.git_branch,
                    msg.duration_ms,
                ]).with_context(|| format!("failed to insert message {} (session={})", msg.id, msg.session_id))?;
                stats.messages_inserted += 1;
            }
        }
    }

    // 3. INSERT content_blocks
    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO content_blocks (message_id, block_index, block_type, content_hash, tool_name, tool_use_id, tool_input_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        for ops in batch {
            for block in &ops.content_blocks {
                let changes = stmt.execute(params![
                    block.message_id,
                    block.block_index,
                    block.block_type,
                    block.content_hash,
                    block.tool_name,
                    block.tool_use_id,
                    block.tool_input_hash,
                ])?;
                if changes > 0 {
                    stats.content_blocks_inserted += 1;
                }
            }
        }
    }

    // 4. INSERT OR IGNORE tool_calls
    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO tool_calls (id, message_id, session_id, tool_name, input_hash, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        for ops in batch {
            for tc in &ops.tool_calls {
                let changes = stmt.execute(params![
                    tc.id,
                    tc.message_id,
                    tc.session_id,
                    tc.tool_name,
                    tc.input_hash,
                    tc.timestamp,
                ])?;
                if changes > 0 {
                    stats.tool_calls_inserted += 1;
                }
            }
        }
    }

    // 5. UPDATE tool_calls SET output_hash (for tool_output_links)
    {
        let mut stmt = tx.prepare_cached(
            "UPDATE tool_calls SET output_hash = ?2 WHERE id = ?1",
        )?;
        for ops in batch {
            for (tool_use_id, output_hash) in &ops.tool_output_links {
                stmt.execute(params![tool_use_id, output_hash])?;
            }
        }
    }

    // 6. INSERT OR IGNORE blob_references
    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO blob_references (hash, message_id, context) VALUES (?1, ?2, ?3)",
        )?;
        for ops in batch {
            for (hash, msg_id, context) in &ops.blob_refs {
                stmt.execute(params![hash, msg_id, context])?;
            }
        }
    }

    // 7. INSERT OR IGNORE → fts_content (with dedup check)
    {
        let mut check_stmt = tx.prepare_cached(
            "SELECT EXISTS(SELECT 1 FROM fts_content WHERE hash = ?1)",
        )?;
        let mut insert_stmt = tx.prepare_cached(
            "INSERT INTO fts_content (hash, kind, content) VALUES (?1, ?2, ?3)",
        )?;
        for ops in batch {
            for (hash, kind, content) in &ops.fts_entries {
                let exists: bool = check_stmt.query_row(params![hash], |row| row.get(0))?;
                if !exists {
                    insert_stmt.execute(params![hash, kind, content])?;
                }
            }
        }
    }

    // 8. INSERT file_references
    {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO file_references (file_path, content_hash, session_id, message_id, operation)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        for ops in batch {
            for fr in &ops.file_refs {
                let changes = stmt.execute(params![
                    fr.file_path,
                    fr.content_hash,
                    fr.session_id,
                    fr.message_id,
                    fr.operation,
                ])?;
                if changes > 0 {
                    stats.file_refs_inserted += 1;
                }
            }
        }
    }

    tx.commit().context("failed to commit batch transaction")?;
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use tempfile::TempDir;

    fn setup() -> (TempDir, Connection) {
        let tmp = TempDir::new().unwrap();
        let conn = db::open(&tmp.path().join("test.db")).unwrap();
        // Insert a session for FK
        conn.execute(
            "INSERT INTO sessions (id, project_path, project_slug, created_at, modified_at, source_file)
             VALUES ('sess1', '/proj', 'proj', '2024-01-01', '2024-01-01', 'test.jsonl')",
            [],
        ).unwrap();
        (tmp, conn)
    }

    #[test]
    fn test_flush_empty_batch() {
        let (_tmp, conn) = setup();
        let stats = flush_batch(&conn, &[]).unwrap();
        assert_eq!(stats.messages_inserted, 0);
    }

    #[test]
    fn test_flush_message_and_blob() {
        let (_tmp, conn) = setup();

        let mut ops = LineOps::default();
        ops.message = Some(MessageRow {
            id: "msg1".into(),
            session_id: "sess1".into(),
            parent_id: None,
            msg_type: "assistant".into(),
            timestamp: "2024-01-01T00:00:00Z".into(),
            model: Some("claude-3".into()),
            stop_reason: Some("end_turn".into()),
            cwd: None,
            git_branch: None,
            duration_ms: None,
        });
        ops.blobs.push((
            "abc123".into(),
            "hello world".into(),
            11,
            "text".into(),
        ));
        ops.blob_refs.push((
            "abc123".into(),
            "msg1".into(),
            "response_text".into(),
        ));

        let stats = flush_batch(&conn, &[ops]).unwrap();
        assert_eq!(stats.messages_inserted, 1);
        assert_eq!(stats.blobs_inserted, 1);

        // Verify message exists
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM messages WHERE id = 'msg1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_dedup_blobs() {
        let (_tmp, conn) = setup();

        let mut ops1 = LineOps::default();
        ops1.blobs.push(("hash1".into(), "content".into(), 7, "text".into()));
        let mut ops2 = LineOps::default();
        ops2.blobs.push(("hash1".into(), "content".into(), 7, "text".into()));

        let stats = flush_batch(&conn, &[ops1, ops2]).unwrap();
        assert_eq!(stats.blobs_inserted, 1); // Second is dedup'd
    }
}
