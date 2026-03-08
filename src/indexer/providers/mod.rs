use anyhow::Result;
use rusqlite::Connection;
use crate::indexer::scanner::{FileEntry, FileKind};
use crate::indexer::provider::{SourceProvider, ProcessStats};
use crate::indexer::{sessions, router, gemini, codex};
use crate::indexer::redact::Redactor;

pub struct ClaudeProvider;

impl SourceProvider for ClaudeProvider {
    fn name(&self) -> &'static str { "claude" }

    fn can_handle(&self, kind: &FileKind) -> bool {
        matches!(kind, FileKind::SessionIndex | FileKind::SessionJsonl | FileKind::ClaudeDesktopSessionIndex)
    }

    fn process_metadata(&self, conn: &Connection, entry: &FileEntry) -> Result<usize> {
        match entry.kind {
            FileKind::SessionIndex => sessions::parse_session_index(conn, &entry.path),
            FileKind::ClaudeDesktopSessionIndex => sessions::parse_desktop_session_index(conn, &entry.path),
            _ => Ok(0),
        }
    }

    fn process_content(
        &self, 
        conn: &Connection, 
        entry: &FileEntry, 
        start_offset: u64,
        redactor: Option<&Redactor>
    ) -> Result<(ProcessStats, u64)> {
        match entry.kind {
            FileKind::SessionJsonl => {
                let (stats, final_offset) = router::process_jsonl(conn, &entry.path, start_offset, false, None, Some("claude"), redactor)?;
                Ok((ProcessStats {
                    messages_processed: stats.messages_processed,
                    messages_skipped: stats.messages_skipped,
                    parse_errors: stats.parse_errors,
                    blobs_inserted: stats.blobs_inserted,
                    tool_calls_inserted: stats.tool_calls_inserted,
                    sessions_parsed: 0,
                }, final_offset))
            }
            _ => Ok((ProcessStats::default(), entry.size_bytes)),
        }
    }
}

pub struct GeminiProvider;

impl SourceProvider for GeminiProvider {
    fn name(&self) -> &'static str { "gemini" }

    fn can_handle(&self, kind: &FileKind) -> bool {
        matches!(kind, FileKind::GeminiSessionJson)
    }

    fn process_content(
        &self, 
        conn: &Connection, 
        entry: &FileEntry, 
        _start_offset: u64,
        _redactor: Option<&Redactor>
    ) -> Result<(ProcessStats, u64)> {
        gemini::process_gemini_session(conn, &entry.path, "gemini")?;
        Ok((ProcessStats {
            sessions_parsed: 1,
            ..Default::default()
        }, entry.size_bytes))
    }
}

pub struct CodexProvider;

impl SourceProvider for CodexProvider {
    fn name(&self) -> &'static str { "codex" }

    fn can_handle(&self, kind: &FileKind) -> bool {
        matches!(kind, FileKind::CodexSessionJsonl)
    }

    fn process_content(
        &self, 
        conn: &Connection, 
        entry: &FileEntry, 
        _start_offset: u64,
        _redactor: Option<&Redactor>
    ) -> Result<(ProcessStats, u64)> {
        codex::process_codex_session(conn, &entry.path, "codex")?;
        Ok((ProcessStats {
            sessions_parsed: 1,
            ..Default::default()
        }, entry.size_bytes))
    }
}
