use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;

use crate::models::SessionMessage;

use super::db_ops::{flush_batch, LineOps};
use super::file_paths::ToolUseTracker;
use super::handlers;
use super::jsonl::JsonlReader;
use super::sessions::ensure_session;

const BATCH_SIZE: usize = 500;

/// Statistics from processing a JSONL file.
#[derive(Debug, Default)]
pub struct RouterStats {
    pub messages_processed: usize,
    pub messages_skipped: usize,
    pub parse_errors: usize,
    pub blobs_inserted: usize,
    pub tool_calls_inserted: usize,
}

/// Process a single JSONL file, routing each message to the appropriate handler.
/// Returns (stats, final_byte_offset).
pub fn process_jsonl(
    conn: &Connection,
    path: &Path,
    start_offset: u64,
    verbose: bool,
) -> Result<(RouterStats, u64)> {
    let mut reader = JsonlReader::open(path, start_offset)?;
    let mut stats = RouterStats::default();
    let mut batch: Vec<LineOps> = Vec::with_capacity(BATCH_SIZE);
    let mut tracker = ToolUseTracker::new();
    let mut seen_sessions: std::collections::HashSet<String> = std::collections::HashSet::new();
    let source_file = path.to_string_lossy().to_string();

    // Derive session_id from filename as fallback (the JSONL filename is the session UUID)
    let filename_session_id = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string());

    while let Some((line, _offset)) = reader.next_line()? {
        // Fast skip for progress/queue-operation messages
        if handlers::is_skippable(&line) {
            stats.messages_skipped += 1;
            continue;
        }

        let msg: SessionMessage = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(e) => {
                if verbose {
                    tracing::warn!(
                        "parse error in {}:{} — {}",
                        path.display(),
                        reader.byte_offset(),
                        e
                    );
                }
                stats.parse_errors += 1;
                continue;
            }
        };

        let ops = match &msg {
            SessionMessage::Assistant(envelope) => {
                if seen_sessions.insert(envelope.session_id.clone()) {
                    ensure_session(
                        conn,
                        &envelope.session_id,
                        &source_file,
                        envelope.cwd.as_deref(),
                        envelope.git_branch.as_deref(),
                        &envelope.timestamp,
                    )
                    .with_context(|| {
                        format!("failed to ensure session for {}", path.display())
                    })?;
                }
                handlers::handle_assistant(envelope, &mut tracker)
            }
            SessionMessage::User(envelope) => {
                if seen_sessions.insert(envelope.session_id.clone()) {
                    ensure_session(
                        conn,
                        &envelope.session_id,
                        &source_file,
                        envelope.cwd.as_deref(),
                        envelope.git_branch.as_deref(),
                        &envelope.timestamp,
                    )
                    .with_context(|| {
                        format!("failed to ensure session for {}", path.display())
                    })?;
                }
                handlers::handle_user(envelope, &mut tracker)
            }
            SessionMessage::System(envelope) => {
                if seen_sessions.insert(envelope.session_id.clone()) {
                    ensure_session(
                        conn,
                        &envelope.session_id,
                        &source_file,
                        None,
                        None,
                        &envelope.timestamp,
                    )?;
                }
                handlers::handle_system(envelope)
            }
            SessionMessage::Summary(envelope) => {
                // Ensure session exists for summaries using filename-derived ID
                if let Some(fsid) = &filename_session_id {
                    if seen_sessions.insert(fsid.clone()) {
                        ensure_session(
                            conn,
                            fsid,
                            &source_file,
                            None,
                            None,
                            "", // no timestamp available from summary
                        )?;
                    }
                }
                let sid = filename_session_id
                    .as_deref()
                    .or(seen_sessions.iter().next().map(|s| s.as_str()))
                    .unwrap_or("unknown");
                handlers::handle_summary(envelope, sid)
            }
            SessionMessage::FileHistorySnapshot(_) => {
                // Skip file history snapshots for now
                continue;
            }
            SessionMessage::Progress(_) | SessionMessage::QueueOperation(_) => {
                // Should have been caught by is_skippable, but handle gracefully
                stats.messages_skipped += 1;
                continue;
            }
        };

        batch.push(ops);
        stats.messages_processed += 1;

        if batch.len() >= BATCH_SIZE {
            let flush_stats = flush_batch(conn, &batch)
                .with_context(|| format!("failed to flush batch for {}", path.display()))?;
            stats.blobs_inserted += flush_stats.blobs_inserted;
            stats.tool_calls_inserted += flush_stats.tool_calls_inserted;
            batch.clear();
        }
    }

    // Flush remaining
    if !batch.is_empty() {
        let flush_stats = flush_batch(conn, &batch)
            .with_context(|| format!("failed to flush final batch for {}", path.display()))?;
        stats.blobs_inserted += flush_stats.blobs_inserted;
        stats.tool_calls_inserted += flush_stats.tool_calls_inserted;
    }

    let final_offset = reader.byte_offset();

    if verbose {
        tracing::info!(
            "{}: {} messages, {} skipped, {} errors",
            path.display(),
            stats.messages_processed,
            stats.messages_skipped,
            stats.parse_errors,
        );
    }

    Ok((stats, final_offset))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_process_jsonl_basic() {
        let tmp = TempDir::new().unwrap();
        let conn = db::open(&tmp.path().join("test.db")).unwrap();

        let jsonl_path = tmp.path().join("test.jsonl");
        let mut f = std::fs::File::create(&jsonl_path).unwrap();

        // Write a user message then an assistant message
        writeln!(f, r#"{{"type":"user","uuid":"u1","parentUuid":null,"sessionId":"sess1","timestamp":"2024-01-01T00:00:00Z","message":{{"role":"user","content":"hello world"}}}}"#).unwrap();
        writeln!(f, r#"{{"type":"assistant","uuid":"a1","parentUuid":"u1","sessionId":"sess1","timestamp":"2024-01-01T00:00:01Z","message":{{"role":"assistant","model":"claude-3","content":[{{"type":"text","text":"hi there"}}],"stop_reason":"end_turn"}}}}"#).unwrap();
        // Write a progress message that should be skipped
        writeln!(f, r#"{{"type":"progress","uuid":"p1","sessionId":"sess1","timestamp":"2024-01-01T00:00:02Z"}}"#).unwrap();

        let (stats, offset) = process_jsonl(&conn, &jsonl_path, 0, false).unwrap();
        assert_eq!(stats.messages_processed, 2);
        assert_eq!(stats.messages_skipped, 1);
        assert!(offset > 0);

        // Verify messages in DB
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_process_jsonl_incremental() {
        let tmp = TempDir::new().unwrap();
        let conn = db::open(&tmp.path().join("test.db")).unwrap();

        let jsonl_path = tmp.path().join("test.jsonl");
        let mut f = std::fs::File::create(&jsonl_path).unwrap();
        let line1 = r#"{"type":"user","uuid":"u1","parentUuid":null,"sessionId":"sess1","timestamp":"2024-01-01T00:00:00Z","message":{"role":"user","content":"hello"}}"#;
        writeln!(f, "{line1}").unwrap();
        f.flush().unwrap();

        let (stats1, offset1) = process_jsonl(&conn, &jsonl_path, 0, false).unwrap();
        assert_eq!(stats1.messages_processed, 1);

        // Append another line
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&jsonl_path)
            .unwrap();
        writeln!(f, r#"{{"type":"user","uuid":"u2","parentUuid":"u1","sessionId":"sess1","timestamp":"2024-01-01T00:00:01Z","message":{{"role":"user","content":"world"}}}}"#).unwrap();
        f.flush().unwrap();

        // Resume from offset
        let (stats2, _) = process_jsonl(&conn, &jsonl_path, offset1, false).unwrap();
        assert_eq!(stats2.messages_processed, 1);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }
}
